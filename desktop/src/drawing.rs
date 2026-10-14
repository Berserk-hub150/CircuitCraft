use super::*;
fn mm(p: Grid) -> Mm {
    Mm::new(p.x as f64, p.y as f64)
}
fn line(p: &egui::Painter, points: Vec<Pos2>, stroke: Stroke) {
    if points.len() > 1 {
        p.add(egui::Shape::line(points, stroke));
    }
}
fn cross(p: &egui::Painter, at: Pos2, c: Color32) {
    p.line_segment(
        [at - Vec2::new(7., 0.), at + Vec2::new(7., 0.)],
        Stroke::new(1., c),
    );
    p.line_segment(
        [at - Vec2::new(0., 7.), at + Vec2::new(0., 7.)],
        Stroke::new(1., c),
    );
}
fn camera_input(ui: &egui::Ui, response: &egui::Response, r: Rect, c: &mut Camera) -> bool {
    let pan = response.hovered()
        && ui.input(|i| {
            i.pointer.button_down(egui::PointerButton::Middle)
                || (i.key_down(Key::Space) && i.pointer.primary_down())
        });
    if pan {
        c.pan += ui.input(|i| i.pointer.delta());
    }
    if response.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel.abs() > 0.1 {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                let before = c.world(r, pos);
                c.scale = (c.scale * (wheel * 0.003).exp()).clamp(5., 100.);
                c.pan = pos - r.center() - Vec2::new(before.x as f32, before.y as f32) * c.scale;
            }
        }
    }
    pan
}
fn grid(p: &egui::Painter, r: Rect, c: &Camera) {
    let a = c.world(r, r.min);
    let b = c.world(r, r.max);
    let step = if c.scale < 13. { 2 } else { 1 };
    for x in (a.x.floor() as i32..=b.x.ceil() as i32).step_by(step) {
        for y in (a.y.floor() as i32..=b.y.ceil() as i32).step_by(step) {
            p.circle_filled(
                c.screen(r, Mm::new(x as f64, y as f64)),
                0.85,
                Color32::from_rgb(39, 54, 72),
            );
        }
    }
}
fn part(p: &egui::Painter, r: Rect, camera: &Camera, c: &Component, color: Color32, ghost: bool) {
    let s = symbol(c.kind);
    let tr = |v: Mm| {
        let v = v.rotated(c.rotation);
        camera.screen(r, Mm::new(c.at.x as f64 + v.x, c.at.y as f64 + v.y))
    };
    let color = if ghost {
        color.gamma_multiply(0.55)
    } else {
        color
    };
    let stroke = Stroke::new(1.8, color);
    for ps in s.paths {
        line(p, ps.into_iter().map(tr).collect(), stroke);
    }
    for (at, rad) in s.circles {
        p.circle_stroke(tr(at), rad as f32 * camera.scale, stroke);
    }
    for (at, text) in s.labels {
        p.text(
            tr(at),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional((camera.scale * 0.43).clamp(9., 16.)),
            color,
        );
    }
    for (i, (_, at)) in c.pins().iter().enumerate() {
        let pos = camera.screen(r, mm(*at));
        p.circle_filled(pos, 2.3, BLUE);
        if camera.scale >= 40. {
            p.text(
                pos + Vec2::new(5., 5.),
                Align2::LEFT_TOP,
                (i + 1).to_string(),
                FontId::proportional(9.),
                MUTED,
            );
        }
    }
    let center = camera.screen(r, mm(c.at));
    let at = if c.rotation % 2 == 0 && !matches!(c.kind, Kind::Voltage | Kind::Current) {
        center + Vec2::new(-camera.scale * 0.95, -camera.scale * 1.3)
    } else {
        center + Vec2::new(camera.scale * 1.15, -camera.scale * 0.4)
    };
    p.text(
        at,
        Align2::LEFT_BOTTOM,
        &c.reference,
        FontId::proportional(13.),
        color,
    );
    let value = if c.value.chars().count() > 28 {
        format!("{}…", c.value.chars().take(26).collect::<String>())
    } else {
        c.value.clone()
    };
    p.text(
        at + Vec2::new(0., 18.),
        Align2::LEFT_BOTTOM,
        value,
        FontId::monospace(11.),
        MUTED,
    );
}
impl App {
    fn fit_schematic(&mut self, r: Rect) {
        let ps = self
            .sheet
            .components
            .iter()
            .flat_map(|c| c.pins().into_iter().map(|(_, p)| p))
            .chain(
                self.sheet
                    .wires
                    .iter()
                    .flat_map(|w| w.points.iter().copied()),
            )
            .chain(self.sheet.grounds.iter().copied())
            .collect::<Vec<_>>();
        let x0 = ps.iter().map(|p| p.x).min().unwrap_or(-5) as f32 - 3.5;
        let y0 = ps.iter().map(|p| p.y).min().unwrap_or(-5) as f32 - 3.5;
        let x1 = ps.iter().map(|p| p.x).max().unwrap_or(5) as f32 + 3.5;
        let y1 = ps.iter().map(|p| p.y).max().unwrap_or(5) as f32 + 3.5;
        self.camera.scale = (r.width() / (x1 - x0))
            .min(r.height() / (y1 - y0))
            .clamp(8., 45.);
        self.camera.pan = -Vec2::new((x0 + x1) / 2., (y0 + y1) / 2.) * self.camera.scale;
        self.fit = false;
    }
    fn schematic_snap(&self, r: Rect, p: Pos2) -> Grid {
        let w = self.camera.world(r, p);
        if let Some((_, p)) = self
            .sheet
            .components
            .iter()
            .flat_map(|c| c.pins())
            .min_by(|(_, a), (_, b)| w.distance(mm(*a)).total_cmp(&w.distance(mm(*b))))
        {
            if w.distance(mm(p)) < 12. / self.camera.scale as f64 {
                return p;
            }
        }
        Grid::new(w.x.round() as i32, w.y.round() as i32)
    }
    fn schematic_hit(&self, r: Rect, p: Pos2) -> Option<Selection> {
        let w = self.camera.world(r, p);
        for c in self.sheet.components.iter().rev() {
            let radius = if matches!(c.kind, Kind::OpAmp | Kind::Subcircuit) {
                2.5
            } else {
                1.35
            };
            if w.distance(mm(c.at)) < radius {
                return Some(Selection::Component(c.id));
            }
        }
        for l in self.sheet.labels.iter().rev() {
            if w.distance(mm(l.at)) < 0.8 {
                return Some(Selection::Label(l.id));
            }
        }
        for p in &self.sheet.grounds {
            if w.distance(mm(*p)) < 0.6 {
                return Some(Selection::Ground(*p));
            }
        }
        for p in &self.sheet.junctions {
            if w.distance(mm(*p)) < 0.4 {
                return Some(Selection::Junction(*p));
            }
        }
        for wire in self.sheet.wires.iter().rev() {
            if wire
                .points
                .windows(2)
                .any(|ab| point_segment(w, mm(ab[0]), mm(ab[1])) < 7. / self.camera.scale as f64)
            {
                return Some(Selection::Wire(wire.id));
            }
        }
        None
    }
    pub(super) fn schematic(&mut self, ui: &mut egui::Ui) {
        let (r, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.canvas = r;
        if self.fit {
            self.fit_schematic(r);
        }
        let p = ui.painter_at(r);
        p.rect_filled(r, 0., BG);
        let pan = camera_input(ui, &response, r, &mut self.camera);
        grid(&p, r, &self.camera);
        let n = connect(&self.sheet);
        for w in &self.sheet.wires {
            let selected = self.selected == Some(Selection::Wire(w.id));
            let highlight = self
                .probe
                .as_ref()
                .is_some_and(|name| n.wire_net.get(&w.id) == Some(name));
            line(
                &p,
                w.points
                    .iter()
                    .map(|a| self.camera.screen(r, mm(*a)))
                    .collect(),
                Stroke::new(
                    if selected || highlight { 3. } else { 2. },
                    if selected {
                        ORANGE
                    } else if highlight {
                        Color32::WHITE
                    } else {
                        GREEN.gamma_multiply(0.8)
                    },
                ),
            );
        }
        for c in &self.sheet.components {
            part(
                &p,
                r,
                &self.camera,
                c,
                if self.selected == Some(Selection::Component(c.id)) {
                    ORANGE
                } else {
                    Color32::from_rgb(223, 234, 244)
                },
                false,
            );
        }
        for point in n.point_net.keys() {
            let degree = self
                .sheet
                .wires
                .iter()
                .flat_map(|w| w.points.windows(2))
                .filter(|ab| on_segment(*point, ab[0], ab[1]))
                .map(|ab| {
                    if *point == ab[0] || *point == ab[1] {
                        1
                    } else {
                        2
                    }
                })
                .sum::<usize>()
                + self
                    .sheet
                    .components
                    .iter()
                    .flat_map(|c| c.pins())
                    .filter(|(_, p)| p == point)
                    .count();
            if degree >= 3 || self.sheet.junctions.contains(point) {
                p.circle_filled(self.camera.screen(r, mm(*point)), 4., GREEN);
            }
        }
        for g in &self.sheet.grounds {
            let at = self.camera.screen(r, mm(*g));
            let s = self.camera.scale;
            p.line_segment([at, at + Vec2::new(0., 0.5 * s)], Stroke::new(1.8, GREEN));
            for (i, width) in [1.3, 0.9, 0.4].iter().enumerate() {
                let y = at.y + (0.5 + i as f32 * 0.23) * s;
                p.line_segment(
                    [
                        Pos2::new(at.x - width * s / 2., y),
                        Pos2::new(at.x + width * s / 2., y),
                    ],
                    Stroke::new(1.8, GREEN),
                );
            }
        }
        for l in &self.sheet.labels {
            let at = self.camera.screen(r, mm(l.at));
            p.text(
                at + Vec2::new(4., -8.),
                Align2::LEFT_BOTTOM,
                &l.name,
                FontId::monospace(12.),
                BLUE,
            );
            p.line_segment([at, at + Vec2::new(0., -5.)], Stroke::new(1., BLUE));
        }
        p.text(
            r.min + Vec2::new(20., 18.),
            Align2::LEFT_TOP,
            &self.sheet.title,
            FontId::proportional(18.),
            Color32::from_rgb(197, 215, 231),
        );
        p.text(
            r.min + Vec2::new(20., 45.),
            Align2::LEFT_TOP,
            "Electrical connectivity · grid units",
            FontId::proportional(11.),
            MUTED,
        );
        if self.sheet.raw_mode {
            p.text(
                r.min + Vec2::new(20., 68.),
                Align2::LEFT_TOP,
                "Simulation uses the edited SPICE netlist",
                FontId::proportional(12.),
                ORANGE,
            );
        }
        if let Some(net) = &self.probe {
            let voltage = if net == "0" {
                " = 0 V".into()
            } else {
                self.result
                    .as_ref()
                    .and_then(|sim| sim.trace(&format!("v({})", net.to_lowercase())))
                    .map(|t| format!(" = {} V", plot::engineering(*t.real.last().unwrap())))
                    .unwrap_or_default()
            };
            p.text(
                r.max - Vec2::new(20., 38.),
                Align2::RIGHT_BOTTOM,
                format!("Net {net}{voltage}"),
                FontId::monospace(12.),
                GREEN,
            );
        }
        if let Some(pointer) = response.hover_pos() {
            let at = self.schematic_snap(r, pointer);
            let screen = self.camera.screen(r, mm(at));
            match self.tool {
                Tool::Place(kind) => {
                    let c = Component {
                        id: 0,
                        kind,
                        reference: format!("{}?", kind.prefix()),
                        value: kind.default_value().into(),
                        at,
                        rotation: self.rotation,
                        params: String::new(),
                        footprint: kind.default_footprint().into(),
                        custom_pads: vec![],
                    };
                    part(&p, r, &self.camera, &c, GREEN, true);
                }
                Tool::Wire => {
                    cross(&p, screen, GREEN);
                    if let Some(start) = self.wire_start {
                        line(
                            &p,
                            vec![
                                self.camera.screen(r, mm(start)),
                                self.camera.screen(r, Mm::new(at.x as f64, start.y as f64)),
                                screen,
                            ],
                            Stroke::new(2., BLUE),
                        );
                    }
                }
                Tool::Ground | Tool::Label | Tool::Junction => cross(&p, screen, BLUE),
                Tool::Probe => {
                    p.circle_stroke(pointer, 8., Stroke::new(1.5, ORANGE));
                }
                _ => {}
            }
            p.text(
                r.max - Vec2::new(20., 20.),
                Align2::RIGHT_BOTTOM,
                format!(
                    "Grid {}, {} · {:.0}%",
                    at.x,
                    at.y,
                    self.camera.scale / 32. * 100.
                ),
                FontId::monospace(10.),
                MUTED,
            );
            if response.clicked() && !pan {
                match self.tool {
                    Tool::Place(kind) => {
                        self.checkpoint();
                        let id = self.sheet.add(kind, at);
                        self.sheet
                            .components
                            .iter_mut()
                            .find(|c| c.id == id)
                            .unwrap()
                            .rotation = self.rotation;
                        self.selected = Some(Selection::Component(id));
                    }
                    Tool::Wire => {
                        if let Some(start) = self.wire_start {
                            if start != at {
                                self.checkpoint();
                                self.sheet.wire(start, at);
                            }
                        }
                        self.wire_start = Some(at);
                    }
                    Tool::Ground => {
                        self.checkpoint();
                        if self.sheet.grounds.contains(&at) {
                            self.sheet.grounds.retain(|p| *p != at);
                        } else {
                            self.sheet.grounds.push(at);
                        }
                    }
                    Tool::Junction => {
                        self.checkpoint();
                        if self.sheet.junctions.contains(&at) {
                            self.sheet.junctions.retain(|p| *p != at);
                        } else {
                            self.sheet.junctions.push(at);
                        }
                    }
                    Tool::Label => {
                        if valid_node(&self.label_name) {
                            self.checkpoint();
                            self.sheet.label(at, &self.label_name.clone());
                        } else {
                            self.say("Labels require letters, numbers and underscores.", true);
                        }
                    }
                    Tool::Probe => {
                        let name = n.point_net.get(&at).cloned().or_else(|| {
                            let w = self.camera.world(r, pointer);
                            self.sheet
                                .wires
                                .iter()
                                .find(|wire| {
                                    wire.points.windows(2).any(|ab| {
                                        point_segment(w, mm(ab[0]), mm(ab[1]))
                                            < 10. / self.camera.scale as f64
                                    })
                                })
                                .and_then(|wire| n.wire_net.get(&wire.id))
                                .cloned()
                        });
                        if let Some(name) = name {
                            self.probe = Some(name.clone());
                            let trace = format!("v({})", name.to_lowercase());
                            if !self.enabled.contains(&trace) {
                                self.enabled.push(trace);
                            }
                            self.say(format!("Probe: {name}"), false);
                        }
                    }
                    _ => self.selected = self.schematic_hit(r, pointer),
                }
            }
            if self.tool == Tool::Select && !pan {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    self.selected = self.schematic_hit(r, pointer);
                    let pos = match self.selected {
                        Some(Selection::Component(id)) => self
                            .sheet
                            .components
                            .iter()
                            .find(|c| c.id == id)
                            .map(|c| c.at),
                        Some(Selection::Label(id)) => {
                            self.sheet.labels.iter().find(|l| l.id == id).map(|l| l.at)
                        }
                        _ => None,
                    };
                    if let Some(pos) = pos {
                        self.drag_offset = Some(Grid::new(pos.x - at.x, pos.y - at.y));
                        self.checkpoint();
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(off) = self.drag_offset {
                        match self.selected {
                            Some(Selection::Component(id)) => {
                                if let Some(c) =
                                    self.sheet.components.iter_mut().find(|c| c.id == id)
                                {
                                    c.at = at.plus(off);
                                }
                            }
                            Some(Selection::Label(id)) => {
                                if let Some(l) = self.sheet.labels.iter_mut().find(|l| l.id == id) {
                                    l.at = at.plus(off);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.drag_offset = None;
        }
    }
    fn b_screen(&self, r: Rect, p: Mm) -> Pos2 {
        self.pcb_camera.screen(r, Mm::new(p.x, -p.y))
    }
    fn b_world(&self, r: Rect, p: Pos2) -> Mm {
        let p = self.pcb_camera.world(r, p);
        Mm::new(p.x, -p.y)
    }
    fn b_snap(&self, r: Rect, p: Pos2) -> Mm {
        let w = self.b_world(r, p);
        let pads = self.sheet.board.pads();
        if let Some(p) = pads
            .iter()
            .min_by(|a, b| a.at.distance(w).total_cmp(&b.at.distance(w)))
        {
            if p.at.distance(w) < 10. / self.pcb_camera.scale as f64 {
                return p.at;
            }
        }
        if let Some(v) = self
            .sheet
            .board
            .vias
            .iter()
            .min_by(|a, b| a.at.distance(w).total_cmp(&b.at.distance(w)))
        {
            if v.at.distance(w) < 10. / self.pcb_camera.scale as f64 {
                return v.at;
            }
        }
        for t in &self.sheet.board.tracks {
            for ab in t.points.windows(2) {
                if point_segment(w, ab[0], ab[1]) < 6. / self.pcb_camera.scale as f64 {
                    if (ab[0].x - ab[1].x).abs() < 1e-6 {
                        return Mm::new(
                            ab[0].x,
                            ((w.y / self.snap_mm).round() * self.snap_mm)
                                .clamp(ab[0].y.min(ab[1].y), ab[0].y.max(ab[1].y)),
                        );
                    }
                    if (ab[0].y - ab[1].y).abs() < 1e-6 {
                        return Mm::new(
                            ((w.x / self.snap_mm).round() * self.snap_mm)
                                .clamp(ab[0].x.min(ab[1].x), ab[0].x.max(ab[1].x)),
                            ab[0].y,
                        );
                    }
                }
            }
        }
        Mm::new(
            (w.x / self.snap_mm).round() * self.snap_mm,
            (w.y / self.snap_mm).round() * self.snap_mm,
        )
    }
    fn b_hit(&self, w: Mm) -> Option<Selection> {
        for f in self.sheet.board.footprints.iter().rev() {
            if f.at.distance(w) < 5. {
                return Some(Selection::Footprint(f.component_id));
            }
        }
        for v in self.sheet.board.vias.iter().rev() {
            if v.at.distance(w) < 0.7 {
                return Some(Selection::Via(v.id));
            }
        }
        for t in self.sheet.board.tracks.iter().rev() {
            if t.points.windows(2).any(|ab| {
                point_segment(w, ab[0], ab[1])
                    < (t.width / 2.).max(6. / self.pcb_camera.scale as f64)
            }) {
                return Some(Selection::Track(t.id));
            }
        }
        None
    }
    pub(super) fn pcb(&mut self, ui: &mut egui::Ui) {
        let (r, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.canvas = r;
        if self.fit {
            let b = &self.sheet.board;
            self.pcb_camera.scale = ((r.width() - 80.) / b.width as f32)
                .min((r.height() - 110.) / b.height as f32)
                .clamp(2., 30.);
            self.pcb_camera.pan =
                Vec2::new(-b.width as f32 / 2., b.height as f32 / 2.) * self.pcb_camera.scale;
            self.fit = false;
        }
        let p = ui.painter_at(r);
        p.rect_filled(r, 0., BG);
        let pan = camera_input(ui, &response, r, &mut self.pcb_camera);
        let board = Rect::from_two_pos(
            self.b_screen(r, Mm::new(0., 0.)),
            self.b_screen(r, Mm::new(self.sheet.board.width, self.sheet.board.height)),
        );
        p.rect_filled(board, 3., Color32::from_rgb(17, 49, 47));
        p.rect_stroke(
            board,
            3.,
            Stroke::new(1.5, GREEN.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
        let step = if self.pcb_camera.scale < 5. { 5 } else { 1 };
        for x in (0..=self.sheet.board.width as i32).step_by(step) {
            for y in (0..=self.sheet.board.height as i32).step_by(step) {
                p.circle_filled(
                    self.b_screen(r, Mm::new(x as f64, y as f64)),
                    0.6,
                    Color32::from_rgb(33, 72, 64),
                );
            }
        }
        if self.show_ratsnest {
            for (_, a, b) in self.sheet.board.ratsnest() {
                p.line_segment(
                    [self.b_screen(r, a), self.b_screen(r, b)],
                    Stroke::new(1., Color32::from_rgba_unmultiplied(177, 198, 141, 100)),
                );
            }
        }
        for t in &self.sheet.board.tracks {
            let selected = self.selected == Some(Selection::Track(t.id));
            let color = if selected {
                Color32::WHITE
            } else if t.layer == Copper::Top {
                Color32::from_rgb(247, 122, 98)
            } else {
                BLUE
            };
            line(
                &p,
                t.points.iter().map(|w| self.b_screen(r, *w)).collect(),
                Stroke::new(
                    (t.width as f32 * self.pcb_camera.scale).max(1.8),
                    color.gamma_multiply(if t.layer == self.layer || selected {
                        1.
                    } else {
                        0.45
                    }),
                ),
            );
        }
        for f in &self.sheet.board.footprints {
            let selected = self.selected == Some(Selection::Footprint(f.component_id));
            let center = self.b_screen(r, f.at);
            for pad in f.pads() {
                let at = self.b_screen(r, pad.at);
                let color = if self.probe.as_ref() == Some(&pad.net) {
                    Color32::WHITE
                } else {
                    ORANGE
                };
                if pad.rectangular {
                    p.rect_filled(
                        Rect::from_center_size(
                            at,
                            Vec2::new(pad.size.x as f32, pad.size.y as f32) * self.pcb_camera.scale,
                        ),
                        1.,
                        color,
                    );
                } else {
                    p.circle_filled(at, pad.size.x as f32 / 2. * self.pcb_camera.scale, color);
                }
                if pad.drill > 0. {
                    p.circle_filled(at, pad.drill as f32 / 2. * self.pcb_camera.scale, BG);
                }
                if self.pcb_camera.scale >= 6. {
                    p.text(
                        at + Vec2::new(0., 13.),
                        Align2::CENTER_TOP,
                        pad.number.to_string(),
                        FontId::monospace(9.),
                        MUTED,
                    );
                }
            }
            p.text(
                center + Vec2::new(0., -22.),
                Align2::CENTER_BOTTOM,
                &f.reference,
                FontId::proportional(12.),
                if selected { Color32::WHITE } else { GREEN },
            );
            if selected {
                p.rect_stroke(
                    Rect::from_center_size(center, Vec2::new(10., 6.) * self.pcb_camera.scale),
                    2.,
                    Stroke::new(1., Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
        }
        for v in &self.sheet.board.vias {
            let at = self.b_screen(r, v.at);
            p.circle_filled(at, v.diameter as f32 / 2. * self.pcb_camera.scale, GREEN);
            p.circle_filled(at, v.drill as f32 / 2. * self.pcb_camera.scale, BG);
        }
        if !self.drc_stale {
            for issue in &self.drc_issues {
                if let Some(at) = issue.at {
                    p.circle_stroke(self.b_screen(r, at), 10., Stroke::new(1.5, RED));
                }
            }
        }
        p.text(
            r.min + Vec2::new(20., 18.),
            Align2::LEFT_TOP,
            "PCB layout",
            FontId::proportional(18.),
            Color32::from_rgb(197, 215, 231),
        );
        p.text(
            r.min + Vec2::new(20., 45.),
            Align2::LEFT_TOP,
            format!(
                "{:.1} × {:.1} mm · 2 layers · manual routing",
                self.sheet.board.width, self.sheet.board.height
            ),
            FontId::proportional(11.),
            MUTED,
        );
        p.text(
            self.b_screen(r, Mm::new(0., 0.)) + Vec2::new(4., 8.),
            Align2::LEFT_TOP,
            "(0, 0) mm · +X → +Y ↑",
            FontId::monospace(9.),
            MUTED,
        );
        if let Some(pointer) = response.hover_pos() {
            let at = self.b_snap(r, pointer);
            let screen = self.b_screen(r, at);
            if matches!(self.tool, Tool::Route | Tool::Via) {
                cross(&p, screen, BLUE);
            }
            if self.tool == Tool::Route {
                if let Some(start) = self.route_start {
                    line(
                        &p,
                        vec![
                            self.b_screen(r, start),
                            self.b_screen(r, Mm::new(at.x, start.y)),
                            screen,
                        ],
                        Stroke::new(
                            (self.track_width as f32 * self.pcb_camera.scale).max(2.),
                            if self.layer == Copper::Top {
                                ORANGE
                            } else {
                                BLUE
                            },
                        ),
                    );
                }
            }
            p.text(
                r.max - Vec2::new(20., 20.),
                Align2::RIGHT_BOTTOM,
                format!("{:.3}, {:.3} mm · {:?}", at.x, at.y, self.layer),
                FontId::monospace(10.),
                MUTED,
            );
            if response.clicked() && !pan {
                match self.tool {
                    Tool::Route => {
                        let pad = self
                            .sheet
                            .board
                            .pads()
                            .into_iter()
                            .find(|p| p.at.distance(at) < 0.001);
                        if let Some(start) = self.route_start {
                            if pad
                                .as_ref()
                                .is_some_and(|p| !p.net.eq_ignore_ascii_case(&self.route_net))
                            {
                                self.say(
                                    "That pad belongs to another net; choose a matching endpoint.",
                                    true,
                                );
                            } else {
                                self.checkpoint();
                                let id = self.sheet.id();
                                self.sheet.board.add_track(
                                    id,
                                    self.route_net.clone(),
                                    self.layer,
                                    self.track_width,
                                    start,
                                    at,
                                );
                                self.route_start = if pad.is_some() { None } else { Some(at) };
                            }
                        } else {
                            if let Some(pad) = pad {
                                self.route_net = pad.net;
                                if pad.drill == 0. {
                                    self.layer = Copper::Top;
                                }
                            } else if let Some(v) = self
                                .sheet
                                .board
                                .vias
                                .iter()
                                .find(|v| v.at.distance(at) < 0.001)
                            {
                                self.route_net = v.net.clone();
                            } else if let Some(t) = self.sheet.board.tracks.iter().find(|t| {
                                t.points
                                    .windows(2)
                                    .any(|ab| point_segment(at, ab[0], ab[1]) < 1e-5)
                            }) {
                                self.route_net = t.net.clone();
                            }
                            self.route_start = Some(at);
                        }
                    }
                    Tool::Via => {
                        self.checkpoint();
                        let id = self.sheet.id();
                        self.sheet.board.vias.push(Via {
                            id,
                            net: self.route_net.clone(),
                            at,
                            diameter: 0.8,
                            drill: 0.4,
                        });
                        self.say(
                            "Via placed. Choose another layer, then route from it.",
                            false,
                        );
                    }
                    _ => self.selected = self.b_hit(self.b_world(r, pointer)),
                }
            }
            if self.tool == Tool::Select && !pan {
                if response.drag_started_by(egui::PointerButton::Primary) {
                    let w = self.b_world(r, pointer);
                    self.selected = self.b_hit(w);
                    if let Some(Selection::Footprint(id)) = self.selected {
                        if let Some(f) = self
                            .sheet
                            .board
                            .footprints
                            .iter()
                            .find(|f| f.component_id == id)
                        {
                            self.pcb_drag_offset = Some(Mm::new(f.at.x - w.x, f.at.y - w.y));
                            self.checkpoint();
                        }
                    }
                }
                if response.dragged_by(egui::PointerButton::Primary) {
                    if let (Some(Selection::Footprint(id)), Some(off)) =
                        (self.selected, self.pcb_drag_offset)
                    {
                        let w = self.b_world(r, pointer);
                        if let Some(f) = self
                            .sheet
                            .board
                            .footprints
                            .iter_mut()
                            .find(|f| f.component_id == id)
                        {
                            f.at = Mm::new(
                                ((w.x + off.x) / self.snap_mm).round() * self.snap_mm,
                                ((w.y + off.y) / self.snap_mm).round() * self.snap_mm,
                            );
                        }
                    }
                }
            }
        }
        if response.drag_stopped() {
            self.pcb_drag_offset = None;
        }
    }
}
