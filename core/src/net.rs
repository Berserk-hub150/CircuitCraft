use crate::{Grid, Kind, Sheet};
use std::collections::{BTreeMap, HashMap, HashSet};
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub error: bool,
    pub message: String,
    pub component: Option<u64>,
}
#[derive(Debug, Clone)]
pub struct Net {
    pub name: String,
    pub points: Vec<Grid>,
    pub pins: Vec<(u64, usize)>,
}
#[derive(Debug, Clone)]
pub struct Connectivity {
    pub nets: Vec<Net>,
    pub pin_net: HashMap<(u64, usize), String>,
    pub point_net: HashMap<Grid, String>,
    pub wire_net: HashMap<u64, String>,
    pub issues: Vec<Diagnostic>,
}
pub fn valid_node(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
pub fn on_segment(p: Grid, a: Grid, b: Grid) -> bool {
    let dx = b.x as i64 - a.x as i64;
    let dy = b.y as i64 - a.y as i64;
    (p.x as i64 - a.x as i64) * dy == (p.y as i64 - a.y as i64) * dx
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
}
struct Dsu(Vec<usize>);
impl Dsu {
    fn root(&mut self, i: usize) -> usize {
        if self.0[i] != i {
            self.0[i] = self.root(self.0[i]);
        }
        self.0[i]
    }
    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        self.0[b] = a;
    }
}
pub fn connect(s: &Sheet) -> Connectivity {
    let mut points = vec![];
    let mut index = HashMap::new();
    let add = |p: Grid, points: &mut Vec<Grid>, index: &mut HashMap<Grid, usize>| {
        if let std::collections::hash_map::Entry::Vacant(e) = index.entry(p) {
            e.insert(points.len());
            points.push(p);
        }
    };
    for c in &s.components {
        for (_, p) in c.pins() {
            add(p, &mut points, &mut index);
        }
    }
    for p in s
        .grounds
        .iter()
        .chain(s.junctions.iter())
        .copied()
        .chain(s.labels.iter().map(|l| l.at))
        .chain(s.wires.iter().flat_map(|w| w.points.iter().copied()))
    {
        add(p, &mut points, &mut index);
    }
    let mut d = Dsu((0..points.len()).collect());
    let mut issues = vec![];
    let error = |msg: String, id: Option<u64>| Diagnostic {
        error: true,
        message: msg,
        component: id,
    };
    for w in &s.wires {
        for ab in w.points.windows(2) {
            d.join(index[&ab[0]], index[&ab[1]]);
            if ab[0].x != ab[1].x && ab[0].y != ab[1].y {
                issues.push(error(format!("Wire {} is diagonal.", w.id), None));
            }
            for (i, p) in points.iter().enumerate() {
                if on_segment(*p, ab[0], ab[1]) {
                    d.join(index[&ab[0]], i);
                }
            }
        }
    }
    let mut ground_points = s.grounds.clone();
    ground_points.extend(
        s.labels
            .iter()
            .filter(|l| l.name == "0" || l.name.eq_ignore_ascii_case("gnd"))
            .map(|l| l.at),
    );
    if let Some(first) = ground_points.first() {
        for p in &ground_points {
            d.join(index[first], index[p]);
        }
    }
    let mut labels = HashMap::<String, usize>::new();
    for l in &s.labels {
        if !valid_node(&l.name) {
            issues.push(error(
                format!(
                    "Invalid net label '{}'. Use letters, digits and underscores.",
                    l.name
                ),
                None,
            ));
            continue;
        }
        let key = l.name.to_lowercase();
        let i = index[&l.at];
        if let Some(j) = labels.get(&key) {
            d.join(i, *j);
        } else {
            labels.insert(key, i);
        }
    }
    let ground = ground_points.first().map(|p| d.root(index[p]));
    if ground.is_none() {
        issues.push(error(
            "Missing electrical ground. Add a ground symbol or GND / 0 label.".into(),
            None,
        ));
    }
    let mut groups = BTreeMap::<usize, Vec<Grid>>::new();
    for (i, p) in points.iter().enumerate() {
        groups.entry(d.root(i)).or_default().push(*p);
    }
    let mut root_names = HashMap::new();
    let mut used = HashSet::new();
    used.insert("0".to_string());
    for (r, ps) in &groups {
        let mut names = s
            .labels
            .iter()
            .filter(|l| ps.contains(&l.at) && valid_node(&l.name))
            .map(|l| l.name.clone())
            .collect::<Vec<_>>();
        names.sort_by_key(|n| n.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        if Some(*r) == ground {
            root_names.insert(*r, "0".to_string());
        } else if let Some(name) = names.first() {
            root_names.insert(*r, name.clone());
            used.insert(name.to_lowercase());
        }
        if names.len() > 1 && Some(*r) != ground {
            issues.push(Diagnostic {
                error: false,
                message: format!("Connected labels {} share one net.", names.join(", ")),
                component: None,
            });
        }
    }
    for r in groups.keys() {
        if !root_names.contains_key(r) {
            let mut i = 1;
            while used.contains(&format!("n{i:03}")) {
                i += 1;
            }
            let name = format!("N{i:03}");
            used.insert(name.to_lowercase());
            root_names.insert(*r, name);
        }
    }
    let mut point_net = HashMap::new();
    let mut nets = vec![];
    for (r, ps) in groups {
        let name = root_names[&r].clone();
        for p in &ps {
            point_net.insert(*p, name.clone());
        }
        nets.push(Net {
            name,
            points: ps,
            pins: vec![],
        });
    }
    let mut pin_net = HashMap::new();
    let mut refs = HashSet::new();
    for c in &s.components {
        if !refs.insert(c.reference.to_lowercase()) {
            issues.push(error(
                format!("Duplicate reference {}.", c.reference),
                Some(c.id),
            ));
        }
        if !valid_node(&c.reference) || !c.reference.starts_with(c.kind.prefix()) {
            issues.push(error(
                format!(
                    "{} reference must start with {}.",
                    c.reference,
                    c.kind.prefix()
                ),
                Some(c.id),
            ));
        }
        for (pi, (_, p)) in c.pins().iter().enumerate() {
            let name = point_net[p].clone();
            pin_net.insert((c.id, pi), name.clone());
            nets.iter_mut()
                .find(|n| n.name == name)
                .unwrap()
                .pins
                .push((c.id, pi));
            let contact = s
                .wires
                .iter()
                .any(|w| w.points.windows(2).any(|ab| on_segment(*p, ab[0], ab[1])))
                || ground_points.contains(p)
                || s.labels.iter().any(|l| l.at == *p)
                || s.components
                    .iter()
                    .any(|o| o.id != c.id && o.pins().iter().any(|(_, q)| q == p));
            if !contact {
                issues.push(Diagnostic {
                    error: false,
                    message: format!("{} terminal {} is unconnected.", c.reference, pi + 1),
                    component: Some(c.id),
                });
            }
        }
        if matches!(c.kind, Kind::Resistor | Kind::Capacitor | Kind::Inductor)
            && crate::parse_value(&c.value)
                .map(|v| v <= 0.)
                .unwrap_or(!c.value.trim().starts_with('{'))
        {
            issues.push(error(
                format!(
                    "{} requires a positive value or SPICE parameter.",
                    c.reference
                ),
                Some(c.id),
            ));
        }
        if c.value.contains(['\n', '\r']) || c.params.contains(['\n', '\r']) {
            issues.push(error(
                format!("{} value and parameters must be single lines.", c.reference),
                Some(c.id),
            ));
        }
    }
    let wire_net = s
        .wires
        .iter()
        .filter_map(|w| w.points.first().map(|p| (w.id, point_net[p].clone())))
        .collect();
    nets.sort_by(|a, b| a.name.cmp(&b.name));
    Connectivity {
        nets,
        pin_net,
        point_net,
        wire_net,
        issues,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_and_junctions() {
        let mut s = Sheet::default();
        s.wire(Grid::new(-3, 0), Grid::new(3, 0));
        s.wire(Grid::new(0, -3), Grid::new(0, 3));
        s.grounds.push(Grid::new(-3, 0));
        let n = connect(&s);
        assert_ne!(
            n.point_net[&Grid::new(-3, 0)],
            n.point_net[&Grid::new(0, -3)]
        );
        s.junctions.push(Grid::new(0, 0));
        let n = connect(&s);
        assert_eq!(n.point_net[&Grid::new(0, -3)], "0");
        s.junctions.clear();
        s.label(Grid::new(0, -3), "REMOTE");
        s.label(Grid::new(20, 20), "remote");
        let n = connect(&s);
        assert_eq!(
            n.point_net[&Grid::new(20, 20)],
            n.point_net[&Grid::new(0, -3)]
        );
    }
    #[test]
    fn divider_topology() {
        let s = Sheet::divider();
        let n = connect(&s);
        assert_eq!(n.nets.len(), 3);
        assert!(n.issues.is_empty(), "{:?}", n.issues);
        assert_eq!(n.pin_net[&(s.components[1].id, 1)], "VOUT");
    }
}
