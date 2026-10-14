use super::*;
pub(super) fn color(i: usize) -> Color32 {
    [
        GREEN,
        BLUE,
        ORANGE,
        Color32::from_rgb(201, 158, 255),
        RED,
        Color32::from_rgb(238, 220, 115),
    ][i % 6]
}
pub(super) fn engineering(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    if v == 0. {
        return "0".into();
    }
    let (factor, suffix) = match v.abs() {
        x if x >= 1e9 => (1e9, "G"),
        x if x >= 1e6 => (1e6, "M"),
        x if x >= 1e3 => (1e3, "k"),
        x if x >= 1. => (1., ""),
        x if x >= 1e-3 => (1e-3, "m"),
        x if x >= 1e-6 => (1e-6, "µ"),
        x if x >= 1e-9 => (1e-9, "n"),
        x if x >= 1e-12 => (1e-12, "p"),
        _ => (1., ""),
    };
    format!("{:.4}{suffix}", v / factor)
}
fn value(t: &Trace, i: usize, mode: usize, complex: bool) -> f64 {
    match mode {
        1 => t.db(i),
        2 => t.phase(i),
        3 => t.real[i],
        4 => t.imag[i],
        _ => {
            if complex {
                t.magnitude(i)
            } else {
                t.real[i]
            }
        }
    }
}
impl App {
    pub(super) fn waveforms(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new().inner_margin(18).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Waveform viewer");
                if self.result.as_ref().is_some_and(|r| r.complex) {
                    egui::ComboBox::from_id_salt("plot-mode")
                        .selected_text(
                            [
                                "Magnitude",
                                "Magnitude · dB",
                                "Phase · degrees",
                                "Real",
                                "Imaginary",
                            ][self.plot_mode],
                        )
                        .show_ui(ui, |ui| {
                            for (i, s) in [
                                "Magnitude",
                                "Magnitude · dB",
                                "Phase · degrees",
                                "Real",
                                "Imaginary",
                            ]
                            .iter()
                            .enumerate()
                            {
                                ui.selectable_value(&mut self.plot_mode, i, *s);
                            }
                        });
                }
            });
            let Some(sim) = &self.result else {
                ui.add_space(40.);
                ui.colored_label(MUTED, "Choose an analysis and Run ngspice.");
                ui.label("DC steady state · AC frequency response · transient time domain");
                return;
            };
            ui.label(
                RichText::new(format!(
                    "{} · {} samples · hover to measure",
                    sim.title,
                    sim.axis.len()
                ))
                .size(12.)
                .color(MUTED),
            );
            if sim.axis.len() == 1 {
                ui.add_space(20.);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("op")
                        .striped(true)
                        .spacing(Vec2::new(28., 13.))
                        .show(ui, |ui| {
                            for s in ["Signal", "Value", "Unit"] {
                                ui.colored_label(GREEN, s);
                            }
                            ui.end_row();
                            for t in &sim.traces {
                                ui.monospace(&t.name);
                                ui.label(engineering(t.real[0]));
                                ui.label(if t.unit == "current" {
                                    "A"
                                } else if t.unit == "voltage" {
                                    "V"
                                } else {
                                    t.unit.as_str()
                                });
                                ui.end_row();
                            }
                        });
                });
                return;
            }
            let selected = sim
                .traces
                .iter()
                .enumerate()
                .filter(|(_, t)| self.enabled.contains(&t.name))
                .collect::<Vec<_>>();
            let mode = if sim.complex { self.plot_mode } else { 3 };
            let log_x = sim.axis_name == "frequency";
            let transform = |x: f64| if log_x { x.max(1e-300).log10() } else { x };
            let first = transform(sim.axis[0]);
            let last = transform(*sim.axis.last().unwrap());
            let xmin = first.min(last);
            let xmax = first.max(last);
            let span = (xmax - xmin).max(1e-15);
            let mut ymin = f64::INFINITY;
            let mut ymax = f64::NEG_INFINITY;
            for (_, t) in &selected {
                for i in 0..sim.axis.len() {
                    let v = value(t, i, mode, sim.complex);
                    if v.is_finite() {
                        ymin = ymin.min(v);
                        ymax = ymax.max(v);
                    }
                }
            }
            if !ymin.is_finite() || !ymax.is_finite() {
                ymin = 0.;
                ymax = 1.;
            }
            let pad = if (ymax - ymin).abs() <= ymax.abs().max(ymin.abs()).max(1e-300) * 1e-9 {
                ymax.abs().max(1e-12) * 0.1
            } else {
                (ymax - ymin) * 0.08
            };
            ymin -= pad;
            ymax += pad;
            let (outer, response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), ui.available_height().max(150.)),
                Sense::hover(),
            );
            let chart = Rect::from_min_max(
                outer.min + Vec2::new(66., 30.),
                outer.max - Vec2::new(18., 52.),
            );
            let p = ui.painter_at(outer);
            p.rect_filled(outer, 8., Color32::from_rgb(12, 19, 30));
            p.rect_filled(chart, 2., Color32::from_rgb(15, 25, 39));
            let screen = |x: f64, y: f64| {
                Pos2::new(
                    chart.left() + ((transform(x) - xmin) / span) as f32 * chart.width(),
                    chart.bottom() - ((y - ymin) / (ymax - ymin)) as f32 * chart.height(),
                )
            };
            for i in 0..=6 {
                let y = ymin + (ymax - ymin) * i as f64 / 6.;
                let at = screen(sim.axis[0], y);
                p.line_segment(
                    [
                        Pos2::new(chart.left(), at.y),
                        Pos2::new(chart.right(), at.y),
                    ],
                    Stroke::new(1., Color32::from_rgb(34, 48, 65)),
                );
                p.text(
                    Pos2::new(chart.left() - 9., at.y),
                    Align2::RIGHT_CENTER,
                    if matches!(mode, 1 | 2) {
                        format!("{y:.1}")
                    } else {
                        engineering(y)
                    },
                    FontId::monospace(10.),
                    MUTED,
                );
            }
            let ticks = if log_x {
                (xmin.floor() as i32..=xmax.ceil() as i32)
                    .map(|i| 10f64.powi(i))
                    .filter(|x| transform(*x) >= xmin - 1e-12 && transform(*x) <= xmax + 1e-12)
                    .collect::<Vec<_>>()
            } else {
                (0..=6).map(|i| xmin + span * i as f64 / 6.).collect()
            };
            for x in ticks {
                let at = screen(x, ymin);
                p.line_segment(
                    [
                        Pos2::new(at.x, chart.top()),
                        Pos2::new(at.x, chart.bottom()),
                    ],
                    Stroke::new(1., Color32::from_rgb(34, 48, 65)),
                );
                p.text(
                    Pos2::new(at.x, chart.bottom() + 16.),
                    Align2::CENTER_TOP,
                    engineering(x),
                    FontId::monospace(10.),
                    MUTED,
                );
            }
            let unit = if mode == 1 {
                "Magnitude · dB"
            } else if mode == 2 {
                "Phase · degrees"
            } else if selected.iter().all(|(_, t)| t.unit == "current") {
                "Current · A"
            } else if selected.iter().all(|(_, t)| t.unit == "voltage") {
                "Voltage · V"
            } else {
                "Signal values · see units in measurements"
            };
            p.text(
                chart.min - Vec2::new(0., 10.),
                Align2::LEFT_BOTTOM,
                unit,
                FontId::proportional(11.),
                MUTED,
            );
            p.text(
                Pos2::new(chart.center().x, chart.bottom() + 37.),
                Align2::CENTER_CENTER,
                format!(
                    "{}{}",
                    sim.axis_name,
                    if log_x {
                        " · Hz (log scale)"
                    } else if sim.axis_name == "time" {
                        " · s"
                    } else {
                        ""
                    }
                ),
                FontId::proportional(11.),
                MUTED,
            );
            let clip = p.with_clip_rect(chart);
            for (j, t) in &selected {
                let bin = (sim.axis.len() / chart.width().max(100.) as usize).max(1);
                let mut points = vec![];
                for start in (0..sim.axis.len()).step_by(bin) {
                    let end = (start + bin).min(sim.axis.len());
                    let mut min = start;
                    let mut max = start;
                    for i in start + 1..end {
                        let v = value(t, i, mode, sim.complex);
                        if v < value(t, min, mode, sim.complex) {
                            min = i;
                        }
                        if v > value(t, max, mode, sim.complex) {
                            max = i;
                        }
                    }
                    let mut indices = vec![start, min, max, end - 1];
                    indices.sort_unstable();
                    indices.dedup();
                    for i in indices {
                        points.push(screen(sim.axis[i], value(t, i, mode, sim.complex)));
                    }
                }
                clip.add(egui::Shape::line(points, Stroke::new(1.8, color(*j))));
            }
            if let Some(pointer) = response.hover_pos().filter(|p| chart.contains(*p)) {
                let x = xmin + (pointer.x - chart.left()) as f64 / chart.width() as f64 * span;
                let i = sim
                    .axis
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        (transform(**a) - x)
                            .abs()
                            .total_cmp(&(transform(**b) - x).abs())
                    })
                    .unwrap()
                    .0;
                self.cursor_index = Some(i);
                let cx = screen(sim.axis[i], ymin).x;
                clip.line_segment(
                    [Pos2::new(cx, chart.top()), Pos2::new(cx, chart.bottom())],
                    Stroke::new(1., MUTED),
                );
                for (j, t) in &selected {
                    clip.circle_filled(
                        screen(sim.axis[i], value(t, i, mode, sim.complex)),
                        3.5,
                        color(*j),
                    );
                }
            }
            if selected.is_empty() {
                p.text(
                    chart.center(),
                    Align2::CENTER_CENTER,
                    "Select signals in the left panel",
                    FontId::proportional(16.),
                    MUTED,
                );
            }
        });
    }
}
