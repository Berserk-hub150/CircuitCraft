//! Embedded hierarchical pages. Labels are local; explicit port bindings connect
//! a child's labelled net to a labelled net on its immediate parent. Ground is global.
use crate::{connect, valid_node, Grid, Label, Sheet};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetPort {
    pub label: String,
    pub parent_net: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChildSheet {
    pub instance: String,
    #[serde(default)]
    pub ports: Vec<SheetPort>,
    pub sheet: Box<Sheet>,
}

pub fn validate_hierarchy(s: &Sheet) -> Result<(), String> {
    fn visit(s: &Sheet, depth: usize, count: &mut usize) -> Result<(), String> {
        *count += 1;
        if depth > 8 || *count > 64 {
            return Err("Hierarchy limit: 8 nested levels and 64 pages.".into());
        }
        let mut names = HashSet::new();
        for c in &s.children {
            if !valid_node(&c.instance)
                || c.instance.len() > 24
                || !names.insert(c.instance.to_lowercase())
            {
                return Err(
                    "Sheet instances need unique names of 1–24 letters/digits/underscores.".into(),
                );
            }
            let mut ports = HashSet::new();
            for p in &c.ports {
                if !valid_node(&p.label)
                    || !valid_node(&p.parent_net)
                    || !ports.insert(p.label.to_lowercase())
                {
                    return Err(format!("{}: invalid or duplicate port binding", c.instance));
                }
            }
            if c.sheet.raw_mode || !c.sheet.include_directory.is_empty() {
                return Err(
                    "Child pages use graphical schematics and the root include directory.".into(),
                );
            }
            visit(&c.sheet, depth + 1, count)?;
        }
        Ok(())
    }
    visit(s, 0, &mut 0)
}

/// Stable IDs keep footprint placement when a sibling page is inserted or removed.
fn scoped_id(path: &str, id: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in path.bytes().chain(id.to_le_bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h | (1 << 63)
}

pub fn flatten(s: &Sheet) -> Result<Sheet, String> {
    if s.children.is_empty() {
        return Ok(s.clone());
    }
    validate_hierarchy(s)?;
    let mut out = Sheet::default();
    out.title = s.title.clone();
    out.include_directory = s.include_directory.clone();
    out.spice_models = s.spice_models.clone();
    let mut ids: HashSet<u64> = s.components.iter().map(|c| c.id).collect();
    let mut names = HashSet::new();
    let root = connect(s);
    names.extend(root.nets.iter().map(|n| n.name.to_lowercase()));
    let root_map = root
        .nets
        .iter()
        .map(|n| (n.name.to_lowercase(), n.name.clone()))
        .collect();
    fn visit(
        s: &Sheet,
        path: &str,
        map: &HashMap<String, String>,
        out: &mut Sheet,
        ids: &mut HashSet<u64>,
        names: &mut HashSet<String>,
    ) -> Result<(), String> {
        let n = connect(s);
        for e in n
            .issues
            .iter()
            .filter(|e| e.error && !e.message.starts_with("Missing electrical ground"))
        {
            return Err(format!("{path}: {}", e.message));
        }
        // Every component gets a disjoint drawing slot; labels carry its original
        // connectivity. This is a simulation/PCB flattening, not a flattened drawing.
        for c in &s.components {
            if out.components.len() >= 10000 {
                return Err("Hierarchy exceeds 10,000 components".into());
            }
            let mut f = c.clone();
            if !path.is_empty() {
                f.id = scoped_id(path, c.id);
                if !ids.insert(f.id) {
                    return Err("Hierarchical component ID collision".into());
                }
                f.reference = format!(
                    "{}_{}_{}",
                    c.kind.prefix(),
                    path.replace('/', "_"),
                    c.reference
                );
                if !valid_node(&f.reference) {
                    return Err(
                        "Hierarchical reference exceeds 64 characters; shorten instance names"
                            .into(),
                    );
                }
            }
            f.at = Grid::new((out.components.len() as i32) * 32, 0);
            for (pi, (_, at)) in f.pins().into_iter().enumerate() {
                let net = &map[&n.pin_net[&(c.id, pi)].to_lowercase()];
                out.labels.push(Label {
                    id: 0,
                    at,
                    name: net.clone(),
                });
            }
            out.components.push(f);
        }
        for child in &s.children {
            let child_path = if path.is_empty() {
                child.instance.clone()
            } else {
                format!("{path}/{}", child.instance)
            };
            let cn = connect(&child.sheet);
            let resolve = |sheet: &Sheet,
                           conn: &crate::Connectivity,
                           label: &str|
             -> Result<String, String> {
                if label == "0" || label.eq_ignore_ascii_case("GND") {
                    if conn.nets.iter().any(|n| n.name == "0") {
                        return Ok("0".into());
                    }
                }
                let l = sheet
                    .labels
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(label))
                    .ok_or_else(|| {
                        format!("{child_path}: labelled port/net '{label}' does not exist")
                    })?;
                Ok(conn.point_net[&l.at].to_lowercase())
            };
            let mut cm = HashMap::<String, String>::new();
            cm.insert("0".into(), "0".into());
            for p in &child.ports {
                let local = resolve(&child.sheet, &cn, &p.label)?;
                let parent = resolve(s, &n, &p.parent_net)?;
                let target = map
                    .get(&parent)
                    .ok_or("Parent port has no mapped net")?
                    .clone();
                if let Some(old) = cm.insert(local, target.clone()) {
                    if old != target {
                        return Err(format!(
                            "{child_path}: bindings would short different parent nets"
                        ));
                    }
                }
            }
            for net in &cn.nets {
                let key = net.name.to_lowercase();
                if cm.contains_key(&key) {
                    continue;
                }
                let base = format!(
                    "h{:x}_{}",
                    scoped_id(&child_path.to_lowercase(), 0),
                    net.name
                );
                let base = base.chars().take(56).collect::<String>();
                let mut candidate = base.clone();
                let mut i = 0;
                while !names.insert(candidate.to_lowercase()) {
                    i += 1;
                    candidate = format!("{base}_{i}");
                }
                cm.insert(key, candidate);
            }
            if !child.sheet.spice_models.trim().is_empty() {
                out.spice_models.push_str(&format!(
                    "\n* Sheet {child_path}\n{}\n",
                    child.sheet.spice_models
                ));
            }
            visit(&child.sheet, &child_path, &cm, out, ids, names)?;
        }
        Ok(())
    }
    visit(s, "", &root_map, &mut out, &mut ids, &mut names)?;
    let mut next = 1u64;
    for l in &mut out.labels {
        while ids.contains(&next) {
            next += 1;
        }
        l.id = next;
        ids.insert(next);
    }
    Ok(out)
}

/// A 10 V divider whose load is on a child page. OUT must be exactly 5 V.
pub fn hierarchical_divider() -> Sheet {
    let mut s = Sheet::default();
    s.title = "Hierarchical voltage divider".into();
    s.add(crate::Kind::Voltage, Grid::new(0, 0));
    s.components[0].value = "DC 10".into();
    s.add(crate::Kind::Resistor, Grid::new(6, -2));
    s.wire(Grid::new(0, -2), Grid::new(4, -2));
    s.grounds.push(Grid::new(0, 2));
    s.label(Grid::new(8, -2), "OUT");
    let mut load = Sheet::default();
    load.title = "Load".into();
    load.add(crate::Kind::Resistor, Grid::new(0, 0));
    load.label(Grid::new(-2, 0), "IN");
    load.grounds.push(Grid::new(2, 0));
    s.children.push(ChildSheet {
        instance: "LOAD".into(),
        ports: vec![SheetPort {
            label: "IN".into(),
            parent_net: "OUT".into(),
        }],
        sheet: Box::new(load),
    });
    s.sync_board();
    s
}
