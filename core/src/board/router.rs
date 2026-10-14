//! Bounded, deterministic two-layer A* routing. Existing copper is never removed.
use super::*;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone)]
pub struct RouteOptions {
    pub grid: f64,
    pub width: f64,
    pub allow_vias: bool,
    pub via_diameter: f64,
    pub via_drill: f64,
    pub max_connections: usize,
    pub max_expansions: usize,
}
impl Default for RouteOptions {
    fn default() -> Self {
        Self {
            grid: 0.5,
            width: 0.3,
            allow_vias: true,
            via_diameter: 0.8,
            via_drill: 0.4,
            max_connections: 64,
            max_expansions: 200000,
        }
    }
}
#[derive(Debug, Clone)]
pub struct RouteReport {
    pub routed: usize,
    pub remaining: usize,
    pub added_tracks: usize,
    pub added_vias: usize,
    pub blocked: Vec<String>,
    pub next_id: u64,
}
struct Search<'a> {
    board: &'a Board,
    shapes: Vec<Shape>,
    options: &'a RouteOptions,
    net: &'a str,
    nx: usize,
    ny: usize,
}
impl Search<'_> {
    fn point(&self, i: usize) -> Mm {
        Mm::new(
            (i % self.nx) as f64 * self.options.grid,
            ((i / self.nx) % self.ny) as f64 * self.options.grid,
        )
    }
    fn layer(&self, i: usize) -> Copper {
        if i / (self.nx * self.ny) == 0 {
            Copper::Top
        } else {
            Copper::Bottom
        }
    }
    fn clear(&self, g: &Geo, layer: Option<Copper>) -> bool {
        let b = bounds(g);
        if b.0 < self.board.edge_clearance
            || b.1 < self.board.edge_clearance
            || b.2 > self.board.width - self.board.edge_clearance
            || b.3 > self.board.height - self.board.edge_clearance
        {
            return false;
        }
        self.shapes.iter().all(|s| {
            s.net.eq_ignore_ascii_case(self.net)
                || !layers(layer, s.layer)
                || distance(g, &s.geo) >= self.board.clearance + 1e-7
        })
    }
    fn segment(&self, a: Mm, b: Mm, layer: Copper) -> bool {
        self.clear(&Geo::Line(a, b, self.options.width / 2.), Some(layer))
    }
    fn via(&self, p: Mm) -> bool {
        self.clear(&Geo::Disk(p, self.options.via_diameter / 2.), None)
    }
    // Endpoints need not lie on the routing grid. Short orthogonal stubs use
    // the exact physical pad centres and are checked against the same geometry.
    fn seeds(&self, p: Mm) -> Vec<(usize, Vec<Mm>, u64)> {
        let through = self
            .board
            .pads()
            .iter()
            .any(|pad| pad.at.distance(p) < 1e-6 && pad.drill > 0.);
        let x = (p.x / self.options.grid).round() as isize;
        let y = (p.y / self.options.grid).round() as isize;
        let mut out = vec![];
        for layer in 0..if through { 2 } else { 1 } {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let (x, y) = (x + dx, y + dy);
                    if x < 0 || y < 0 || x >= self.nx as isize || y >= self.ny as isize {
                        continue;
                    }
                    let i = layer * self.nx * self.ny + y as usize * self.nx + x as usize;
                    let q = self.point(i);
                    for corner in [Mm::new(q.x, p.y), Mm::new(p.x, q.y)] {
                        let ps = vec![p, corner, q];
                        if ps
                            .windows(2)
                            .all(|ab| self.segment(ab[0], ab[1], self.layer(i)))
                        {
                            let cost =
                                ((p.x - q.x).abs() + (p.y - q.y).abs()) / self.options.grid * 100.;
                            out.push((i, ps, cost.ceil() as u64));
                            break;
                        }
                    }
                }
            }
        }
        out
    }
    fn path(&self, a: Mm, b: Mm, cancel: &AtomicBool) -> Option<Vec<(Mm, Copper)>> {
        let start = self.seeds(a);
        let goals: HashMap<_, _> = self
            .seeds(b)
            .into_iter()
            .map(|(i, ps, cost)| (i, (ps, cost)))
            .collect();
        if start.is_empty() || goals.is_empty() {
            return None;
        }
        let count = self.nx * self.ny * 2;
        let mut costs = vec![u64::MAX; count];
        let mut parent = vec![usize::MAX; count];
        let mut roots = HashMap::new();
        let mut heap = BinaryHeap::new();
        // Manhattan distance to the nearest goal stub is admissible; include
        // the mandatory via cost when the target is on the opposite layer.
        let heuristic = |i: usize| {
            goals
                .iter()
                .map(|(g, (_, tail))| {
                    ((i % self.nx).abs_diff(g % self.nx)
                        + ((i / self.nx) % self.ny).abs_diff((g / self.nx) % self.ny))
                        as u64
                        * 100
                        + if self.layer(i) != self.layer(*g) {
                            1200
                        } else {
                            0
                        }
                        + tail
                })
                .min()
                .unwrap_or(0)
        };
        for (i, ps, cost) in start {
            if cost < costs[i] {
                costs[i] = cost;
                roots.insert(i, ps);
                heap.push(Reverse((cost + heuristic(i), cost, i)));
            }
        }
        let mut best = None;
        for expanded in 0..self.options.max_expansions {
            if expanded % 256 == 0 && cancel.load(Ordering::Relaxed) {
                return None;
            }
            let Some(Reverse((estimate, cost, i))) = heap.pop() else {
                break;
            };
            if cost != costs[i] {
                continue;
            }
            if best.is_some_and(|(score, _)| estimate > score) {
                break;
            }
            if let Some((_, tail)) = goals.get(&i) {
                let score = cost + tail;
                if best.is_none_or(|(old, _)| score < old) {
                    best = Some((score, i));
                }
            }
            let x = i % self.nx;
            let y = (i / self.nx) % self.ny;
            let mut neighbors = vec![];
            if x > 0 {
                neighbors.push((i - 1, 100));
            }
            if x + 1 < self.nx {
                neighbors.push((i + 1, 100));
            }
            if y > 0 {
                neighbors.push((i - self.nx, 100));
            }
            if y + 1 < self.ny {
                neighbors.push((i + self.nx, 100));
            }
            if self.options.allow_vias && self.via(self.point(i)) {
                let layer = self.nx * self.ny;
                neighbors.push((if i < layer { i + layer } else { i - layer }, 1200));
            }
            for (j, step) in neighbors {
                let c = cost + step;
                if c < costs[j]
                    && (self.layer(i) != self.layer(j)
                        || self.segment(self.point(i), self.point(j), self.layer(i)))
                {
                    costs[j] = c;
                    parent[j] = i;
                    heap.push(Reverse((c + heuristic(j), c, j)));
                }
            }
        }
        let (_, goal) = best?;
        let mut chain = vec![goal];
        while parent[*chain.last()?] != usize::MAX {
            chain.push(parent[*chain.last()?]);
        }
        chain.reverse();
        let mut out: Vec<_> = roots
            .get(&chain[0])?
            .iter()
            .map(|p| (*p, self.layer(chain[0])))
            .collect();
        out.extend(chain.iter().map(|i| (self.point(*i), self.layer(*i))));
        out.extend(
            goals
                .get(&goal)?
                .0
                .iter()
                .rev()
                .map(|p| (*p, self.layer(goal))),
        );
        out.dedup_by(|a, b| a.1 == b.1 && a.0.distance(b.0) < 1e-9);
        Some(out)
    }
}

impl Board {
    pub fn autoroute(
        &mut self,
        options: &RouteOptions,
        next_id: u64,
    ) -> Result<RouteReport, String> {
        self.autoroute_cancel(options, next_id, &AtomicBool::new(false))
    }
    pub fn autoroute_cancel(
        &mut self,
        o: &RouteOptions,
        mut next_id: u64,
        cancel: &AtomicBool,
    ) -> Result<RouteReport, String> {
        self.validate()?;
        if !o.grid.is_finite()
            || !(0.1..=2.).contains(&o.grid)
            || !o.width.is_finite()
            || o.width < self.min_track
            || o.width > 5.
            || !o.via_diameter.is_finite()
            || !o.via_drill.is_finite()
            || o.via_drill <= 0.
            || o.via_diameter <= o.via_drill
            || o.via_diameter > 5.
            || o.max_connections == 0
            || o.max_connections > 128
            || o.max_expansions == 0
            || o.max_expansions > 500000
        {
            return Err("Invalid router options: grid 0.1–2 mm, width ≥ minimum, valid via, 1–128 connections, at most 500,000 search steps.".into());
        }
        let nx = (self.width / o.grid).floor() as usize + 1;
        let ny = (self.height / o.grid).floor() as usize + 1;
        if nx * ny * 2 > 500000 || self.copper().len() > 1500 {
            return Err("Routing limit: 500,000 grid states and 1,500 initial copper shapes; use a larger grid or a smaller board.".into());
        }
        if self
            .drc()
            .iter()
            .any(|i| !i.message.starts_with("Unrouted connection"))
        {
            return Err("Fix board geometry/clearance errors before autorouting.".into());
        }
        next_id = next_id.max(
            self.tracks
                .iter()
                .map(|t| t.id)
                .chain(self.vias.iter().map(|v| v.id))
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("ID space exhausted")?,
        );
        let mut work = self.clone();
        let initial = work.ratsnest().len();
        let mut report = RouteReport {
            routed: 0,
            remaining: initial,
            added_tracks: 0,
            added_vias: 0,
            blocked: vec![],
            next_id,
        };
        let mut failed = vec![];
        for _ in 0..o.max_connections {
            if cancel.load(Ordering::Relaxed) {
                return Err("Autorouting cancelled; original board retained.".into());
            }
            let todo = work.ratsnest();
            let Some((net, a, b)) = todo
                .iter()
                .find(|(net, a, b)| !failed.contains(&(net.clone(), *a, *b)))
            else {
                break;
            };
            let search = Search {
                board: &work,
                shapes: work.copper(),
                options: o,
                net,
                nx,
                ny,
            };
            let Some(path) = search.path(*a, *b, cancel) else {
                failed.push((net.clone(), *a, *b));
                report
                    .blocked
                    .push(format!("{net}: no path within grid/search limits"));
                continue;
            };
            let mut candidate = work.clone();
            let mut ps = vec![path[0].0];
            let mut layer = path[0].1;
            let track_start = candidate.tracks.len();
            let via_start = candidate.vias.len();
            for (p, l) in path.into_iter().skip(1) {
                if layer != l {
                    append_track(&mut candidate, &mut next_id, net, layer, o.width, &ps)?;
                    candidate.vias.push(Via {
                        id: next_id,
                        net: net.clone(),
                        at: p,
                        diameter: o.via_diameter,
                        drill: o.via_drill,
                    });
                    next_id = next_id.checked_add(1).ok_or("ID space exhausted")?;
                    ps = vec![p];
                    layer = l;
                } else {
                    ps.push(p);
                }
            }
            append_track(&mut candidate, &mut next_id, net, layer, o.width, &ps)?;
            let issues = candidate.drc();
            let remaining = candidate.ratsnest().len();
            if issues
                .iter()
                .any(|i| !i.message.starts_with("Unrouted connection"))
                || remaining >= todo.len()
            {
                failed.push((net.clone(), *a, *b));
                report.blocked.push(format!(
                    "{net}: candidate rejected by final DRC/connectivity"
                ));
                continue;
            }
            report.added_tracks += candidate.tracks.len() - track_start;
            report.added_vias += candidate.vias.len() - via_start;
            work = candidate;
            failed.clear();
            report.routed = initial - remaining;
            report.remaining = remaining;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("Autorouting cancelled; original board retained.".into());
        }
        report.next_id = next_id;
        *self = work;
        Ok(report)
    }
}
fn append_track(
    b: &mut Board,
    id: &mut u64,
    net: &str,
    layer: Copper,
    width: f64,
    ps: &[Mm],
) -> Result<(), String> {
    let mut points: Vec<Mm> = vec![];
    for p in ps {
        if points.last().is_some_and(|q| q.distance(*p) < 1e-9) {
            continue;
        }
        if points.len() >= 2 {
            let a = points[points.len() - 2];
            let q = points[points.len() - 1];
            if ((q.x - a.x) * (p.y - q.y) - (q.y - a.y) * (p.x - q.x)).abs() < 1e-9 {
                points.pop();
            }
        }
        points.push(*p);
    }
    if points.len() >= 2 {
        if points.len() > 1000 {
            return Err("Routed track exceeds geometry limit".into());
        }
        b.tracks.push(Track {
            id: *id,
            net: net.into(),
            layer,
            width,
            points,
        });
        *id = id.checked_add(1).ok_or("ID space exhausted")?;
    }
    Ok(())
}
