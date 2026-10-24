use super::*;
pub struct Engineering {
    pub router: RouteOptions,
    job: Option<Receiver<Result<(Sheet, RouteReport), String>>>,
    cancel: Arc<AtomicBool>,
    revision: u64,
    yaw: f64,
    pitch: f64,
    zoom: f64,
    pan: Vec2,
    bodies: bool,
    top: bool,
    bottom: bool,
    texture: Option<egui::TextureHandle>,
}
impl Default for Engineering {
    fn default() -> Self {
        Self {
            router: RouteOptions::default(),
            job: None,
            cancel: Arc::new(AtomicBool::new(false)),
            revision: 0,
            yaw: -0.6,
            pitch: 0.75,
            zoom: 1.,
            pan: Vec2::ZERO,
            bodies: true,
            top: true,
            bottom: true,
            texture: None,
        }
    }
}
impl App {
    #[cfg(test)]
    pub(super) fn router_running(&self) -> bool {
        self.engineering.job.is_some()
    }
    pub(super) fn cancel_router(&self) {
        self.engineering.cancel.store(true, Ordering::Relaxed);
    }
    pub(super) fn project_with(&self, active: &Sheet) -> Sheet {
        let mut out = active.clone();
        for (parent, index) in self.parents.iter().rev() {
            let mut root = parent.clone();
            root.children[*index].sheet = Box::new(out);
            out = root;
        }
        out
    }
    pub(super) fn project(&self) -> Sheet {
        self.project_with(&self.sheet)
    }
    fn reset_page_view(&mut self) {
        self.selected = None;
        self.fit = true;
        self.set_tool(Tool::Select);
    }
    pub(super) fn enter_page(&mut self, index: usize) {
        if let Some(c) = self.sheet.children.get(index) {
            let child = (*c.sheet).clone();
            let parent = std::mem::replace(&mut self.sheet, child);
            self.parents.push((parent, index));
            self.reset_page_view();
        }
    }
    pub(super) fn leave_page(&mut self) {
        if let Some((parent, i)) = self.parents.pop() {
            let child = std::mem::replace(&mut self.sheet, parent);
            self.sheet.children[i].sheet = Box::new(child);
            self.reset_page_view();
        }
    }
    pub(super) fn root_page(&mut self) {
        while !self.parents.is_empty() {
            self.leave_page();
        }
    }
    pub(super) fn undo_project(&mut self) -> bool {
        self.root_page();
        let ok = self.history.undo(&mut self.sheet);
        if ok {
            self.reset_page_view();
        }
        ok
    }
    pub(super) fn redo_project(&mut self) -> bool {
        self.root_page();
        let ok = self.history.redo(&mut self.sheet);
        if ok {
            self.reset_page_view();
        }
        ok
    }
    pub(super) fn add_page(&mut self, mut page: Sheet) {
        if self.parents.len() >= 8 {
            self.say("Maximum 8 nested levels.", true);
            return;
        }
        // Includes are resolved from the project root.
        if page.raw_mode || !page.include_directory.is_empty() {
            self.say(
                "Imported page must use graphical mode and the root include directory.",
                true,
            );
            return;
        }
        page.board = Board::default();
        let mut n = 1;
        while self
            .sheet
            .children
            .iter()
            .any(|c| c.instance.eq_ignore_ascii_case(&format!("H{n}")))
        {
            n += 1;
        }
        let before = self.sheet.clone();
        self.sheet.children.push(ChildSheet {
            instance: format!("H{n}"),
            ports: vec![],
            sheet: Box::new(page),
        });
        if let Err(e) = validate_hierarchy(&self.project()) {
            self.sheet = before;
            self.say(e, true);
            return;
        }
        self.changed(&before);
    }
    pub(super) fn hierarchy_tools(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("HIERARCHICAL PAGES").default_open(true).show(ui, |ui| {
            let path = self.parents.iter().map(|(p,i)|p.children[*i].instance.as_str()).collect::<Vec<_>>().join(" / ");
            ui.colored_label(GREEN, if path.is_empty() { "Root" } else { &path });
            if !self.parents.is_empty() && ui.button("↑ Parent page").clicked() { self.leave_page(); }
            if let Some((p,i)) = self.parents.last() {
                for port in &p.children[*i].ports { ui.label(format!("{} → parent {}",port.label,port.parent_net)); }
            }
            ui.label(RichText::new("Child labels are local. Bind a child label to a labelled parent net; ground is global.").size(10.).color(MUTED));
            ui.horizontal(|ui| {
                if ui.button("New page").clicked() { self.add_page(Sheet::default()); }
                if ui.button("Import…").clicked() { self.request_file(FileAction::ImportSheet); }
            });
            let before = self.sheet.clone();
            let mut changed = false;
            let mut enter = None;
            let mut remove = None;
            for (i,c) in self.sheet.children.iter_mut().enumerate() {
                egui::CollapsingHeader::new(format!("{} · {}",c.instance,c.sheet.title)).id_salt(i).show(ui, |ui| {
                    changed |= row(ui, "Instance", &mut c.instance);
                    ui.horizontal(|ui| {
                        if ui.button("Open page").clicked() { enter = Some(i); }
                        if ui.button("Remove page").clicked() { remove = Some(i); }
                    });
                    let mut delete_port = None;
                    for (j,port) in c.ports.iter_mut().enumerate() {
                        ui.push_id(j, |ui| {
                            changed |= row(ui, "Child label", &mut port.label);
                            changed |= row(ui, "Parent net", &mut port.parent_net);
                            if ui.small_button("Remove binding").clicked() { delete_port = Some(j); }
                        });
                    }
                    if let Some(j)=delete_port { c.ports.remove(j);changed=true; }
                    if ui.button("Add port binding").clicked() { c.ports.push(SheetPort { label:"IN".into(),parent_net:"OUT".into() });changed=true; }
                });
            }
            if let Some(i)=remove { self.sheet.children.remove(i);changed=true;enter=None; }
            if changed { self.changed(&before); }
            if let Some(i)=enter { self.enter_page(i); }
        });
    }
    pub(super) fn start_router(&mut self) {
        if self.engineering.job.is_some() {
            return;
        }
        self.root_page();
        let errors: Vec<_> = self
            .sheet
            .pcb_drc()
            .into_iter()
            .filter(|e| !e.message.starts_with("Unrouted connection"))
            .collect();
        if !errors.is_empty() {
            self.say(
                "Synchronize the PCB and fix DRC errors before autorouting.",
                true,
            );
            return;
        }
        let mut sheet = self.sheet.clone();
        let options = self.engineering.router.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.engineering.cancel = cancel.clone();
        self.engineering.revision = self.revision;
        let (tx, rx) = mpsc::channel();
        self.engineering.job = Some(rx);
        std::thread::spawn(move || {
            let result = sheet
                .autoroute_cancel(&options, &cancel)
                .map(|r| (sheet, r));
            let _ = tx.send(result);
        });
        self.say("Autorouting in background…", false);
    }
    pub(super) fn poll_router(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.engineering.job {
            ctx.request_repaint_after(Duration::from_millis(40));
            match job.try_recv() {
                Ok(result) => {
                    self.engineering.job = None;
                    if self.engineering.revision != self.revision {
                        self.say("Circuit changed; autorouter result discarded.", false);
                        return;
                    }
                    match result {
                        Ok((sheet, r)) => {
                            self.root_page();
                            if r.added_tracks > 0 || r.added_vias > 0 {
                                self.checkpoint();
                                self.sheet = sheet;
                            }
                            self.drc_issues = self.sheet.pcb_drc();
                            self.drc_stale = false;
                            self.say(
                                format!(
                                    "Router: {} connections routed, {} remain · {} tracks, {} vias",
                                    r.routed, r.remaining, r.added_tracks, r.added_vias
                                ),
                                r.remaining > 0,
                            );
                            for b in r.blocked {
                                self.log.push(b);
                            }
                        }
                        Err(e) => self.say(e, true),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.engineering.job = None;
                    self.say("Router worker stopped; original board retained.", true);
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    pub(super) fn router_tools(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label(
            RichText::new("AUTOROUTER · two layers")
                .strong()
                .color(GREEN),
        );
        ui.horizontal(|ui| {
            ui.label("Grid mm");
            ui.add(
                egui::DragValue::new(&mut self.engineering.router.grid)
                    .range(0.1..=2.)
                    .speed(0.1),
            );
        });
        self.engineering.router.width = self.track_width;
        ui.checkbox(&mut self.engineering.router.allow_vias, "Allow plated vias");
        if self.engineering.job.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                if ui.button("Cancel router").clicked() {
                    self.engineering.cancel.store(true, Ordering::Relaxed);
                }
            });
        } else if ui.button("Route unrouted connections").clicked() {
            self.start_router();
        }
        ui.label(RichText::new("Retains existing copper. Undo restores the board. Remaining connections need manual review.").size(10.).color(MUTED));
    }
    pub(super) fn mesh_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading("PCB 3D review");
        ui.checkbox(&mut self.engineering.bodies, "Component envelopes");
        ui.checkbox(&mut self.engineering.top, "Top copper");
        ui.checkbox(&mut self.engineering.bottom, "Bottom copper");
        let before = self.sheet.clone();
        ui.horizontal(|ui| {
            ui.label("Thickness mm");
            if ui
                .add(
                    egui::DragValue::new(&mut self.sheet.board.thickness)
                        .range(0.2..=10.)
                        .speed(0.1),
                )
                .changed()
            {
                self.changed(&before);
            }
        });
        for (name, yaw, pitch) in [
            ("Isometric", -0.6, 0.75),
            ("Top", 0., std::f64::consts::FRAC_PI_2),
            ("Bottom", 0., -std::f64::consts::FRAC_PI_2),
        ] {
            if ui.button(name).clicked() {
                self.engineering.yaw = yaw;
                self.engineering.pitch = pitch;
                self.engineering.zoom = 1.;
                self.engineering.pan = Vec2::ZERO;
            }
        }
        if ui.button("Export 3D OBJ…").clicked() {
            self.request_file(FileAction::BoardOBJ);
        }
        ui.label(RichText::new("Drag to orbit · wheel to zoom · middle drag to pan. Footprint bodies are approximate envelopes; this is a review view, not a mechanical clearance check.").color(MUTED));
    }
    pub(super) fn pcb_3d(&mut self, ui: &mut egui::Ui) {
        let (r, response) = ui.allocate_exact_size(ui.available_size(), Sense::drag());
        self.canvas = r;
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = ui.input(|i| i.pointer.delta());
            self.engineering.yaw += delta.x as f64 * 0.008;
            self.engineering.pitch =
                (self.engineering.pitch - delta.y as f64 * 0.008).clamp(-1.56, 1.56);
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.engineering.pan += ui.input(|i| i.pointer.delta());
        }
        if response.hovered() {
            self.engineering.zoom = (self.engineering.zoom
                * (ui.input(|i| i.smooth_scroll_delta.y) as f64 * 0.003).exp())
            .clamp(0.2, 8.);
        }
        let mesh = match self.sheet.board.mesh(self.engineering.bodies) {
            Ok(m) => m,
            Err(e) => {
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    e,
                    FontId::proportional(16.),
                    RED,
                );
                return;
            }
        };
        let w = self.sheet.board.width;
        let h = self.sheet.board.height;
        let t = self.sheet.board.thickness;
        let (sy, cy) = self.engineering.yaw.sin_cos();
        let (sp, cp) = self.engineering.pitch.sin_cos();
        let scale = (r.width() as f64 / (w.hypot(h) + 5.))
            .min(r.height() as f64 / (w.hypot(h) * sp.abs() + 8. * cp.abs() + 5.))
            * 0.85
            * self.engineering.zoom;
        let project = |p: Vertex3| {
            let (x, y, z) = (p.x - w / 2., p.y - h / 2., p.z - t / 2.);
            let u = x * cy + y * sy;
            let forward = -x * sy + y * cy;
            (
                r.center()
                    + self.engineering.pan
                    + Vec2::new((u * scale) as f32, ((forward * sp - z * cp) * scale) as f32),
                forward * cp + z * sp,
            )
        };
        let mut faces = vec![];
        for o in mesh.objects {
            if (o.material == 1 && !self.engineering.top)
                || (o.material == 2 && !self.engineering.bottom)
            {
                continue;
            }
            for tri in o.triangles {
                let (a, da) = project(tri[0]);
                let (b, db) = project(tri[1]);
                let (c, dc) = project(tri[2]);
                let u = Vertex3::new(
                    tri[1].x - tri[0].x,
                    tri[1].y - tri[0].y,
                    tri[1].z - tri[0].z,
                );
                let v = Vertex3::new(
                    tri[2].x - tri[0].x,
                    tri[2].y - tri[0].y,
                    tri[2].z - tri[0].z,
                );
                let n = Vertex3::new(
                    u.y * v.z - u.z * v.y,
                    u.z * v.x - u.x * v.z,
                    u.x * v.y - u.y * v.x,
                );
                let norm = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt().max(1e-9);
                let shade = 0.5 + 0.5 * ((-0.3 * n.x - 0.4 * n.y + 0.86 * n.z) / norm).abs();
                let rgb = MESH_COLORS[o.material];
                let color = Color32::from_rgb(
                    (rgb[0] as f64 * shade) as u8,
                    (rgb[1] as f64 * shade) as u8,
                    (rgb[2] as f64 * shade) as u8,
                );
                faces.push(([da, db, dc], [a, b, c], color));
            }
        }
        // Depth-buffered orthographic rasterization avoids painter-order errors
        // where a large substrate triangle crosses a small copper/component face.
        let iw = (r.width().ceil() as usize).clamp(1, 1200);
        let ih = (r.height().ceil() as usize).clamp(1, 1000);
        let mut pixels = vec![BG; iw * ih];
        let mut depth = vec![f64::NEG_INFINITY; iw * ih];
        let convert = |p: Pos2| {
            Pos2::new(
                (p.x - r.min.x) * iw as f32 / r.width(),
                (p.y - r.min.y) * ih as f32 / r.height(),
            )
        };
        let edge =
            |a: Pos2, b: Pos2, p: Pos2| (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x);
        for (z, points, color) in faces {
            let [a, b, c] = points.map(convert);
            let area = edge(a, b, c);
            if area.abs() < 1e-7 {
                continue;
            }
            let x0 = a.x.min(b.x).min(c.x).floor().clamp(0., iw as f32) as usize;
            let y0 = a.y.min(b.y).min(c.y).floor().clamp(0., ih as f32) as usize;
            let x1 = a.x.max(b.x).max(c.x).ceil().clamp(0., iw as f32) as usize;
            let y1 = a.y.max(b.y).max(c.y).ceil().clamp(0., ih as f32) as usize;
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let wa = edge(b, c, p) / area;
                    let wb = edge(c, a, p) / area;
                    let wc = 1. - wa - wb;
                    if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                        continue;
                    }
                    let d = wa as f64 * z[0] + wb as f64 * z[1] + wc as f64 * z[2];
                    let i = y * iw + x;
                    if d > depth[i] {
                        depth[i] = d;
                        pixels[i] = color;
                    }
                }
            }
        }
        let bitmap = egui::ColorImage {
            size: [iw, ih],
            pixels,
        };
        if let Some(texture) = &mut self.engineering.texture {
            texture.set(bitmap, egui::TextureOptions::LINEAR);
        } else {
            self.engineering.texture = Some(ui.ctx().load_texture(
                "pcb-3d-depth-view",
                bitmap,
                egui::TextureOptions::LINEAR,
            ));
        }
        let painter = ui.painter_at(r);
        painter.image(
            self.engineering.texture.as_ref().unwrap().id(),
            r,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
        painter.text(
            r.left_top() + Vec2::new(16., 16.),
            Align2::LEFT_TOP,
            format!("{} × {} mm · {:.2} mm substrate", w, h, t),
            FontId::proportional(13.),
            MUTED,
        );
    }
}
