#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod advanced;
mod drawing;
mod files;
mod plot;
use advanced::{component_footprint, JobOutput};
use circuitcraft_core::*;
use eframe::egui::{self, Align2, Color32, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};
const BG: Color32 = Color32::from_rgb(13, 20, 31);
const PANEL: Color32 = Color32::from_rgb(19, 29, 43);
const GREEN: Color32 = Color32::from_rgb(64, 220, 182);
const BLUE: Color32 = Color32::from_rgb(104, 179, 255);
const MUTED: Color32 = Color32::from_rgb(132, 154, 179);
const ORANGE: Color32 = Color32::from_rgb(255, 185, 105);
const RED: Color32 = Color32::from_rgb(255, 112, 130);
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Schematic,
    Pcb,
    Simulation,
    Spice,
    Bom,
}
#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Place(Kind),
    Wire,
    Ground,
    Junction,
    Label,
    Probe,
    Route,
    Via,
}
#[derive(Clone, Copy, PartialEq)]
enum Selection {
    Component(u64),
    Wire(u64),
    Label(u64),
    Ground(Grid),
    Junction(Grid),
    Footprint(u64),
    Track(u64),
    Via(u64),
}
#[derive(Clone, Copy)]
enum FileAction {
    Open,
    Save,
    SaveAs,
    Import,
    Spice,
    SVG,
    Bom,
    CSV,
    Fabrication,
    Measurements,
    SweepCSV,
    Placement,
    BoardSVG,
    ImportFootprint,
    ExportFootprint,
}
impl FileAction {
    fn title(self) -> &'static str {
        match self {
            Self::Open | Self::Save | Self::SaveAs => "CircuitCraft project",
            Self::Import | Self::Spice => "SPICE netlist",
            Self::SVG => "Schematic SVG",
            Self::Bom => "Bill of materials",
            Self::CSV => "Simulation data",
            Self::Fabrication => "Fabrication folder name",
            Self::Measurements => "Automatic measurements",
            Self::SweepCSV => "Full sweep data",
            Self::Placement => "Footprint placement",
            Self::BoardSVG => "PCB review drawing",
            Self::ImportFootprint | Self::ExportFootprint => "Custom footprint",
        }
    }
    fn ext(self) -> &'static str {
        match self {
            Self::Open | Self::Save | Self::SaveAs => "cforge",
            Self::Import | Self::Spice => "cir",
            Self::SVG => "svg",
            Self::Bom | Self::CSV => "csv",
            Self::Fabrication => "fab",
            Self::Measurements | Self::SweepCSV | Self::Placement => "csv",
            Self::BoardSVG => "svg",
            Self::ImportFootprint | Self::ExportFootprint => "json",
        }
    }
    fn save(self) -> bool {
        !matches!(self, Self::Open | Self::Import | Self::ImportFootprint)
    }
}
#[derive(Clone)]
struct Camera {
    scale: f32,
    pan: Vec2,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            scale: 32.,
            pan: Vec2::ZERO,
        }
    }
}
impl Camera {
    fn screen(&self, r: Rect, p: Mm) -> Pos2 {
        r.center() + self.pan + Vec2::new(p.x as f32, p.y as f32) * self.scale
    }
    fn world(&self, r: Rect, p: Pos2) -> Mm {
        let v = (p - r.center() - self.pan) / self.scale;
        Mm::new(v.x as f64, v.y as f64)
    }
}
struct App {
    sheet: Sheet,
    history: History,
    path: Option<PathBuf>,
    dirty: bool,
    revision: u64,
    job_revision: u64,
    tab: Tab,
    tool: Tool,
    selected: Option<Selection>,
    camera: Camera,
    pcb_camera: Camera,
    canvas: Rect,
    fit: bool,
    rotation: u8,
    wire_start: Option<Grid>,
    route_start: Option<Mm>,
    drag_offset: Option<Grid>,
    pcb_drag_offset: Option<Mm>,
    library_search: String,
    label_name: String,
    probe: Option<String>,
    layer: Copper,
    track_width: f64,
    snap_mm: f64,
    route_net: String,
    show_ratsnest: bool,
    drc_issues: Vec<PcbIssue>,
    drc_stale: bool,
    result: Option<Simulation>,
    job: Option<Receiver<Result<JobOutput, String>>>,
    sweep: Option<SweepResult>,
    sweep_enabled: bool,
    sweep_reference: String,
    sweep_input: String,
    sweep_signal: String,
    cancel: Arc<AtomicBool>,
    noise_output: String,
    noise_source: String,
    math_a: String,
    math_b: String,
    math_op: TraceMath,
    engine: String,
    analysis: usize,
    step: String,
    stop: String,
    uic: bool,
    ac_start: String,
    ac_stop: String,
    ac_points: u32,
    dc_source: String,
    dc_start: String,
    dc_stop: String,
    dc_step: String,
    enabled: Vec<String>,
    plot_mode: usize,
    cursor_index: Option<usize>,
    status: String,
    status_error: bool,
    log: Vec<String>,
    show_log: bool,
    show_help: bool,
    file_action: Option<FileAction>,
    path_input: String,
    close_prompt: bool,
    allow_close: bool,
}
impl Default for App {
    fn default() -> Self {
        Self {
            sheet: Sheet::rc(),
            history: History::default(),
            path: None,
            dirty: false,
            revision: 0,
            job_revision: 0,
            tab: Tab::Schematic,
            tool: Tool::Select,
            selected: None,
            camera: Camera::default(),
            pcb_camera: Camera::default(),
            canvas: Rect::NOTHING,
            fit: true,
            rotation: 0,
            wire_start: None,
            route_start: None,
            drag_offset: None,
            pcb_drag_offset: None,
            library_search: String::new(),
            label_name: "SIGNAL".into(),
            probe: None,
            layer: Copper::Top,
            track_width: 0.3,
            snap_mm: 0.25,
            route_net: "0".into(),
            show_ratsnest: true,
            drc_issues: vec![],
            drc_stale: true,
            result: None,
            job: None,
            sweep: None,
            sweep_enabled: false,
            sweep_reference: "R1".into(),
            sweep_input: "500, 1k, 2k".into(),
            sweep_signal: "v(vout)".into(),
            cancel: Arc::new(AtomicBool::new(false)),
            noise_output: "VOUT".into(),
            noise_source: "V1".into(),
            math_a: "v(vout)".into(),
            math_b: "v(vin)".into(),
            math_op: TraceMath::Ratio,
            engine: find_engine().to_string_lossy().into_owned(),
            analysis: 2,
            step: "10u".into(),
            stop: "5m".into(),
            uic: true,
            ac_start: "1".into(),
            ac_stop: "100k".into(),
            ac_points: 100,
            dc_source: "V1".into(),
            dc_start: "0".into(),
            dc_stop: "10".into(),
            dc_step: "0.1".into(),
            enabled: vec!["v(vout)".into()],
            plot_mode: 0,
            cursor_index: None,
            status: "Ready · choose Run to simulate".into(),
            status_error: false,
            log: vec!["Ready. Included RC example: choose Run, or File → New.".into()],
            show_log: false,
            show_help: false,
            file_action: None,
            path_input: String::new(),
            close_prompt: false,
            allow_close: false,
        }
    }
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        style(&cc.egui_ctx);
        Self::default()
    }
    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.result = None;
        self.sweep = None;
        self.cancel.store(true, Ordering::Relaxed);
        self.cursor_index = None;
        self.probe = None;
    }
    fn checkpoint(&mut self) {
        self.history.record(&self.sheet);
        self.dirty = true;
        self.drc_stale = true;
        self.invalidate();
    }
    fn changed(&mut self, before: &Sheet) {
        self.history.record(before);
        self.dirty = true;
        self.drc_stale = true;
        self.invalidate();
    }
    fn say(&mut self, s: impl Into<String>, error: bool) {
        let s = s.into();
        self.status = s
            .lines()
            .next()
            .unwrap_or("Ready")
            .chars()
            .take(180)
            .collect();
        self.status_error = error;
        self.log.push(s);
        if self.log.len() > 100 {
            self.log.remove(0);
        }
    }
    fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        self.wire_start = None;
        self.route_start = None;
        self.drag_offset = None;
        self.pcb_drag_offset = None;
    }
    fn replace(&mut self, s: Sheet) {
        self.checkpoint();
        self.sheet = s;
        self.path = None;
        self.selected = None;
        self.fit = true;
        self.tab = Tab::Schematic;
        self.set_tool(Tool::Select);
    }
    fn delete(&mut self) {
        if let Some(s) = self.selected {
            self.checkpoint();
            match s {
                Selection::Component(id) => self.sheet.components.retain(|c| c.id != id),
                Selection::Wire(id) => self.sheet.wires.retain(|w| w.id != id),
                Selection::Label(id) => self.sheet.labels.retain(|l| l.id != id),
                Selection::Ground(p) => self.sheet.grounds.retain(|g| *g != p),
                Selection::Junction(p) => self.sheet.junctions.retain(|j| *j != p),
                Selection::Footprint(id) => {
                    self.sheet.board.footprints.retain(|f| f.component_id != id)
                }
                Selection::Track(id) => self.sheet.board.tracks.retain(|t| t.id != id),
                Selection::Via(id) => self.sheet.board.vias.retain(|v| v.id != id),
            }
            self.selected = None;
        }
    }
    fn rotate(&mut self) {
        if matches!(self.tool, Tool::Place(_)) {
            self.rotation = (self.rotation + 1) % 4;
            return;
        }
        match self.selected {
            Some(Selection::Component(id)) => {
                self.checkpoint();
                if let Some(c) = self.sheet.components.iter_mut().find(|c| c.id == id) {
                    c.rotation = (c.rotation + 1) % 4;
                }
            }
            Some(Selection::Footprint(id)) => {
                self.checkpoint();
                if let Some(f) = self
                    .sheet
                    .board
                    .footprints
                    .iter_mut()
                    .find(|f| f.component_id == id)
                {
                    f.rotation = (f.rotation + 1) % 4;
                }
            }
            _ => self.rotation = (self.rotation + 1) % 4,
        }
    }
    fn duplicate(&mut self) {
        if let Some(Selection::Component(id)) = self.selected {
            if let Some(c) = self.sheet.components.iter().find(|c| c.id == id).cloned() {
                self.checkpoint();
                let id = self.sheet.add(c.kind, c.at.plus(Grid::new(4, 0)));
                let copy = self
                    .sheet
                    .components
                    .iter_mut()
                    .find(|c| c.id == id)
                    .unwrap();
                copy.value = c.value;
                copy.params = c.params;
                copy.rotation = c.rotation;
                copy.footprint = c.footprint;
                copy.custom_pads = c.custom_pads;
                self.selected = Some(Selection::Component(id));
            }
        }
    }
    fn body(&self) -> Result<String, String> {
        if self.sheet.raw_mode {
            Ok(self.sheet.raw_netlist.clone())
        } else {
            netlist(&self.sheet)
        }
    }
    fn analysis(&self) -> Result<Analysis, String> {
        Ok(match self.analysis {
            0 => Analysis::OperatingPoint,
            1 => Analysis::Transient {
                step: parse_value(&self.step)?,
                stop: parse_value(&self.stop)?,
                uic: self.uic,
            },
            2 => Analysis::Ac {
                points_per_decade: self.ac_points,
                start: parse_value(&self.ac_start)?,
                stop: parse_value(&self.ac_stop)?,
            },
            3 => Analysis::DcSweep {
                source: self.dc_source.clone(),
                start: parse_value(&self.dc_start)?,
                stop: parse_value(&self.dc_stop)?,
                step: parse_value(&self.dc_step)?,
            },
            _ => Analysis::Noise {
                output: self.noise_output.clone(),
                source: self.noise_source.clone(),
                points_per_decade: self.ac_points,
                start: parse_value(&self.ac_start)?,
                stop: parse_value(&self.ac_stop)?,
            },
        })
    }
    fn run(&mut self, builtin: bool) {
        if self.job.is_some() {
            return;
        }
        if builtin && self.sheet.raw_mode {
            self.say(
                "Linear DC uses the schematic; disable edited-netlist mode first.",
                true,
            );
            return;
        }
        let (tx, rx) = mpsc::channel();
        let sheet = self.sheet.clone();
        let body = self.body();
        let analysis = self.analysis();
        let engine = PathBuf::from(&self.engine);
        let sweep = self.sweep_enabled && !builtin;
        let reference = self.sweep_reference.clone();
        let values = sweep_values(&self.sweep_input);
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let base = if !sheet.include_directory.is_empty() {
            Some(PathBuf::from(&sheet.include_directory))
        } else {
            self.path
                .as_ref()
                .and_then(|p| p.parent().map(Path::to_path_buf))
        };
        std::thread::spawn(move || {
            let r = if builtin {
                linear_dc(&sheet).map(JobOutput::Simulation)
            } else if sweep {
                values
                    .and_then(|values| {
                        analysis.and_then(|a| {
                            run_sweep(
                                &sheet,
                                &reference,
                                &values,
                                &a,
                                &engine,
                                base.as_deref(),
                                cancel,
                            )
                        })
                    })
                    .map(JobOutput::Sweep)
            } else {
                body.and_then(|body| {
                    analysis.and_then(|a| run_spice(&body, &a, &engine, base.as_deref()))
                })
                .map(JobOutput::Simulation)
            };
            let _ = tx.send(r);
        });
        self.job = Some(rx);
        self.job_revision = self.revision;
        self.say("Running simulation…", false);
    }
    fn poll(&mut self) {
        if let Some(rx) = &self.job {
            match rx.try_recv() {
                Ok(r) => {
                    self.job = None;
                    if self.job_revision != self.revision {
                        self.say(
                            "Circuit changed during simulation; run again for current results.",
                            false,
                        );
                        return;
                    }
                    match r {
                        Ok(output) => {
                            let sim =
                                match output {
                                    JobOutput::Simulation(sim) => {
                                        self.sweep = None;
                                        sim
                                    }
                                    JobOutput::Sweep(sweep) => {
                                        if !sweep.runs[0].1.traces.iter().any(|t| {
                                            t.name.eq_ignore_ascii_case(&self.sweep_signal)
                                        }) {
                                            self.sweep_signal = sweep.runs[0]
                                                .1
                                                .traces
                                                .iter()
                                                .find(|t| t.unit == "voltage")
                                                .or_else(|| sweep.runs[0].1.traces.first())
                                                .map(|t| t.name.clone())
                                                .unwrap_or_default();
                                        }
                                        let sim = match sweep.overlay(&self.sweep_signal) {
                                            Ok(sim) => sim,
                                            Err(e) => {
                                                self.say(e, true);
                                                return;
                                            }
                                        };
                                        self.enabled =
                                            sim.traces.iter().map(|t| t.name.clone()).collect();
                                        self.sweep = Some(sweep);
                                        sim
                                    }
                                };
                            if !sim.traces.iter().any(|t| self.enabled.contains(&t.name)) {
                                self.enabled = sim
                                    .traces
                                    .iter()
                                    .filter(|t| t.unit == "voltage" || t.name == "onoise_spectrum")
                                    .take(1)
                                    .map(|t| t.name.clone())
                                    .collect();
                                if self.enabled.is_empty() {
                                    self.enabled =
                                        sim.traces.iter().take(1).map(|t| t.name.clone()).collect();
                                }
                            }
                            self.say(
                                format!(
                                    "{} · {} samples · {:.3} s",
                                    sim.title,
                                    sim.axis.len(),
                                    sim.elapsed
                                ),
                                false,
                            );
                            self.log.push(sim.log.clone());
                            self.result = Some(sim);
                            self.tab = Tab::Simulation;
                        }
                        Err(e) => {
                            self.show_log = true;
                            self.say(e, true)
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.job = None;
                    self.say("Simulation worker disconnected.", true);
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    fn request_file(&mut self, a: FileAction) {
        if matches!(a, FileAction::Save) {
            if let Some(p) = self.path.clone() {
                self.file_do(a, &p);
                return;
            }
        }
        let default = self
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("circuit.cforge"))
            .with_extension(a.ext());
        if let Some(p) = files::choose(
            a.save(),
            a.title(),
            a.ext(),
            if a.save() { Some(&default) } else { None },
        ) {
            self.file_do(a, &p);
        } else if !cfg!(windows) {
            self.file_action = Some(a);
            self.path_input = default.to_string_lossy().into_owned();
        }
    }
    fn file_do(&mut self, a: FileAction, p: &Path) {
        let r: Result<String, String> = (|| match a {
            FileAction::Save | FileAction::SaveAs => {
                fs::write(p, self.sheet.to_json()?).map_err(|e| e.to_string())?;
                self.path = Some(fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()));
                self.dirty = false;
                Ok(format!("Saved {}", p.display()))
            }
            FileAction::Open => {
                let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
                self.replace(Sheet::from_json(&text)?);
                self.path = Some(fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()));
                self.dirty = false;
                Ok(format!("Opened {}", p.display()))
            }
            FileAction::Import => {
                let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
                self.checkpoint();
                self.sheet.raw_netlist = text;
                self.sheet.raw_mode = true;
                self.sheet.include_directory = fs::canonicalize(p)
                    .ok()
                    .and_then(|p| p.parent().map(Path::to_path_buf))
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.tab = Tab::Spice;
                Ok("Imported textual SPICE. Graphical schematic remains separate.".into())
            }
            FileAction::Spice => {
                let body = self
                    .body()?
                    .lines()
                    .filter(|l| l.trim().to_lowercase().split_whitespace().next() != Some(".end"))
                    .collect::<Vec<_>>()
                    .join("\n");
                fs::write(p, format!("{body}\n.end\n")).map_err(|e| e.to_string())?;
                Ok("SPICE exported.".into())
            }
            FileAction::SVG => {
                fs::write(p, schematic_svg(&self.sheet)).map_err(|e| e.to_string())?;
                Ok("SVG exported.".into())
            }
            FileAction::Bom => {
                fs::write(p, bom_csv(&self.sheet)).map_err(|e| e.to_string())?;
                Ok("BOM exported.".into())
            }
            FileAction::CSV => {
                let sim = self.result.as_ref().ok_or("Run a simulation first.")?;
                fs::write(p, sim.csv()).map_err(|e| e.to_string())?;
                Ok("Full-resolution data exported.".into())
            }
            FileAction::Fabrication => {
                let files = self.sheet.board.gerbers()?;
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
                for (n, data) in files {
                    fs::write(p.join(n), data).map_err(|e| e.to_string())?;
                }
                self.drc_issues = self.sheet.pcb_drc();
                self.drc_stale = false;
                Ok(format!("Fabrication files exported; {} DRC issues. Check in an independent CAM viewer.",self.drc_issues.len()))
            }
            FileAction::Measurements => {
                let csv = if let Some(sweep) = &self.sweep {
                    sweep.measurements_csv()
                } else {
                    measurements_csv(self.result.as_ref().ok_or("Run a simulation first.")?)
                };
                fs::write(p, csv).map_err(|e| e.to_string())?;
                Ok("Measurements exported.".into())
            }
            FileAction::SweepCSV => {
                fs::write(
                    p,
                    self.sweep
                        .as_ref()
                        .ok_or("Run a component sweep first.")?
                        .csv(),
                )
                .map_err(|e| e.to_string())?;
                Ok("All original sweep samples exported.".into())
            }
            FileAction::Placement => {
                fs::write(p, self.sheet.board.placement_csv()).map_err(|e| e.to_string())?;
                Ok("Placement CSV exported: millimetres, origin bottom left.".into())
            }
            FileAction::BoardSVG => {
                fs::write(p, self.sheet.board.svg()?).map_err(|e| e.to_string())?;
                Ok("PCB review SVG exported.".into())
            }
            FileAction::ImportFootprint | FileAction::ExportFootprint => {
                let id = match self.selected {
                    Some(Selection::Component(id)) => id,
                    _ => return Err("Select a schematic component first.".into()),
                };
                if matches!(a, FileAction::ImportFootprint) {
                    let def = FootprintDefinition::from_json(
                        &fs::read_to_string(p).map_err(|e| e.to_string())?,
                    )?;
                    let c = self
                        .sheet
                        .components
                        .iter()
                        .find(|c| c.id == id)
                        .ok_or("Component missing")?;
                    if def.pads.len() != c.pins().len() {
                        return Err("Pad count must match the component's SPICE terminals.".into());
                    }
                    self.checkpoint();
                    let c = self
                        .sheet
                        .components
                        .iter_mut()
                        .find(|c| c.id == id)
                        .unwrap();
                    c.footprint = def.name;
                    c.custom_pads = def.pads;
                    Ok("Custom footprint imported. Synchronize the PCB.".into())
                } else {
                    let c = self
                        .sheet
                        .components
                        .iter()
                        .find(|c| c.id == id)
                        .ok_or("Component missing")?;
                    let f = component_footprint(c);
                    fs::write(p, f.to_json()?).map_err(|e| e.to_string())?;
                    Ok("Reusable footprint JSON exported.".into())
                }
            }
        })();
        match r {
            Ok(s) => self.say(s, false),
            Err(e) => {
                self.show_log = true;
                self.say(e, true)
            }
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
            self.request_file(FileAction::Save);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O)) {
            self.request_file(FileAction::Open);
        }
        if ctx.input(|i| i.key_pressed(Key::F5)) {
            self.run(false);
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            let ok = if ctx.input(|i| i.modifiers.shift) {
                self.history.redo(&mut self.sheet)
            } else {
                self.history.undo(&mut self.sheet)
            };
            if ok {
                self.dirty = true;
                self.drc_stale = true;
                self.invalidate();
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Y)) {
            if self.history.redo(&mut self.sheet) {
                self.dirty = true;
                self.drc_stale = true;
                self.invalidate();
            }
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::D)) {
            self.duplicate();
        }
        if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            self.delete();
        }
        if ctx.input(|i| i.key_pressed(Key::R)) {
            self.rotate();
        }
        for (k, t) in [
            (Key::V, Tool::Select),
            (Key::G, Tool::Ground),
            (Key::J, Tool::Junction),
            (Key::L, Tool::Label),
        ] {
            if ctx.input(|i| i.key_pressed(k)) {
                self.set_tool(t);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::W)) {
            self.set_tool(if self.tab == Tab::Pcb {
                Tool::Route
            } else {
                Tool::Wire
            });
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.set_tool(Tool::Select);
        }
        if ctx.input(|i| i.key_pressed(Key::Home)) {
            self.fit = true;
        }
    }
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("⚡ CircuitCraft")
                            .strong()
                            .size(23.)
                            .color(GREEN),
                    );
                    ui.label(
                        RichText::new("SCHEMATIC / SIMULATION / PCB")
                            .size(10.)
                            .color(MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                self.job.is_none(),
                                egui::Button::new("▶ Run · F5")
                                    .fill(Color32::from_rgb(24, 111, 91)),
                            )
                            .clicked()
                        {
                            self.run(false);
                        }
                        if self.job.is_some() {
                            ui.spinner();
                        }
                        if ui.button("Help").clicked() {
                            self.show_help = true;
                        }
                        if ui.button("Save").clicked() {
                            self.request_file(FileAction::Save);
                        }
                    });
                });
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New empty schematic").clicked() {
                            self.replace(Sheet::default());
                            ui.close_menu();
                        }
                        for (a, s) in [
                            (FileAction::Open, "Open project…"),
                            (FileAction::Save, "Save · Ctrl+S"),
                            (FileAction::SaveAs, "Save as…"),
                            (FileAction::Import, "Import textual SPICE…"),
                            (FileAction::Spice, "Export SPICE…"),
                            (FileAction::SVG, "Export schematic SVG…"),
                            (FileAction::Bom, "Export BOM CSV…"),
                            (FileAction::CSV, "Export waveform CSV…"),
                            (FileAction::Fabrication, "Export Gerber + drills…"),
                            (FileAction::Measurements, "Export measurements CSV…"),
                            (FileAction::SweepCSV, "Export full sweep CSV…"),
                            (FileAction::Placement, "Export placement CSV…"),
                            (FileAction::BoardSVG, "Export PCB SVG…"),
                            (
                                FileAction::ImportFootprint,
                                "Import footprint for selected part…",
                            ),
                            (FileAction::ExportFootprint, "Export selected footprint…"),
                        ] {
                            if ui.button(s).clicked() {
                                self.request_file(a);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.menu_button("Examples", |ui| {
                        if ui.button("Voltage divider").clicked() {
                            self.replace(Sheet::divider());
                            self.analysis = 0;
                            ui.close_menu();
                        }
                        if ui.button("RC low-pass").clicked() {
                            self.replace(Sheet::rc());
                            self.analysis = 2;
                            ui.close_menu();
                        }
                        if ui.button("Half-wave rectifier").clicked() {
                            self.replace(Sheet::rectifier());
                            self.analysis = 1;
                            self.step = "100u".into();
                            self.stop = "200m".into();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo · Ctrl+Z").clicked() {
                            if self.history.undo(&mut self.sheet) {
                                self.dirty = true;
                                self.drc_stale = true;
                                self.invalidate();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Redo · Ctrl+Y").clicked() {
                            if self.history.redo(&mut self.sheet) {
                                self.dirty = true;
                                self.drc_stale = true;
                                self.invalidate();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Duplicate · Ctrl+D").clicked() {
                            self.duplicate();
                            ui.close_menu();
                        }
                        if ui.button("Rotate · R").clicked() {
                            self.rotate();
                            ui.close_menu();
                        }
                        if ui.button("Delete").clicked() {
                            self.delete();
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    for (t, s) in [
                        (Tab::Schematic, "Schematic"),
                        (Tab::Pcb, "PCB layout"),
                        (Tab::Simulation, "Waveforms"),
                        (Tab::Spice, "SPICE"),
                        (Tab::Bom, "BOM"),
                    ] {
                        if ui.selectable_label(self.tab == t, s).clicked() {
                            self.set_tool(Tool::Select);
                            self.selected = None;
                            if t == Tab::Pcb {
                                self.fit = true;
                            }
                            self.tab = t;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.dirty {
                                "● Unsaved"
                            } else {
                                "Saved / example"
                            })
                            .size(11.)
                            .color(if self.dirty {
                                ORANGE
                            } else {
                                MUTED
                            }),
                        );
                    });
                });
            });
    }
    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.show_log, "Console").clicked() {
                        self.show_log = !self.show_log;
                    }
                    ui.colored_label(if self.status_error { RED } else { MUTED }, &self.status);
                });
            });
        if self.show_log {
            egui::TopBottomPanel::bottom("console")
                .resizable(true)
                .default_height(140.)
                .max_height(300.)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for l in &self.log {
                                ui.label(RichText::new(l).monospace().size(11.));
                            }
                        });
                });
        }
    }
    fn left(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("library").default_width(230.).resizable(true).width_range(190. ..=330.).frame(egui::Frame::new().fill(PANEL).inner_margin(14)).show(ctx,|ui|{
 ui.label(RichText::new("PROJECT").strong().size(10.).color(MUTED));let before=self.sheet.clone();
 if ui.add(egui::TextEdit::singleline(&mut self.sheet.title).desired_width(f32::INFINITY)).changed(){self.history.record(&before);self.dirty=true;}
 ui.label(RichText::new(format!("{} parts · {} wires",self.sheet.components.len(),self.sheet.wires.len())).size(11.).color(MUTED));
 egui::CollapsingHeader::new("Project notes").show(ui,|ui|{let before=self.sheet.clone();if ui.add(egui::TextEdit::multiline(&mut self.sheet.notes).desired_rows(4)).changed(){self.history.record(&before);self.dirty=true;}});
 ui.add_space(12.);
 match self.tab{
 Tab::Pcb=>self.pcb_tools(ui),
 Tab::Simulation=>{
 self.simulation_tools(ui);ui.separator();ui.label(RichText::new("SIGNALS").size(11.).strong().color(MUTED));
 if let Some(sim)=&self.result{egui::ScrollArea::vertical().max_height((ui.available_height()-50.).max(30.)).show(ui,|ui|{
 for(i,t)in sim.traces.iter().enumerate(){let mut enabled=self.enabled.contains(&t.name);ui.horizontal(|ui|{let(dot,_)=ui.allocate_exact_size(Vec2::new(8.,8.),Sense::hover());ui.painter().circle_filled(dot.center(),3.,plot::color(i));if ui.checkbox(&mut enabled,&t.name).changed(){if enabled{self.enabled.push(t.name.clone());}else{self.enabled.retain(|n|n!=&t.name);}}});}
 });}if ui.button("Export all data…").clicked(){self.request_file(FileAction::CSV);}
 }
 _=>{
 ui.label(RichText::new("TOOLS").strong().size(11.).color(MUTED));ui.horizontal_wrapped(|ui|{for(t,s)in [(Tool::Select,"Select · V"),(Tool::Wire,"Wire · W"),(Tool::Ground,"Ground · G"),(Tool::Junction,"Junction · J"),(Tool::Label,"Label · L"),(Tool::Probe,"Probe")]{if ui.selectable_label(self.tool==t,s).clicked(){self.set_tool(t);}}});
 if self.tool==Tool::Label{ui.text_edit_singleline(&mut self.label_name);}
 ui.add_space(10.);ui.separator();ui.label(RichText::new("COMPONENTS · choose, then place").size(11.).strong().color(MUTED));
 ui.add(egui::TextEdit::singleline(&mut self.library_search).hint_text("Search components…"));let filter=self.library_search.to_lowercase();
 egui::ScrollArea::vertical().max_height((ui.available_height()-80.).max(40.)).show(ui,|ui|{for k in Kind::ALL{if !filter.is_empty()&&!k.name().to_lowercase().contains(&filter){continue;}ui.horizontal(|ui|{ui.label(RichText::new(format!("{:>2}",k.prefix())).monospace().color(GREEN));if ui.selectable_label(self.tool==Tool::Place(k),k.name()).clicked(){self.set_tool(Tool::Place(k));}});}});
 ui.separator();ui.label(RichText::new("R rotates · Esc ends wire\nMiddle drag pans · Wheel zooms\nHome fits the circuit").size(11.).color(MUTED));
 }}});
    }
    fn simulation_tools(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("ANALYSIS").strong().size(11.).color(MUTED));
        egui::ComboBox::from_id_salt("analysis")
            .selected_text(
                [
                    "Operating point",
                    "Transient",
                    "AC frequency sweep",
                    "DC source sweep",
                    "Noise spectrum",
                ][self.analysis],
            )
            .show_ui(ui, |ui| {
                for (i, s) in [
                    "Operating point",
                    "Transient",
                    "AC frequency sweep",
                    "DC source sweep",
                    "Noise spectrum",
                ]
                .iter()
                .enumerate()
                {
                    ui.selectable_value(&mut self.analysis, i, *s);
                }
            });
        match self.analysis {
            1 => {
                row(ui, "Step", &mut self.step);
                row(ui, "Stop", &mut self.stop);
                ui.checkbox(&mut self.uic, "Skip initial operating point");
            }
            2 => {
                row(ui, "Start Hz", &mut self.ac_start);
                row(ui, "Stop Hz", &mut self.ac_stop);
                ui.horizontal(|ui| {
                    ui.label("Points / decade");
                    ui.add(egui::DragValue::new(&mut self.ac_points).range(1..=1000));
                });
            }
            3 => {
                row(ui, "Source", &mut self.dc_source);
                row(ui, "Start", &mut self.dc_start);
                row(ui, "Stop", &mut self.dc_stop);
                row(ui, "Step", &mut self.dc_step);
            }
            4 => {
                row(ui, "Output node", &mut self.noise_output);
                row(ui, "Input source", &mut self.noise_source);
                row(ui, "Start Hz", &mut self.ac_start);
                row(ui, "Stop Hz", &mut self.ac_stop);
                ui.horizontal(|ui| {
                    ui.label("Points / decade");
                    ui.add(egui::DragValue::new(&mut self.ac_points).range(1..=1000));
                });
                ui.label(
                    RichText::new("Small-signal noise density. Input source needs AC excitation.")
                        .size(10.)
                        .color(MUTED),
                );
            }
            _ => {
                ui.label(
                    RichText::new("Steady-state node voltages and branch currents.")
                        .size(11.)
                        .color(MUTED),
                );
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.job.is_none(),
                    egui::Button::new("▶ Run ngspice").fill(Color32::from_rgb(24, 111, 91)),
                )
                .clicked()
            {
                self.run(false);
            }
            if ui
                .add_enabled(self.job.is_none(), egui::Button::new("Linear DC"))
                .clicked()
            {
                self.run(true);
            }
        });
        egui::CollapsingHeader::new("Engine settings").show(ui,|ui|{ui.label("ngspice executable");ui.add(egui::TextEdit::singleline(&mut self.engine));ui.label(RichText::new("Windows package: ngspice included.\nLinux/macOS: install ngspice.\nGeneric device models are demonstrations; import manufacturer models for accuracy.").size(10.).color(MUTED));});
        ui.label(
            RichText::new("Values: 1k, 4.7u, 10m, 1MEG.\nSPICE M=milli; MEG=mega.")
                .size(10.)
                .color(MUTED),
        );
        self.advanced_tools(ui);
    }
    fn pcb_tools(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("PCB TOOLS").size(11.).strong().color(MUTED));
        ui.horizontal_wrapped(|ui| {
            for (t, s) in [
                (Tool::Select, "Select"),
                (Tool::Route, "Route · W"),
                (Tool::Via, "Via"),
            ] {
                if ui.selectable_label(self.tool == t, s).clicked() {
                    self.set_tool(t);
                }
            }
        });
        ui.label("Copper layer");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.layer, Copper::Top, "Top");
            ui.selectable_value(&mut self.layer, Copper::Bottom, "Bottom");
        });
        ui.horizontal(|ui| {
            ui.label("Track mm");
            ui.add(
                egui::DragValue::new(&mut self.track_width)
                    .speed(0.025)
                    .range(0.1..=5.),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Grid mm");
            ui.add(
                egui::DragValue::new(&mut self.snap_mm)
                    .speed(0.05)
                    .range(0.05..=5.),
            );
        });
        let n = connect(&self.sheet);
        egui::ComboBox::from_id_salt("route-net")
            .selected_text(format!("Net {}", self.route_net))
            .show_ui(ui, |ui| {
                for net in n.nets {
                    ui.selectable_value(&mut self.route_net, net.name.clone(), net.name);
                }
            });
        ui.checkbox(&mut self.show_ratsnest, "Show unrouted connections");
        ui.label(RichText::new("Start on a pad to select its net. Click corners/endpoints; Esc finishes. Use a via to join the two layers.").size(11.).color(MUTED));
        ui.add_space(10.);
        ui.separator();
        if ui.button("Synchronize from schematic").clicked() {
            self.checkpoint();
            self.sheet.sync_board();
            self.say(
                "Footprints updated; existing routing retained. Run DRC.",
                false,
            );
        }
        if ui.button("Run design rule check").clicked() {
            self.drc_issues = self.sheet.pcb_drc();
            self.drc_stale = false;
            self.say(
                format!("DRC: {} issues", self.drc_issues.len()),
                !self.drc_issues.is_empty(),
            );
        }
        if ui.button("Export Gerber + drills…").clicked() {
            self.request_file(FileAction::Fabrication);
        }
        ui.add_space(10.);
        ui.label(
            RichText::new("BOARD & RULES · mm")
                .strong()
                .size(11.)
                .color(MUTED),
        );
        let before = self.sheet.clone();
        let mut changed = false;
        egui::Grid::new("rules").show(ui, |ui| {
            for (name, value) in [
                ("Width", &mut self.sheet.board.width),
                ("Height", &mut self.sheet.board.height),
                ("Clearance", &mut self.sheet.board.clearance),
                ("Edge clearance", &mut self.sheet.board.edge_clearance),
                ("Min. track", &mut self.sheet.board.min_track),
            ] {
                ui.label(name);
                changed |= ui
                    .add(egui::DragValue::new(value).speed(0.1).range(0.05..=2000.))
                    .changed();
                ui.end_row();
            }
        });
        if changed {
            self.history.record(&before);
            self.dirty = true;
            self.drc_stale = true;
        }
        ui.add_space(8.);
        ui.label(RichText::new("Pad numbering follows SPICE terminal order. Check against the physical component datasheet.").size(11.).color(ORANGE));
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("INSPECTOR").strong().size(11.).color(MUTED));
        let before = self.sheet.clone();
        let mut changed = false;
        match self.selected {
            Some(Selection::Component(id)) => {
                if let Some(c) = self.sheet.components.iter_mut().find(|c| c.id == id) {
                    ui.label(RichText::new(c.kind.name()).strong().size(17.));
                    changed |= row(ui, "Reference", &mut c.reference);
                    changed |= row(ui, "Value / model", &mut c.value);
                    changed |= row(ui, "Parameters", &mut c.params);
                    ui.horizontal(|ui| {
                        ui.label("Grid X / Y");
                        changed |= ui
                            .add(egui::DragValue::new(&mut c.at.x).range(-100000..=100000))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut c.at.y).range(-100000..=100000))
                            .changed();
                    });
                    egui::ComboBox::from_id_salt("footprint")
                        .selected_text(&c.footprint)
                        .show_ui(ui, |ui| {
                            for f in FOOTPRINTS {
                                if ui.selectable_value(&mut c.footprint, f.into(), f).changed() {
                                    c.custom_pads.clear();
                                    changed = true;
                                }
                            }
                        });
                    ui.label(
                        RichText::new(format!(
                            "Pins: {}",
                            c.pins()
                                .iter()
                                .enumerate()
                                .map(|(i, (n, _))| format!("{}={n}", i + 1))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                        .size(11.)
                        .color(MUTED),
                    );
                    if matches!(c.kind, Kind::Voltage | Kind::Current) {
                        ui.label(
                            RichText::new("DC 5 AC 1\nPULSE(0 5 0 1n 1n 1m 2m)\nSIN(0 5 50)")
                                .monospace()
                                .size(10.)
                                .color(MUTED),
                        );
                    }
                }
                changed |= self.footprint_editor(ui, id);
            }
            Some(Selection::Label(id)) => {
                if let Some(l) = self.sheet.labels.iter_mut().find(|l| l.id == id) {
                    changed |= row(ui, "Net name", &mut l.name);
                }
            }
            Some(Selection::Footprint(id)) => {
                if let Some(f) = self
                    .sheet
                    .board
                    .footprints
                    .iter_mut()
                    .find(|f| f.component_id == id)
                {
                    ui.label(RichText::new(&f.reference).strong().size(18.));
                    ui.label(&f.style);
                    ui.horizontal(|ui| {
                        ui.label("X / Y mm");
                        changed |= ui
                            .add(egui::DragValue::new(&mut f.at.x).speed(0.25))
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut f.at.y).speed(0.25))
                            .changed();
                    });
                    for p in f.pads() {
                        ui.label(format!("Pad {} → {}", p.number, p.net));
                    }
                }
            }
            Some(Selection::Track(id)) => {
                if let Some(t) = self.sheet.board.tracks.iter_mut().find(|t| t.id == id) {
                    ui.label("Copper track");
                    changed |= row(ui, "Net", &mut t.net);
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut t.width)
                                    .speed(0.025)
                                    .range(0.1..=5.),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= ui
                            .selectable_value(&mut t.layer, Copper::Top, "Top")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut t.layer, Copper::Bottom, "Bottom")
                            .changed();
                    });
                }
            }
            Some(Selection::Via(id)) => {
                if let Some(v) = self.sheet.board.vias.iter_mut().find(|v| v.id == id) {
                    ui.label("Plated via");
                    changed |= row(ui, "Net", &mut v.net);
                    ui.horizontal(|ui| {
                        ui.label("Diameter / drill");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut v.diameter)
                                    .speed(0.05)
                                    .range(0.3..=5.),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut v.drill)
                                    .speed(0.05)
                                    .range(0.1..=4.),
                            )
                            .changed();
                    });
                }
            }
            Some(Selection::Wire(_)) => {
                ui.label("Electrical wire");
                ui.label(RichText::new("Crossings remain separate unless joined by a terminal, vertex, label or explicit junction.").size(11.).color(MUTED));
            }
            Some(Selection::Ground(_)) => {
                ui.label("Ground · SPICE node 0");
            }
            Some(Selection::Junction(_)) => {
                ui.label("Explicit electrical junction");
            }
            None => {
                ui.label(
                    RichText::new("Select a component, wire or footprint to edit its properties.")
                        .size(12.)
                        .color(MUTED),
                );
            }
        }
        if changed {
            self.changed(&before);
        }
        if self.selected.is_some() {
            ui.horizontal(|ui| {
                if ui.button("Rotate · R").clicked() {
                    self.rotate();
                }
                if ui.button("Delete").clicked() {
                    self.delete();
                }
            });
        }
        if let Some(net) = &self.probe {
            ui.colored_label(GREEN, format!("Highlighted net {net}"));
        }
    }
    fn right(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .default_width(282.)
            .resizable(true)
            .width_range(220. ..=420.)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
            .show(ctx, |ui| {
                if self.tab == Tab::Pcb {
                    self.inspector(ui);
                    ui.separator();
                    ui.label(
                        RichText::new("DESIGN RULE CHECK")
                            .strong()
                            .size(11.)
                            .color(MUTED),
                    );
                    if self.drc_stale {
                        ui.colored_label(ORANGE, "Changed since last check");
                    }
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if !self.drc_stale && self.drc_issues.is_empty() {
                            ui.colored_label(GREEN, "No issues found by implemented rules");
                        }
                        for d in &self.drc_issues {
                            ui.colored_label(RED, &d.message);
                            ui.add_space(4.);
                        }
                    });
                } else if self.tab == Tab::Simulation {
                    ui.label(
                        RichText::new("MEASUREMENTS")
                            .strong()
                            .size(11.)
                            .color(MUTED),
                    );
                    if let Some(sim) = &self.result {
                        ui.label(&sim.title);
                        ui.label(format!("{} samples", sim.axis.len()));
                        let i = self.cursor_index.unwrap_or(sim.axis.len() - 1);
                        ui.label(format!(
                            "{} = {}",
                            sim.axis_name,
                            plot::engineering(sim.axis[i])
                        ));
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for t in sim
                                .traces
                                .iter()
                                .filter(|t| self.enabled.contains(&t.name) || sim.axis.len() == 1)
                            {
                                ui.add_space(8.);
                                ui.colored_label(GREEN, &t.name);
                                ui.label(format!(
                                    "{} {}",
                                    plot::engineering(t.real[i]),
                                    trace_unit(t)
                                ));
                                if sim.complex {
                                    ui.label(format!(
                                        "Magnitude: {}",
                                        plot::engineering(t.magnitude(i))
                                    ));
                                    ui.label(format!("{:.3} dB · {:.2}°", t.db(i), t.phase(i)));
                                }
                                let min = t.real.iter().copied().fold(f64::INFINITY, f64::min);
                                let max = t.real.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                                ui.label(
                                    RichText::new(format!(
                                        "Min {} · Max {}",
                                        plot::engineering(min),
                                        plot::engineering(max)
                                    ))
                                    .size(10.)
                                    .color(MUTED),
                                );
                                for m in measurements(sim, t) {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: {} {}",
                                            m.name,
                                            plot::engineering(m.value),
                                            m.unit
                                        ))
                                        .size(11.)
                                        .color(MUTED),
                                    );
                                }
                            }
                        });
                    } else {
                        ui.label("Run an analysis to see measurements.");
                    }
                } else {
                    self.inspector(ui);
                    ui.separator();
                    egui::CollapsingHeader::new("Simulation")
                        .default_open(true)
                        .show(ui, |ui| self.simulation_tools(ui));
                    let n = connect(&self.sheet);
                    ui.separator();
                    egui::CollapsingHeader::new(format!("Electrical checks · {}", n.issues.len()))
                        .default_open(!n.issues.is_empty())
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(200.)
                                .show(ui, |ui| {
                                    if n.issues.is_empty() {
                                        ui.colored_label(GREEN, "No schematic ERC issues.");
                                    }
                                    for i in &n.issues {
                                        if ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new(&i.message).color(if i.error {
                                                        RED
                                                    } else {
                                                        ORANGE
                                                    }),
                                                )
                                                .sense(Sense::click()),
                                            )
                                            .clicked()
                                        {
                                            self.selected = i.component.map(Selection::Component);
                                        }
                                    }
                                });
                        });
                    egui::CollapsingHeader::new(format!("Nets · {}", n.nets.len())).show(
                        ui,
                        |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(150.)
                                .show(ui, |ui| {
                                    for net in n.nets {
                                        if ui
                                            .selectable_label(
                                                self.probe.as_ref() == Some(&net.name),
                                                format!(
                                                    "{} · {} terminals",
                                                    net.name,
                                                    net.pins.len()
                                                ),
                                            )
                                            .clicked()
                                        {
                                            self.probe = Some(net.name);
                                        }
                                    }
                                });
                        },
                    );
                }
            });
    }
    fn spice_workspace(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new().inner_margin(18).show(ui,|ui|{
 ui.heading("SPICE workspace");ui.label(RichText::new("Models, parameters, includes and subcircuits. Textual imports remain separate from the graphical schematic.").size(12.).color(MUTED));
 let before=self.sheet.clone();let mut changed=ui.checkbox(&mut self.sheet.raw_mode,"Simulate edited netlist instead of schematic").changed();
 if self.sheet.raw_mode&&self.sheet.raw_netlist.is_empty(){self.sheet.raw_netlist=netlist(&self.sheet).unwrap_or_default();}
 ui.horizontal(|ui|{if ui.button("Copy schematic into editor").clicked(){match netlist(&self.sheet){Ok(s)=>{self.sheet.raw_netlist=s;self.sheet.raw_mode=true;changed=true;},Err(e)=>self.say(e,true)}}if ui.button("Import .cir…").clicked(){self.request_file(FileAction::Import);}if ui.button("Export .cir…").clicked(){self.request_file(FileAction::Spice);}});
 changed|=row(ui,"Include base directory",&mut self.sheet.include_directory);if changed{self.changed(&before);}ui.separator();
 if self.sheet.raw_mode{let before=self.sheet.clone();egui::ScrollArea::both().show(ui,|ui|{if ui.add(egui::TextEdit::multiline(&mut self.sheet.raw_netlist).code_editor().desired_width(f32::INFINITY).desired_rows(32)).changed(){self.changed(&before);}});}
 else{
 ui.label(RichText::new("Additional model definitions").strong());let before=self.sheet.clone();if ui.add(egui::TextEdit::multiline(&mut self.sheet.spice_models).code_editor().desired_width(f32::INFINITY).desired_rows(9).hint_text(".include \"vendor-model.lib\"\n.param Rload=1k\n.model ...\n.subckt ...\n...\n.ends")).changed(){self.changed(&before);}
 ui.add_space(10.);ui.label(RichText::new("Generated netlist").strong());match netlist(&self.sheet){Ok(mut text)=>{egui::ScrollArea::both().max_height((ui.available_height()-10.).max(40.)).show(ui,|ui|{ui.add(egui::TextEdit::multiline(&mut text).code_editor().desired_width(f32::INFINITY).interactive(false));});},Err(e)=>{ui.colored_label(RED,e);}}
 }
 });
    }
    fn bom(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new().inner_margin(20).show(ui,|ui|{
 ui.horizontal(|ui|{ui.heading("Bill of materials");if ui.button("Export CSV…").clicked(){self.request_file(FileAction::Bom);}});
 ui.label(RichText::new("Generic models and footprints do not identify purchasable parts. Sources represent external signal / power connections.").size(12.).color(MUTED));ui.add_space(12.);
 egui::ScrollArea::both().show(ui,|ui|{egui::Grid::new("bom").striped(true).min_col_width(100.).spacing(Vec2::new(18.,12.)).show(ui,|ui|{
 for s in ["Reference","Type","Value / model","Footprint"]{ui.label(RichText::new(s).strong().color(GREEN));}ui.end_row();
 for c in &self.sheet.components{if ui.selectable_label(self.selected==Some(Selection::Component(c.id)),&c.reference).clicked(){self.selected=Some(Selection::Component(c.id));}ui.label(c.kind.name());ui.label(&c.value);ui.label(&c.footprint);ui.end_row();}
 });});
 });
    }
    fn center(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(0))
            .show(ctx, |ui| match self.tab {
                Tab::Schematic => self.schematic(ui),
                Tab::Pcb => self.pcb(ui),
                Tab::Simulation => self.waveforms(ui),
                Tab::Spice => self.spice_workspace(ui),
                Tab::Bom => self.bom(ui),
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(a) = self.file_action {
            egui::Window::new(a.title())
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(if matches!(a, FileAction::Fabrication) {
                        "Destination folder name; files will be created inside."
                    } else {
                        "Absolute or relative path"
                    });
                    ui.add(egui::TextEdit::singleline(&mut self.path_input).desired_width(500.));
                    ui.horizontal(|ui| {
                        if ui.button("Continue").clicked() {
                            let p = PathBuf::from(self.path_input.trim());
                            self.file_action = None;
                            self.file_do(a, &p);
                        }
                        if ui.button("Cancel").clicked() {
                            self.file_action = None;
                        }
                    });
                });
        }
        if self.show_help {
            egui::Window::new("CircuitCraft · quick guide").open(&mut self.show_help).default_width(600.).show(ctx,|ui|{
 ui.heading("Draw → simulate → route");for s in [
 "Choose a part in the library, then click the grid. R rotates; Esc returns to selection.",
 "W starts a wire. Click terminals/grid points to continue; Esc ends it. G places ground, J joins crossings, L places a net label.",
 "Select a part to edit values, source waveform, model, parameters and footprint. Moving terminals requires rerouting connected wires.",
 "Choose operating point, transient, AC, DC sweep or noise and Run (F5). Waveforms provide magnitude, dB, phase, cursors and CSV.",
 "PCB: synchronize footprints, place them, route from pads, use top/bottom layers and plated vias. Routing is manual.",
 "Run DRC and export Gerber/drills. Check the physical component's pad numbering and exported layers before fabrication.",
 "Keys: V select · W wire/route · R rotate · G ground · J junction · L label · Del delete · Ctrl+D duplicate · Ctrl+Z undo · Ctrl+Y redo · Ctrl+S save · Home fit.",
 "SPICE imports are textual; .control scripts are rejected. Generic models are demonstrations.",
 "Version 0.2: custom pad editor, parameter sweeps, noise and signal math. Autorouting, copper zones, hierarchy, PCB 3D, native KiCad/Altium import and digital/MCU co-simulation remain future work."]{ui.label(s);ui.add_space(8.);}
 });
        }
        if self.close_prompt {
            egui::Window::new("Save before closing?")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("The project has unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.request_file(FileAction::Save);
                            if !self.dirty {
                                self.allow_close = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            self.close_prompt = false;
                        }
                        if ui.button("Discard and close").clicked() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.close_prompt = false;
                        }
                    });
                });
        }
    }
}
fn row(ui: &mut egui::Ui, label: &str, s: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.).color(MUTED));
        ui.add(egui::TextEdit::singleline(s).desired_width(140.))
            .changed()
    })
    .inner
}
fn style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
    v.faint_bg_color = Color32::from_rgb(23, 36, 52);
    v.selection.bg_fill = Color32::from_rgb(30, 87, 80);
    v.selection.stroke = Stroke::new(1., GREEN);
    v.widgets.inactive.bg_fill = Color32::from_rgb(27, 42, 58);
    v.widgets.hovered.bg_fill = Color32::from_rgb(41, 63, 82);
    ctx.set_visuals(v);
    let mut s = (*ctx.style()).clone();
    s.spacing.item_spacing = Vec2::new(8., 7.);
    s.spacing.button_padding = Vec2::new(9., 5.);
    s.text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(13.));
    s.text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(12.));
    ctx.set_style(s);
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        self.shortcuts(ctx);
        if self.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        for dropped in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(p) = dropped.path {
                self.file_do(
                    if p.extension().is_some_and(|x| x == "cir" || x == "sp") {
                        FileAction::Import
                    } else {
                        FileAction::Open
                    },
                    &p,
                );
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_prompt = true;
        }
        self.header(ctx);
        self.footer(ctx);
        self.left(ctx);
        self.right(ctx);
        self.center(ctx);
        self.dialogs(ctx);
    }
}
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CircuitCraft · Electronic design studio")
            .with_inner_size([1440., 940.])
            .with_min_inner_size([1100., 700.]),
        ..Default::default()
    };
    eframe::run_native(
        "CircuitCraft",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
#[cfg(test)]
mod tests;
