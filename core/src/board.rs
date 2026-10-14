use crate::{connect, valid_node, Sheet};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod router;
pub use router::*;
fn default_thickness() -> f64 {
    1.6
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Mm {
    pub x: f64,
    pub y: f64,
}
impl Mm {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, p: Self) -> f64 {
        (self.x - p.x).hypot(self.y - p.y)
    }
    pub fn rotated(self, r: u8) -> Self {
        match r % 4 {
            0 => self,
            1 => Self::new(-self.y, self.x),
            2 => Self::new(-self.x, -self.y),
            _ => Self::new(self.y, -self.x),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Copper {
    Top,
    Bottom,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Footprint {
    pub component_id: u64,
    pub reference: String,
    pub style: String,
    pub at: Mm,
    pub rotation: u8,
    pub nets: Vec<String>,
    #[serde(default)]
    pub custom_pads: Vec<PadSpec>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PadSpec {
    pub at: Mm,
    pub size: Mm,
    pub rectangular: bool,
    /// Zero denotes a surface-mount pad on the top layer.
    pub drill: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FootprintDefinition {
    pub name: String,
    pub pads: Vec<PadSpec>,
}
impl FootprintDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.name.contains(['\n', '\r'])
            || self.pads.is_empty()
            || self.pads.len() > 32
        {
            return Err("Footprint needs a name and 1–32 numbered pads.".into());
        }
        for p in &self.pads {
            if ![p.at.x, p.at.y, p.size.x, p.size.y, p.drill]
                .iter()
                .all(|v| v.is_finite())
                || p.at.x.abs() > 1000.
                || p.at.y.abs() > 1000.
                || p.size.x <= 0.
                || p.size.y <= 0.
                || p.size.x > 100.
                || p.size.y > 100.
                || p.drill < 0.
                || p.drill >= p.size.x.min(p.size.y)
                || (!p.rectangular && (p.size.x - p.size.y).abs() > 1e-6)
            {
                return Err("Invalid pad: positive size, drill smaller than copper, circular pads need equal X/Y size.".into());
            }
        }
        Ok(())
    }
    pub fn from_json(s: &str) -> Result<Self, String> {
        let f: Self = serde_json::from_str(s).map_err(|e| e.to_string())?;
        f.validate()?;
        Ok(f)
    }
    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub net: String,
    pub layer: Copper,
    pub width: f64,
    pub points: Vec<Mm>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Via {
    pub id: u64,
    pub net: String,
    pub at: Mm,
    pub diameter: f64,
    pub drill: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub width: f64,
    pub height: f64,
    #[serde(default = "default_thickness")]
    pub thickness: f64,
    pub clearance: f64,
    pub edge_clearance: f64,
    pub min_track: f64,
    pub footprints: Vec<Footprint>,
    pub tracks: Vec<Track>,
    pub vias: Vec<Via>,
}
impl Default for Board {
    fn default() -> Self {
        Self {
            width: 80.,
            height: 60.,
            thickness: default_thickness(),
            clearance: 0.25,
            edge_clearance: 0.5,
            min_track: 0.2,
            footprints: vec![],
            tracks: vec![],
            vias: vec![],
        }
    }
}
pub const FOOTPRINTS: [&str; 7] = [
    "THT-2-5.08",
    "THT-2-7.62",
    "THT-2-10.16",
    "SMD-0603",
    "SMD-0805",
    "TO-92",
    "DIP-4-7.62",
];
#[derive(Debug, Clone)]
pub struct Pad {
    pub component_id: u64,
    pub reference: String,
    pub number: usize,
    pub net: String,
    pub at: Mm,
    pub size: Mm,
    pub rectangular: bool,
    pub drill: f64,
}
impl Footprint {
    pub fn pads(&self) -> Vec<Pad> {
        if !self.custom_pads.is_empty() {
            return self
                .custom_pads
                .iter()
                .enumerate()
                .take(self.nets.len())
                .map(|(i, p)| {
                    let at = p.at.rotated(self.rotation);
                    Pad {
                        component_id: self.component_id,
                        reference: self.reference.clone(),
                        number: i + 1,
                        net: self.nets[i].clone(),
                        at: Mm::new(self.at.x + at.x, self.at.y + at.y),
                        size: if self.rotation % 2 == 0 {
                            p.size
                        } else {
                            Mm::new(p.size.y, p.size.x)
                        },
                        rectangular: p.rectangular,
                        drill: p.drill,
                    }
                })
                .collect();
        }
        let (pins, size, rectangular, drill) = match self.style.as_str() {
            "SMD-0603" => (
                vec![Mm::new(-0.8, 0.), Mm::new(0.8, 0.)],
                Mm::new(0.9, 0.8),
                true,
                0.,
            ),
            "SMD-0805" => (
                vec![Mm::new(-1., 0.), Mm::new(1., 0.)],
                Mm::new(1.1, 1.4),
                true,
                0.,
            ),
            "TO-92" => (
                vec![Mm::new(-1.27, 0.), Mm::new(0., 0.), Mm::new(1.27, 0.)],
                Mm::new(1.05, 1.05),
                false,
                0.55,
            ),
            "DIP-4-7.62" => (
                vec![
                    Mm::new(-3.81, -1.27),
                    Mm::new(-3.81, 1.27),
                    Mm::new(3.81, 1.27),
                    Mm::new(3.81, -1.27),
                ],
                Mm::new(1.7, 1.7),
                false,
                0.8,
            ),
            "THT-2-7.62" => (
                vec![Mm::new(-3.81, 0.), Mm::new(3.81, 0.)],
                Mm::new(1.8, 1.8),
                false,
                0.8,
            ),
            "THT-2-10.16" => (
                vec![Mm::new(-5.08, 0.), Mm::new(5.08, 0.)],
                Mm::new(1.8, 1.8),
                false,
                0.8,
            ),
            _ => (
                vec![Mm::new(-2.54, 0.), Mm::new(2.54, 0.)],
                Mm::new(1.8, 1.8),
                false,
                0.8,
            ),
        };
        pins.into_iter()
            .enumerate()
            .take(self.nets.len())
            .map(|(i, p)| {
                let p = p.rotated(self.rotation);
                Pad {
                    component_id: self.component_id,
                    reference: self.reference.clone(),
                    number: i + 1,
                    net: self.nets[i].clone(),
                    at: Mm::new(self.at.x + p.x, self.at.y + p.y),
                    size: if self.rotation % 2 == 0 {
                        size
                    } else {
                        Mm::new(size.y, size.x)
                    },
                    rectangular,
                    drill,
                }
            })
            .collect()
    }
}
#[derive(Debug, Clone)]
pub struct PcbIssue {
    pub error: bool,
    pub message: String,
    pub at: Option<Mm>,
}
impl Board {
    pub fn validate(&self) -> Result<(), String> {
        let pos =
            |p: Mm| p.x.is_finite() && p.y.is_finite() && p.x.abs() < 10000. && p.y.abs() < 10000.;
        if !self.width.is_finite()
            || !self.thickness.is_finite()
            || self.thickness < 0.2
            || self.thickness > 10.
            || !self.height.is_finite()
            || self.width < 5.
            || self.height < 5.
            || self.width > 2000.
            || self.height > 2000.
            || !self.clearance.is_finite()
            || self.clearance < 0.
            || !self.edge_clearance.is_finite()
            || self.edge_clearance < 0.
            || !self.min_track.is_finite()
            || self.min_track <= 0.
        {
            return Err("Invalid board dimensions or rules.".into());
        }
        if self.footprints.len() > 10000 || self.tracks.len() > 10000 || self.vias.len() > 10000 {
            return Err("PCB exceeds 10,000 objects per type.".into());
        }
        for f in &self.footprints {
            if !pos(f.at)
                || (f.custom_pads.is_empty() && !FOOTPRINTS.contains(&f.style.as_str()))
                || f.nets.len() > 32
            {
                return Err("Invalid footprint.".into());
            }
            if !f.custom_pads.is_empty() {
                FootprintDefinition {
                    name: f.style.clone(),
                    pads: f.custom_pads.clone(),
                }
                .validate()?;
            }
        }
        for t in &self.tracks {
            if !valid_node(&t.net)
                || t.points.len() < 2
                || t.points.len() > 1000
                || t.points.iter().any(|p| !pos(*p))
                || !t.width.is_finite()
                || t.width <= 0.
                || t.width > 50.
            {
                return Err("Invalid track geometry or net name.".into());
            }
        }
        for v in &self.vias {
            if !valid_node(&v.net)
                || !pos(v.at)
                || !v.diameter.is_finite()
                || !v.drill.is_finite()
                || v.drill <= 0.
                || v.diameter <= v.drill
                || v.diameter > 50.
            {
                return Err("Invalid via.".into());
            }
        }
        Ok(())
    }
    pub fn sync(&mut self, s: &Sheet) {
        let n = connect(s);
        self.footprints = s
            .components
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let old = self.footprints.iter().find(|f| f.component_id == c.id);
                Footprint {
                    component_id: c.id,
                    reference: c.reference.clone(),
                    style: c.footprint.clone(),
                    custom_pads: c.custom_pads.clone(),
                    at: old.map(|f| f.at).unwrap_or(Mm::new(
                        15. + (i % 5) as f64 * 12.,
                        15. + (i / 5) as f64 * 12.,
                    )),
                    rotation: old.map(|f| f.rotation).unwrap_or(0),
                    nets: (0..c.pins().len())
                        .map(|p| n.pin_net[&(c.id, p)].clone())
                        .collect(),
                }
            })
            .collect();
    }
    pub fn pads(&self) -> Vec<Pad> {
        self.footprints.iter().flat_map(|f| f.pads()).collect()
    }
    pub fn add_track(&mut self, id: u64, net: String, layer: Copper, width: f64, a: Mm, b: Mm) {
        if a.distance(b) < 1e-6 {
            return;
        }
        let mut ps = vec![a];
        if (a.x - b.x).abs() > 1e-6 && (a.y - b.y).abs() > 1e-6 {
            ps.push(Mm::new(b.x, a.y));
        }
        ps.push(b);
        self.tracks.push(Track {
            id,
            net,
            layer,
            width,
            points: ps,
        });
    }
    fn copper(&self) -> Vec<Shape> {
        let mut out = vec![];
        for p in self.pads() {
            out.push(Shape {
                net: p.net,
                layer: if p.drill > 0. {
                    None
                } else {
                    Some(Copper::Top)
                },
                object: format!("pad-{}-{}", p.component_id, p.number),
                label: format!("{} pad {}", p.reference, p.number),
                geo: if p.rectangular {
                    Geo::Rect(p.at, p.size.x / 2., p.size.y / 2.)
                } else {
                    Geo::Disk(p.at, p.size.x / 2.)
                },
            });
        }
        for t in &self.tracks {
            for ab in t.points.windows(2) {
                out.push(Shape {
                    net: t.net.clone(),
                    layer: Some(t.layer),
                    object: format!("track-{}", t.id),
                    label: format!("track {}", t.id),
                    geo: Geo::Line(ab[0], ab[1], t.width / 2.),
                });
            }
        }
        for v in &self.vias {
            out.push(Shape {
                net: v.net.clone(),
                layer: None,
                object: format!("via-{}", v.id),
                label: format!("via {}", v.id),
                geo: Geo::Disk(v.at, v.diameter / 2.),
            });
        }
        out
    }
    pub fn ratsnest(&self) -> Vec<(String, Mm, Mm)> {
        let shapes = self.copper();
        let pads = self.pads();
        if shapes.len() > 3000 {
            return vec![];
        }
        let mut parent = (0..shapes.len()).collect::<Vec<_>>();
        fn root(p: &mut [usize], i: usize) -> usize {
            if p[i] != i {
                p[i] = root(p, p[i]);
            }
            p[i]
        }
        for i in 0..shapes.len() {
            for j in 0..i {
                if shapes[i].net.eq_ignore_ascii_case(&shapes[j].net)
                    && layers(shapes[i].layer, shapes[j].layer)
                    && distance(&shapes[i].geo, &shapes[j].geo) < 1e-6
                {
                    let a = root(&mut parent, i);
                    let b = root(&mut parent, j);
                    parent[b] = a;
                }
            }
        }
        let mut groups = BTreeMap::<String, Vec<(usize, Mm)>>::new();
        for (i, p) in pads.iter().enumerate() {
            if !p.net.is_empty() {
                groups
                    .entry(p.net.to_lowercase())
                    .or_default()
                    .push((root(&mut parent, i), p.at));
            }
        }
        let mut out = vec![];
        for (net, ps) in groups {
            let mut done = vec![ps[0]];
            let mut remaining = ps[1..].to_vec();
            while !remaining.is_empty() {
                remaining.retain(|p| !done.iter().any(|q| q.0 == p.0));
                if remaining.is_empty() {
                    break;
                }
                let mut best = (f64::INFINITY, 0, 0);
                for (i, a) in done.iter().enumerate() {
                    for (j, b) in remaining.iter().enumerate() {
                        let d = a.1.distance(b.1);
                        if d < best.0 {
                            best = (d, i, j);
                        }
                    }
                }
                let b = remaining.remove(best.2);
                out.push((net.clone(), done[best.1].1, b.1));
                done.push(b);
            }
        }
        out
    }
    pub fn drc(&self) -> Vec<PcbIssue> {
        let issue = |message: String, at: Option<Mm>| PcbIssue {
            error: true,
            message,
            at,
        };
        if let Err(e) = self.validate() {
            return vec![issue(e, None)];
        }
        let mut out = vec![];
        let shapes = self.copper();
        for t in &self.tracks {
            if t.width < self.min_track - 1e-6 {
                out.push(issue(
                    format!(
                        "Track {} is below minimum width {:.3} mm.",
                        t.id, self.min_track
                    ),
                    t.points.first().copied(),
                ));
            }
        }
        for f in &self.footprints {
            let max = if !f.custom_pads.is_empty() {
                f.custom_pads.len()
            } else if f.style == "TO-92" {
                3
            } else if f.style == "DIP-4-7.62" {
                4
            } else {
                2
            };
            if f.nets.len() > max {
                out.push(issue(
                    format!(
                        "{} footprint has {max} pads for {} terminals.",
                        f.reference,
                        f.nets.len()
                    ),
                    Some(f.at),
                ));
            }
        }
        for s in &shapes {
            let (x0, y0, x1, y1) = bounds(&s.geo);
            if x0 < self.edge_clearance
                || y0 < self.edge_clearance
                || x1 > self.width - self.edge_clearance
                || y1 > self.height - self.edge_clearance
            {
                out.push(issue(
                    format!("{} violates board-edge clearance.", s.label),
                    Some(center(&s.geo)),
                ));
            }
        }
        if shapes.len() > 3000 {
            out.push(issue(
                "DRC limit: 3,000 copper shapes. A clean result cannot be reported.".into(),
                None,
            ));
            return out;
        }
        for i in 0..shapes.len() {
            for j in 0..i {
                let a = &shapes[i];
                let b = &shapes[j];
                if a.net.eq_ignore_ascii_case(&b.net)
                    || !layers(a.layer, b.layer)
                    || a.object == b.object
                {
                    continue;
                }
                let ba = bounds(&a.geo);
                let bb = bounds(&b.geo);
                if ba.0 > bb.2 + self.clearance
                    || bb.0 > ba.2 + self.clearance
                    || ba.1 > bb.3 + self.clearance
                    || bb.1 > ba.3 + self.clearance
                {
                    continue;
                }
                let d = distance(&a.geo, &b.geo);
                if d < self.clearance - 1e-6 {
                    out.push(issue(
                        format!(
                            "{} / {}: {} ({:.3} mm; rule {:.3} mm).",
                            a.label,
                            b.label,
                            if d <= 0. { "copper short" } else { "clearance" },
                            d.max(0.),
                            self.clearance
                        ),
                        Some(center(&a.geo)),
                    ));
                    if out.len() > 300 {
                        return out;
                    }
                }
            }
        }
        for (net, a, _) in self.ratsnest() {
            out.push(issue(format!("Unrouted connection on {net}."), Some(a)));
        }
        out
    }
    pub fn gerbers(&self) -> Result<Vec<(String, String)>, String> {
        self.validate()?;
        let mut files = vec![];
        let mut paste = Gerber::new();
        for p in self.pads().into_iter().filter(|p| p.drill == 0.) {
            let ap = if p.rectangular {
                format!("R,{:.6}X{:.6}", p.size.x, p.size.y)
            } else {
                format!("C,{:.6}", p.size.x)
            };
            paste.flash(&ap, p.at);
        }
        files.push(("board-F_Paste.gbr".into(), paste.finish()));
        for (filename, layer, mask) in [
            ("board-F_Cu.gbr", Copper::Top, false),
            ("board-B_Cu.gbr", Copper::Bottom, false),
            ("board-F_Mask.gbr", Copper::Top, true),
            ("board-B_Mask.gbr", Copper::Bottom, true),
        ] {
            let mut g = Gerber::new();
            for p in self.pads() {
                if p.drill == 0. && layer == Copper::Bottom {
                    continue;
                }
                let grow = if mask { 0.2 } else { 0. };
                let ap = if p.rectangular {
                    format!("R,{:.6}X{:.6}", p.size.x + grow, p.size.y + grow)
                } else {
                    format!("C,{:.6}", p.size.x + grow)
                };
                g.flash(&ap, p.at);
            }
            for v in &self.vias {
                g.flash(
                    &format!("C,{:.6}", v.diameter + if mask { 0.2 } else { 0. }),
                    v.at,
                );
            }
            if !mask {
                for t in self.tracks.iter().filter(|t| t.layer == layer) {
                    g.polyline(&format!("C,{:.6}", t.width), &t.points);
                }
            }
            files.push((filename.into(), g.finish()));
        }
        let mut g = Gerber::new();
        g.polyline(
            "C,0.100000",
            &[
                Mm::new(0., 0.),
                Mm::new(self.width, 0.),
                Mm::new(self.width, self.height),
                Mm::new(0., self.height),
                Mm::new(0., 0.),
            ],
        );
        files.push(("board-Edge_Cuts.gbr".into(), g.finish()));
        let mut g = Gerber::new();
        for f in &self.footprints {
            let pads = f.pads();
            if pads.is_empty() {
                continue;
            }
            let x0 = pads.iter().map(|p| p.at.x).fold(f64::INFINITY, f64::min) - 1.3;
            let x1 = pads
                .iter()
                .map(|p| p.at.x)
                .fold(f64::NEG_INFINITY, f64::max)
                + 1.3;
            let y0 = pads.iter().map(|p| p.at.y).fold(f64::INFINITY, f64::min) - 1.3;
            let y1 = pads
                .iter()
                .map(|p| p.at.y)
                .fold(f64::NEG_INFINITY, f64::max)
                + 1.3;
            g.polyline(
                "C,0.150000",
                &[
                    Mm::new(x0, y0),
                    Mm::new(x1, y0),
                    Mm::new(x1, y1),
                    Mm::new(x0, y1),
                    Mm::new(x0, y0),
                ],
            );
        }
        files.push(("board-F_SilkS.gbr".into(), g.finish()));
        let mut holes = BTreeMap::<i64, Vec<Mm>>::new();
        for p in self.pads().into_iter().filter(|p| p.drill > 0.) {
            holes
                .entry((p.drill * 1e6).round() as i64)
                .or_default()
                .push(p.at);
        }
        for v in &self.vias {
            holes
                .entry((v.drill * 1e6).round() as i64)
                .or_default()
                .push(v.at);
        }
        let mut drill = "M48\n; CircuitCraft plated drill holes\nMETRIC,TZ\n".to_string();
        for (i, (diam, _)) in holes.iter().enumerate() {
            drill.push_str(&format!("T{:02}C{:.6}\n", i + 1, *diam as f64 / 1e6));
        }
        drill.push_str("G90\n%\n");
        for (i, (_, ps)) in holes.iter().enumerate() {
            drill.push_str(&format!("T{:02}\n", i + 1));
            for p in ps {
                drill.push_str(&format!("X{:.6}Y{:.6}\n", p.x, p.y));
            }
        }
        drill.push_str("M30\n");
        files.push(("board-PTH.drl".into(), drill));
        files.push(("FABRICATION.txt".into(),format!("CircuitCraft: {:.3} x {:.3} mm, 2 copper layers.\nCommon origin, metric coordinates. Drill holes are plated.\nMask expansion 0.1 mm. Top solder paste uses full SMD pad dimensions; adjust for your process. No assembly text.\nGeometric DRC issues: {}.\nCheck physical footprint pin mapping, datasheets and layers in an independent CAM tool before fabrication.\n",self.width,self.height,self.drc().len())));
        Ok(files)
    }
}
impl Sheet {
    pub fn pcb_drc(&self) -> Vec<PcbIssue> {
        if !self.children.is_empty() {
            return match crate::flatten(self) {
                Ok(mut flat) => {
                    flat.board = self.board.clone();
                    flat.pcb_drc()
                }
                Err(message) => vec![PcbIssue {
                    error: true,
                    message,
                    at: None,
                }],
            };
        }
        let mut out = self.board.drc();
        let n = connect(self);
        let issue = |message: String, at: Option<Mm>| PcbIssue {
            error: true,
            message,
            at,
        };
        for e in n.issues.iter().filter(|e| e.error) {
            out.push(issue(format!("Schematic: {}", e.message), None));
        }
        for c in &self.components {
            if let Some(f) = self
                .board
                .footprints
                .iter()
                .find(|f| f.component_id == c.id)
            {
                let pins = (0..c.pins().len())
                    .map(|p| n.pin_net[&(c.id, p)].to_lowercase())
                    .collect::<Vec<_>>();
                if f.nets.iter().map(|n| n.to_lowercase()).collect::<Vec<_>>() != pins
                    || f.reference != c.reference
                    || f.style != c.footprint
                    || f.custom_pads != c.custom_pads
                {
                    out.push(issue(
                        format!(
                            "{} footprint out of sync. Synchronize from schematic.",
                            c.reference
                        ),
                        Some(f.at),
                    ));
                }
            } else {
                out.push(issue(
                    format!("{} is missing from PCB. Synchronize.", c.reference),
                    None,
                ));
            }
        }
        for f in &self.board.footprints {
            if !self.components.iter().any(|c| c.id == f.component_id) {
                out.push(issue(
                    format!("{} has no schematic component.", f.reference),
                    Some(f.at),
                ));
            }
        }
        for t in &self.board.tracks {
            if !n.nets.iter().any(|n| n.name.eq_ignore_ascii_case(&t.net)) {
                out.push(issue(
                    format!("Track {} has unknown net {}.", t.id, t.net),
                    t.points.first().copied(),
                ));
            }
        }
        out
    }
}
struct Gerber {
    apertures: Vec<String>,
    body: String,
}
impl Gerber {
    fn new() -> Self {
        Self {
            apertures: vec![],
            body: String::new(),
        }
    }
    fn select(&mut self, ap: &str) {
        let i = self
            .apertures
            .iter()
            .position(|a| a == ap)
            .unwrap_or_else(|| {
                self.apertures.push(ap.into());
                self.apertures.len() - 1
            });
        self.body.push_str(&format!("D{}*\n", i + 10));
    }
    fn coord(p: Mm) -> String {
        format!(
            "X{}Y{}",
            (p.x * 1e6).round() as i64,
            (p.y * 1e6).round() as i64
        )
    }
    fn flash(&mut self, ap: &str, p: Mm) {
        self.select(ap);
        self.body.push_str(&format!("{}D03*\n", Self::coord(p)));
    }
    fn polyline(&mut self, ap: &str, ps: &[Mm]) {
        self.select(ap);
        for (i, p) in ps.iter().enumerate() {
            self.body.push_str(&format!(
                "{}D{:02}*\n",
                Self::coord(*p),
                if i == 0 { 2 } else { 1 }
            ));
        }
    }
    fn finish(self) -> String {
        let mut out =
            "G04 CircuitCraft Extended Gerber*\n%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\n".to_string();
        for (i, ap) in self.apertures.iter().enumerate() {
            out.push_str(&format!("%ADD{}{ap}*%\n", i + 10));
        }
        out.push_str("G01*\n");
        out.push_str(&self.body);
        out.push_str("M02*\n");
        out
    }
}
#[derive(Clone)]
enum Geo {
    Disk(Mm, f64),
    Rect(Mm, f64, f64),
    Line(Mm, Mm, f64),
}
struct Shape {
    net: String,
    layer: Option<Copper>,
    object: String,
    label: String,
    geo: Geo,
}
fn layers(a: Option<Copper>, b: Option<Copper>) -> bool {
    a.is_none() || b.is_none() || a == b
}
fn center(g: &Geo) -> Mm {
    match g {
        Geo::Disk(p, _) | Geo::Rect(p, _, _) => *p,
        Geo::Line(a, b, _) => Mm::new((a.x + b.x) / 2., (a.y + b.y) / 2.),
    }
}
fn bounds(g: &Geo) -> (f64, f64, f64, f64) {
    match g {
        Geo::Disk(p, r) => (p.x - r, p.y - r, p.x + r, p.y + r),
        Geo::Rect(p, x, y) => (p.x - x, p.y - y, p.x + x, p.y + y),
        Geo::Line(a, b, r) => (
            a.x.min(b.x) - r,
            a.y.min(b.y) - r,
            a.x.max(b.x) + r,
            a.y.max(b.y) + r,
        ),
    }
}
pub fn point_segment(p: Mm, a: Mm, b: Mm) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx * dx + dy * dy;
    if len < 1e-20 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len).clamp(0., 1.);
    p.distance(Mm::new(a.x + t * dx, a.y + t * dy))
}
fn segment_distance(a: Mm, b: Mm, c: Mm, d: Mm) -> f64 {
    let rx = b.x - a.x;
    let ry = b.y - a.y;
    let sx = d.x - c.x;
    let sy = d.y - c.y;
    let den = rx * sy - ry * sx;
    if den.abs() > 1e-12 {
        let t = ((c.x - a.x) * sy - (c.y - a.y) * sx) / den;
        let u = ((c.x - a.x) * ry - (c.y - a.y) * rx) / den;
        if (0. ..=1.).contains(&t) && (0. ..=1.).contains(&u) {
            return 0.;
        }
    }
    point_segment(a, c, d)
        .min(point_segment(b, c, d))
        .min(point_segment(c, a, b))
        .min(point_segment(d, a, b))
}
fn point_rect(p: Mm, c: Mm, x: f64, y: f64) -> f64 {
    ((p.x - c.x).abs() - x)
        .max(0.)
        .hypot(((p.y - c.y).abs() - y).max(0.))
}
fn distance(a: &Geo, b: &Geo) -> f64 {
    match (a, b) {
        (Geo::Disk(a, r), Geo::Disk(b, s)) => a.distance(*b) - r - s,
        (Geo::Rect(a, x, y), Geo::Rect(b, u, v)) => ((a.x - b.x).abs() - x - u)
            .max(0.)
            .hypot(((a.y - b.y).abs() - y - v).max(0.)),
        (Geo::Disk(a, r), Geo::Rect(b, x, y)) | (Geo::Rect(b, x, y), Geo::Disk(a, r)) => {
            point_rect(*a, *b, *x, *y) - r
        }
        (Geo::Line(a, b, r), Geo::Disk(c, s)) | (Geo::Disk(c, s), Geo::Line(a, b, r)) => {
            point_segment(*c, *a, *b) - r - s
        }
        (Geo::Line(a, b, r), Geo::Line(c, d, s)) => segment_distance(*a, *b, *c, *d) - r - s,
        (Geo::Line(a, b, r), Geo::Rect(c, x, y)) | (Geo::Rect(c, x, y), Geo::Line(a, b, r)) => {
            if point_rect(*a, *c, *x, *y) == 0. || point_rect(*b, *c, *x, *y) == 0. {
                return -*r;
            }
            let p = Mm::new(c.x - x, c.y - y);
            let q = Mm::new(c.x + x, c.y - y);
            let u = Mm::new(c.x + x, c.y + y);
            let v = Mm::new(c.x - x, c.y + y);
            [(p, q), (q, u), (u, v), (v, p)]
                .into_iter()
                .map(|(c, d)| segment_distance(*a, *b, c, d))
                .fold(f64::INFINITY, f64::min)
                - r
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routed_examples_and_stale_footprints() {
        for s in [Sheet::divider(), Sheet::rc(), Sheet::rectifier()] {
            assert!(s.pcb_drc().is_empty(), "{:?}", s.pcb_drc());
            assert_eq!(s.board.gerbers().unwrap().len(), 9);
        }
        let mut s = Sheet::rc();
        s.components[1].reference = "R5".into();
        assert!(s
            .pcb_drc()
            .iter()
            .any(|i| i.message.contains("out of sync")));
        s.sync_board();
        assert!(s.pcb_drc().is_empty());
    }
    #[test]
    fn route_connectivity_and_short() {
        let s = Sheet::divider();
        let mut b = Board::default();
        b.sync(&s);
        assert_eq!(b.ratsnest().len(), 3);
        let ps = b.pads();
        let vin = ps.iter().filter(|p| p.net == "VIN").collect::<Vec<_>>();
        b.add_track(100, "VIN".into(), Copper::Top, 0.3, vin[0].at, vin[1].at);
        assert_eq!(b.ratsnest().len(), 2);
        let mut t = b.tracks[0].clone();
        t.id = 101;
        t.net = "VOUT".into();
        b.tracks.push(t);
        assert!(b.drc().iter().any(|i| i.message.contains("copper short")));
    }
}
