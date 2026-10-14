use crate::{Board, Copper, Mm, PadSpec, Track};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize,
)]
pub struct Grid {
    pub x: i32,
    pub y: i32,
}
impl Grid {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn rotate(self, r: u8) -> Self {
        match r % 4 {
            0 => self,
            1 => Self::new(-self.y, self.x),
            2 => Self::new(-self.x, -self.y),
            _ => Self::new(self.y, -self.x),
        }
    }
    pub fn plus(self, p: Self) -> Self {
        Self::new(self.x + p.x, self.y + p.y)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Resistor,
    Capacitor,
    Inductor,
    Voltage,
    Current,
    Diode,
    Led,
    Npn,
    Pnp,
    Nmos,
    Pmos,
    OpAmp,
    Switch,
    Connector,
    Subcircuit,
}
impl Kind {
    pub const ALL: [Self; 15] = [
        Self::Resistor,
        Self::Capacitor,
        Self::Inductor,
        Self::Voltage,
        Self::Current,
        Self::Diode,
        Self::Led,
        Self::Npn,
        Self::Pnp,
        Self::Nmos,
        Self::Pmos,
        Self::OpAmp,
        Self::Switch,
        Self::Connector,
        Self::Subcircuit,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Resistor => "Resistor",
            Self::Capacitor => "Capacitor",
            Self::Inductor => "Inductor",
            Self::Voltage => "Voltage source",
            Self::Current => "Current source",
            Self::Diode => "Diode",
            Self::Led => "LED",
            Self::Npn => "NPN transistor",
            Self::Pnp => "PNP transistor",
            Self::Nmos => "N-channel MOSFET",
            Self::Pmos => "P-channel MOSFET",
            Self::OpAmp => "Ideal op amp (VCVS)",
            Self::Switch => "Controlled switch",
            Self::Connector => "2-pin connector",
            Self::Subcircuit => "4-pin SPICE subcircuit",
        }
    }
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Resistor => "R",
            Self::Capacitor => "C",
            Self::Inductor => "L",
            Self::Voltage => "V",
            Self::Current => "I",
            Self::Diode | Self::Led => "D",
            Self::Npn | Self::Pnp => "Q",
            Self::Nmos | Self::Pmos => "M",
            Self::OpAmp => "E",
            Self::Switch => "S",
            Self::Connector => "J",
            Self::Subcircuit => "X",
        }
    }
    pub fn default_value(self) -> &'static str {
        match self {
            Self::Resistor => "1k",
            Self::Capacitor => "1u",
            Self::Inductor => "10m",
            Self::Voltage => "DC 5 AC 1",
            Self::Current => "DC 1m",
            Self::Diode => "DGEN",
            Self::Led => "DLED",
            Self::Npn => "QGEN",
            Self::Pnp => "QPGEN",
            Self::Nmos => "NGEN",
            Self::Pmos => "PGEN",
            Self::OpAmp => "100000",
            Self::Switch => "SWGEN",
            Self::Connector => "",
            Self::Subcircuit => "CUSTOM",
        }
    }
    /// Terminal numbers follow SPICE, and PCB pad numbers use this same order.
    pub fn pins(self) -> Vec<(&'static str, Grid)> {
        match self {
            Self::Voltage | Self::Current => vec![("+", Grid::new(0, -2)), ("-", Grid::new(0, 2))],
            Self::Npn | Self::Pnp => vec![
                ("C", Grid::new(2, -2)),
                ("B", Grid::new(-2, 0)),
                ("E", Grid::new(2, 2)),
            ],
            Self::Nmos | Self::Pmos => vec![
                ("D", Grid::new(2, -2)),
                ("G", Grid::new(-2, 0)),
                ("S", Grid::new(2, 2)),
                ("B", Grid::new(3, 1)),
            ],
            Self::OpAmp => vec![
                ("OUT+", Grid::new(3, 0)),
                ("OUT-", Grid::new(0, 3)),
                ("IN+", Grid::new(-3, -1)),
                ("IN-", Grid::new(-3, 1)),
            ],
            Self::Switch => vec![
                ("A", Grid::new(-2, 0)),
                ("B", Grid::new(2, 0)),
                ("CTRL+", Grid::new(-1, 3)),
                ("CTRL-", Grid::new(1, 3)),
            ],
            Self::Subcircuit => vec![
                ("1", Grid::new(-3, -1)),
                ("2", Grid::new(-3, 1)),
                ("3", Grid::new(3, -1)),
                ("4", Grid::new(3, 1)),
            ],
            _ => vec![("1", Grid::new(-2, 0)), ("2", Grid::new(2, 0))],
        }
    }
    pub fn default_footprint(self) -> &'static str {
        if matches!(self, Self::Npn | Self::Pnp) {
            "TO-92"
        } else if self.pins().len() == 2 {
            "THT-2-5.08"
        } else {
            "DIP-4-7.62"
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: u64,
    pub kind: Kind,
    pub reference: String,
    pub value: String,
    pub at: Grid,
    pub rotation: u8,
    #[serde(default)]
    pub params: String,
    pub footprint: String,
    #[serde(default)]
    pub custom_pads: Vec<PadSpec>,
}
impl Component {
    pub fn pins(&self) -> Vec<(&'static str, Grid)> {
        self.kind
            .pins()
            .into_iter()
            .map(|(n, p)| (n, self.at.plus(p.rotate(self.rotation))))
            .collect()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wire {
    pub id: u64,
    pub points: Vec<Grid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub at: Grid,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub format_version: u32,
    pub title: String,
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
    pub grounds: Vec<Grid>,
    pub junctions: Vec<Grid>,
    pub labels: Vec<Label>,
    pub spice_models: String,
    #[serde(default)]
    pub raw_netlist: String,
    #[serde(default)]
    pub raw_mode: bool,
    #[serde(default)]
    pub include_directory: String,
    pub notes: String,
    pub board: Board,
    next_id: u64,
}
impl Default for Sheet {
    fn default() -> Self {
        Self {
            format_version: 1,
            title: "Untitled circuit".into(),
            components: vec![],
            wires: vec![],
            grounds: vec![],
            junctions: vec![],
            labels: vec![],
            spice_models: String::new(),
            raw_netlist: String::new(),
            raw_mode: false,
            include_directory: String::new(),
            notes: String::new(),
            board: Board::default(),
            next_id: 1,
        }
    }
}
impl Sheet {
    pub fn id(&mut self) -> u64 {
        let n = self.next_id;
        self.next_id += 1;
        n
    }
    pub fn add(&mut self, kind: Kind, at: Grid) -> u64 {
        let mut n = 1;
        while self.components.iter().any(|c| {
            c.reference
                .eq_ignore_ascii_case(&format!("{}{n}", kind.prefix()))
        }) {
            n += 1;
        }
        let id = self.id();
        self.components.push(Component {
            id,
            kind,
            reference: format!("{}{n}", kind.prefix()),
            value: kind.default_value().into(),
            at,
            rotation: 0,
            params: String::new(),
            footprint: kind.default_footprint().into(),
            custom_pads: vec![],
        });
        id
    }
    pub fn wire(&mut self, a: Grid, b: Grid) {
        if a == b {
            return;
        }
        let mut ps = vec![a];
        if a.x != b.x && a.y != b.y {
            ps.push(Grid::new(b.x, a.y));
        }
        ps.push(b);
        let id = self.id();
        self.wires.push(Wire { id, points: ps });
    }
    pub fn label(&mut self, at: Grid, name: &str) {
        let id = self.id();
        self.labels.push(Label {
            id,
            at,
            name: name.into(),
        });
    }
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut s: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if s.format_version != 1 {
            return Err("Unsupported project format.".into());
        }
        if s.components.len() > 10000 || s.wires.len() > 30000 {
            return Err("Project exceeds 10,000 components / 30,000 wires.".into());
        }
        let mut ids = HashSet::new();
        for id in s
            .components
            .iter()
            .map(|c| c.id)
            .chain(s.wires.iter().map(|w| w.id))
            .chain(s.labels.iter().map(|l| l.id))
            .chain(s.board.tracks.iter().map(|t| t.id))
            .chain(s.board.vias.iter().map(|v| v.id))
        {
            if id == 0 || !ids.insert(id) {
                return Err("Duplicate or zero identifier.".into());
            }
        }
        for w in &s.wires {
            if w.points.len() < 2 || w.points.len() > 1000 {
                return Err("Malformed wire.".into());
            }
        }
        for p in s
            .components
            .iter()
            .map(|c| c.at)
            .chain(s.wires.iter().flat_map(|w| w.points.iter().copied()))
            .chain(s.grounds.iter().copied())
            .chain(s.junctions.iter().copied())
            .chain(s.labels.iter().map(|l| l.at))
        {
            if p.x.unsigned_abs() > 1000000 || p.y.unsigned_abs() > 1000000 {
                return Err("Coordinate out of range.".into());
            }
        }
        for c in &s.components {
            if !c.custom_pads.is_empty() {
                crate::FootprintDefinition {
                    name: c.footprint.clone(),
                    pads: c.custom_pads.clone(),
                }
                .validate()?;
                if c.custom_pads.len() != c.pins().len() {
                    return Err(format!(
                        "{}: custom footprint pad count must match SPICE terminals.",
                        c.reference
                    ));
                }
            }
        }
        s.board.validate()?;
        s.next_id = ids.into_iter().max().unwrap_or(0) + 1;
        Ok(s)
    }
    pub fn sync_board(&mut self) {
        let mut b = self.board.clone();
        b.sync(self);
        self.board = b;
    }
    fn demo_board(&mut self) {
        self.sync_board();
        for (f, p) in self.board.footprints.iter_mut().zip([
            Mm::new(20., 30.),
            Mm::new(40., 45.),
            Mm::new(55., 25.),
        ]) {
            f.at = p;
        }
        let a = self.id();
        let b = self.id();
        let c = self.id();
        self.board.tracks = vec![
            Track {
                id: a,
                net: "VIN".into(),
                layer: Copper::Top,
                width: 0.4,
                points: vec![
                    Mm::new(17.46, 30.),
                    Mm::new(17.46, 45.),
                    Mm::new(37.46, 45.),
                ],
            },
            Track {
                id: b,
                net: "VOUT".into(),
                layer: Copper::Bottom,
                width: 0.4,
                points: vec![
                    Mm::new(42.54, 45.),
                    Mm::new(52.46, 45.),
                    Mm::new(52.46, 25.),
                ],
            },
            Track {
                id: c,
                net: "0".into(),
                layer: Copper::Top,
                width: 0.6,
                points: vec![
                    Mm::new(22.54, 30.),
                    Mm::new(22.54, 10.),
                    Mm::new(57.54, 10.),
                    Mm::new(57.54, 25.),
                ],
            },
        ];
    }
    pub fn divider() -> Self {
        let mut s = Self {
            title: "Voltage divider · 10 V to 5 V".into(),
            ..Self::default()
        };
        s.add(Kind::Voltage, Grid::new(0, 0));
        s.components[0].value = "DC 10 AC 1".into();
        s.add(Kind::Resistor, Grid::new(6, -2));
        s.add(Kind::Resistor, Grid::new(8, 3));
        s.components[2].rotation = 1;
        for (a, b) in [
            (Grid::new(0, -2), Grid::new(4, -2)),
            (Grid::new(8, -2), Grid::new(8, 1)),
            (Grid::new(8, 5), Grid::new(0, 5)),
            (Grid::new(0, 5), Grid::new(0, 2)),
        ] {
            s.wire(a, b);
        }
        s.grounds.push(Grid::new(0, 5));
        s.label(Grid::new(0, -2), "VIN");
        s.label(Grid::new(8, -2), "VOUT");
        s.notes="10 V source and two 1 kΩ resistors. Expected VOUT = 5 V. PCB pin numbers follow SPICE terminal order; verify against the actual component datasheet.".into();
        s.demo_board();
        s
    }
    pub fn rc() -> Self {
        let mut s = Self::divider();
        s.title = "RC low-pass · 159.15 Hz".into();
        s.components[2].kind = Kind::Capacitor;
        s.components[2].reference = "C1".into();
        s.components[2].value = "1u".into();
        s.components[0].value = "DC 1 AC 1 PULSE(0 1 0 1n 1n 1 2)".into();
        s.notes="1 kΩ / 1 µF RC filter. Cutoff = 159.155 Hz. Run transient with UIC to see the step response.".into();
        s.sync_board();
        s
    }
    pub fn rectifier() -> Self {
        let mut s = Self::divider();
        s.title = "Half-wave rectifier".into();
        s.components[0].value = "DC 0 SIN(0 10 50)".into();
        s.components[1].kind = Kind::Diode;
        s.components[1].reference = "D1".into();
        s.components[1].value = "DGEN".into();
        let c = s.add(Kind::Capacitor, Grid::new(13, 3));
        let part = s.components.last_mut().unwrap();
        part.rotation = 1;
        part.value = "100u".into();
        s.wire(Grid::new(8, -2), Grid::new(13, 1));
        s.wire(Grid::new(13, 5), Grid::new(8, 5));
        s.notes="50 Hz sine, generic diode, 1 kΩ load, 100 µF capacitor. Transient: stop 200m, step 100u.".into();
        s.sync_board();
        s.board
            .footprints
            .iter_mut()
            .find(|f| f.component_id == c)
            .unwrap()
            .at = Mm::new(65., 40.);
        let a = s.id();
        let b = s.id();
        s.board.tracks.push(Track {
            id: a,
            net: "VOUT".into(),
            layer: Copper::Bottom,
            width: 0.4,
            points: vec![
                Mm::new(42.54, 45.),
                Mm::new(62.46, 45.),
                Mm::new(62.46, 40.),
            ],
        });
        s.board.tracks.push(Track {
            id: b,
            net: "0".into(),
            layer: Copper::Top,
            width: 0.4,
            points: vec![
                Mm::new(57.54, 25.),
                Mm::new(67.54, 25.),
                Mm::new(67.54, 40.),
            ],
        });
        s
    }
}
#[derive(Default)]
pub struct History {
    undo: Vec<Sheet>,
    redo: Vec<Sheet>,
}
impl History {
    pub fn record(&mut self, s: &Sheet) {
        if self.undo.last() != Some(s) {
            self.undo.push(s.clone());
            if self.undo.len() > 80 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }
    pub fn undo(&mut self, s: &mut Sheet) -> bool {
        if let Some(p) = self.undo.pop() {
            self.redo.push(s.clone());
            *s = p;
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self, s: &mut Sheet) -> bool {
        if let Some(p) = self.redo.pop() {
            self.undo.push(s.clone());
            *s = p;
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_history() {
        let mut s = Sheet::rc();
        let original = s.clone();
        let mut h = History::default();
        h.record(&s);
        s.add(Kind::Led, Grid::new(20, 0));
        assert!(h.undo(&mut s));
        assert_eq!(s, original);
        assert!(h.redo(&mut s));
        assert_eq!(s, Sheet::from_json(&s.to_json().unwrap()).unwrap());
        let max = s.board.tracks.iter().map(|t| t.id).max().unwrap();
        assert!(s.id() > max);
    }
}
