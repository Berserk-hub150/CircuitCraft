use super::*;
pub(super) enum JobOutput {
    Simulation(Simulation),
    Sweep(SweepResult),
}
pub(super) fn component_footprint(c: &Component) -> FootprintDefinition {
    let pads = if c.custom_pads.is_empty() {
        Footprint {
            component_id: c.id,
            reference: c.reference.clone(),
            style: c.footprint.clone(),
            at: Mm::new(0., 0.),
            rotation: 0,
            nets: c.pins().iter().map(|_| "0".into()).collect(),
            custom_pads: vec![],
        }
        .pads()
        .iter()
        .map(|p| PadSpec {
            at: p.at,
            size: p.size,
            rectangular: p.rectangular,
            drill: p.drill,
        })
        .collect()
    } else {
        c.custom_pads.clone()
    };
    FootprintDefinition {
        name: c.footprint.clone(),
        pads,
    }
}
impl App {
    pub(super) fn advanced_tools(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Component parameter sweep")
            .default_open(self.sweep_enabled)
            .show(ui, |ui| {
                ui.checkbox(&mut self.sweep_enabled, "Sweep when running ngspice");
                egui::ComboBox::from_id_salt("sweep-part")
                    .selected_text(&self.sweep_reference)
                    .show_ui(ui, |ui| {
                        for c in self.sheet.components.iter().filter(|c| {
                            matches!(c.kind, Kind::Resistor | Kind::Capacitor | Kind::Inductor)
                        }) {
                            ui.selectable_value(
                                &mut self.sweep_reference,
                                c.reference.clone(),
                                &c.reference,
                            );
                        }
                    });
                ui.label("Values (1–24, comma separated)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.sweep_input).desired_width(f32::INFINITY),
                );
                ui.label(
                    RichText::new(
                        "R/C/L only. The original circuit is retained. Each run has a 45 s limit.",
                    )
                    .size(10.)
                    .color(MUTED),
                );
                if self.job.is_some()
                    && self.sweep_enabled
                    && ui.button("Cancel after current run").clicked()
                {
                    self.cancel.store(true, Ordering::Relaxed);
                }
            });
        if let Some(sweep) = &self.sweep {
            let signals = sweep.runs[0]
                .1
                .traces
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>();
            let old = self.sweep_signal.clone();
            egui::ComboBox::from_id_salt("sweep-signal")
                .selected_text(&self.sweep_signal)
                .show_ui(ui, |ui| {
                    for s in signals {
                        ui.selectable_value(&mut self.sweep_signal, s.clone(), s);
                    }
                });
            if old != self.sweep_signal {
                match sweep.overlay(&self.sweep_signal) {
                    Ok(sim) => {
                        self.enabled = sim.traces.iter().map(|t| t.name.clone()).collect();
                        self.result = Some(sim);
                        self.cursor_index = None;
                    }
                    Err(e) => self.say(e, true),
                }
            }
            if ui.button("Export complete sweep CSV…").clicked() {
                self.request_file(FileAction::SweepCSV);
            }
        }
        if self.tab == Tab::Simulation {
            egui::CollapsingHeader::new("Signal math / gain").show(ui,|ui|{
                let names=self.result.as_ref().map(|s|s.traces.iter().map(|t|t.name.clone()).collect::<Vec<_>>()).unwrap_or_default();
                for (label,id,target) in [("A","math-a",&mut self.math_a),("B","math-b",&mut self.math_b)] {
                    ui.label(label);egui::ComboBox::from_id_salt(id).selected_text(&*target).show_ui(ui,|ui|{for s in &names{ui.selectable_value(target,s.clone(),s);}});
                }
                egui::ComboBox::from_id_salt("math-op").selected_text(match self.math_op{TraceMath::Difference=>"A − B",TraceMath::Ratio=>"A / B (gain)",TraceMath::Product=>"A × B"}).show_ui(ui,|ui|{
                    for (op,label) in [(TraceMath::Difference,"A − B"),(TraceMath::Ratio,"A / B (gain)"),(TraceMath::Product,"A × B")] {ui.selectable_value(&mut self.math_op,op,label);}
                });
                if ui.button("Add calculated signal").clicked() {
                    let result=self.result.as_ref().ok_or("Run a simulation first.".to_string()).and_then(|s|derived_trace(s,&self.math_a,&self.math_b,self.math_op));
                    match result {
                        Ok(t)=>{let name=t.name.clone();let sim=self.result.as_mut().unwrap();sim.traces.retain(|old|old.name!=name);sim.traces.push(t);if !self.enabled.contains(&name){self.enabled.push(name);}self.say("Calculated signal added to plots and CSV.",false);},
                        Err(e)=>self.say(e,true),
                    }
                }
                ui.label(RichText::new("AC calculations use complex phasors. Ratio to the input gives gain. V × I is an instantaneous product in transient analysis.").size(10.).color(MUTED));
            });
            if ui.button("Export measurements CSV…").clicked() {
                self.request_file(FileAction::Measurements);
            }
        }
    }
    pub(super) fn footprint_editor(&mut self, ui: &mut egui::Ui, id: u64) -> bool {
        let mut changed = false;
        egui::CollapsingHeader::new("Custom footprint / pad editor").show(ui,|ui|{
            let Some(c)=self.sheet.components.iter_mut().find(|c|c.id==id) else {return;};
            if c.custom_pads.is_empty() {
                if ui.button("Customize this footprint").clicked() {
                    c.custom_pads=component_footprint(c).pads;
                    // A default two-pad footprint must not silently omit MOS/IC pins.
                    while c.custom_pads.len()<c.pins().len() {
                        let i=c.custom_pads.len();c.custom_pads.push(PadSpec{at:Mm::new(i as f64*2.54,2.54),size:Mm::new(1.8,1.8),rectangular:false,drill:0.8});
                    }
                    c.footprint=format!("Custom-{}",c.reference);changed=true;
                }
            } else {
                changed|=row(ui,"Name",&mut c.footprint);
                egui::ScrollArea::vertical().id_salt("pads-editor").max_height(225.).show(ui,|ui|{
                    for (i,p) in c.custom_pads.iter_mut().enumerate() {
                        ui.label(RichText::new(format!("Pad {} · SPICE terminal {}",i+1,i+1)).color(GREEN));
                        for (label,a,b) in [("X / Y mm",&mut p.at.x,&mut p.at.y),("Size X / Y",&mut p.size.x,&mut p.size.y)] {
                            ui.horizontal(|ui|{ui.label(label);changed|=ui.add(egui::DragValue::new(a).speed(0.05)).changed();changed|=ui.add(egui::DragValue::new(b).speed(0.05)).changed();});
                        }
                        changed|=ui.checkbox(&mut p.rectangular,"Rectangular pad").changed();
                        if !p.rectangular&&(p.size.x-p.size.y).abs()>1e-6 {p.size.y=p.size.x;changed=true;}
                        ui.horizontal(|ui|{ui.label("Drill (0 = SMD)");changed|=ui.add(egui::DragValue::new(&mut p.drill).speed(0.05).range(0. ..=20.)).changed();});
                        ui.separator();
                    }
                });
                if let Err(e)=(FootprintDefinition{name:c.footprint.clone(),pads:c.custom_pads.clone()}).validate(){ui.colored_label(RED,e);}
                ui.label(RichText::new("File menu imports/exports reusable footprint JSON. Synchronize the PCB after editing. Verify pin order against the datasheet.").size(10.).color(MUTED));
                if ui.button("Use default footprint again").clicked(){c.custom_pads.clear();c.footprint=c.kind.default_footprint().into();changed=true;}
            }
        });
        changed
    }
}
