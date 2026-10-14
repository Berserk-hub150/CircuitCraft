//! A shared, millimetre-scale triangle mesh for the interactive review and OBJ.
//! Component bodies are footprint-derived envelopes, not manufacturer models.
use crate::{Board, Copper};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Vertex3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}
#[derive(Debug, Clone)]
pub struct MeshObject {
    pub name: String,
    pub material: usize,
    pub triangles: Vec<[Vertex3; 3]>,
}
#[derive(Debug, Clone)]
pub struct BoardMesh {
    pub objects: Vec<MeshObject>,
}
pub const MESH_COLORS: [[u8; 3]; 5] = [
    [32, 105, 77],
    [255, 185, 105],
    [104, 179, 255],
    [240, 204, 102],
    [53, 64, 78],
];
fn box_mesh(name: String, material: usize, min: Vertex3, max: Vertex3) -> MeshObject {
    let v = [
        Vertex3::new(min.x, min.y, min.z),
        Vertex3::new(max.x, min.y, min.z),
        Vertex3::new(max.x, max.y, min.z),
        Vertex3::new(min.x, max.y, min.z),
        Vertex3::new(min.x, min.y, max.z),
        Vertex3::new(max.x, min.y, max.z),
        Vertex3::new(max.x, max.y, max.z),
        Vertex3::new(min.x, max.y, max.z),
    ];
    let mut triangles = vec![];
    for [a, b, c, d] in [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ] {
        triangles.extend([[v[a], v[b], v[c]], [v[a], v[c], v[d]]]);
    }
    MeshObject {
        name,
        material,
        triangles,
    }
}
fn ring(
    name: String,
    material: usize,
    x: f64,
    y: f64,
    radius: f64,
    hole: f64,
    z0: f64,
    z1: f64,
) -> MeshObject {
    let mut triangles = vec![];
    for i in 0..24 {
        let a = i as f64 * std::f64::consts::TAU / 24.;
        let b = (i + 1) as f64 * std::f64::consts::TAU / 24.;
        let p = |r: f64, t: f64, z: f64| Vertex3::new(x + r * t.cos(), y + r * t.sin(), z);
        for [u, v, w, q] in [
            [
                p(radius, a, z1),
                p(radius, b, z1),
                p(hole, b, z1),
                p(hole, a, z1),
            ],
            [
                p(radius, b, z0),
                p(radius, a, z0),
                p(hole, a, z0),
                p(hole, b, z0),
            ],
            [
                p(radius, a, z0),
                p(radius, b, z0),
                p(radius, b, z1),
                p(radius, a, z1),
            ],
            [
                p(hole, b, z0),
                p(hole, a, z0),
                p(hole, a, z1),
                p(hole, b, z1),
            ],
        ] {
            triangles.extend([[u, v, w], [u, w, q]]);
        }
    }
    MeshObject {
        name,
        material,
        triangles,
    }
}
impl Board {
    pub fn mesh(&self, components: bool) -> Result<BoardMesh, String> {
        self.validate()?;
        let t = self.thickness;
        let mut objects = vec![box_mesh(
            "substrate".into(),
            0,
            Vertex3::new(0., 0., 0.),
            Vertex3::new(self.width, self.height, t),
        )];
        for track in &self.tracks {
            let z = if track.layer == Copper::Top {
                t + 0.04
            } else {
                -0.06
            };
            for (i, ab) in track.points.windows(2).enumerate() {
                let (dx, dy) = (ab[1].x - ab[0].x, ab[1].y - ab[0].y);
                let len = dx.hypot(dy);
                if len < 1e-9 {
                    continue;
                }
                let (nx, ny) = (-dy / len * track.width / 2., dx / len * track.width / 2.);
                let mut obj = box_mesh(
                    format!("track_{}_{}", track.id, i),
                    if track.layer == Copper::Top { 1 } else { 2 },
                    Vertex3::new(0., -1., z),
                    Vertex3::new(1., 1., z + 0.035),
                );
                for tri in &mut obj.triangles {
                    for v in tri {
                        let f = v.x;
                        let n = v.y;
                        v.x = ab[0].x + f * dx + n * nx;
                        v.y = ab[0].y + f * dy + n * ny;
                    }
                }
                objects.push(obj);
            }
        }
        for p in self.pads() {
            for (side, z) in if p.drill > 0. {
                vec![("top", t + 0.04), ("bottom", -0.075)]
            } else {
                vec![("top", t + 0.04)]
            } {
                let name = format!("pad_{}_{}_{}", p.component_id, p.number, side);
                objects.push(if p.rectangular {
                    box_mesh(
                        name,
                        3,
                        Vertex3::new(p.at.x - p.size.x / 2., p.at.y - p.size.y / 2., z),
                        Vertex3::new(p.at.x + p.size.x / 2., p.at.y + p.size.y / 2., z + 0.035),
                    )
                } else {
                    ring(
                        name,
                        3,
                        p.at.x,
                        p.at.y,
                        p.size.x / 2.,
                        p.drill / 2.,
                        z,
                        z + 0.035,
                    )
                });
            }
        }
        for v in &self.vias {
            objects.push(ring(
                format!("via_{}", v.id),
                3,
                v.at.x,
                v.at.y,
                v.diameter / 2.,
                v.drill / 2.,
                -0.075,
                t + 0.075,
            ));
        }
        if components {
            for f in &self.footprints {
                let pads = f.pads();
                if pads.is_empty() {
                    continue;
                }
                let x0 = pads.iter().map(|p| p.at.x).fold(f64::INFINITY, f64::min) - 0.5;
                let y0 = pads.iter().map(|p| p.at.y).fold(f64::INFINITY, f64::min) - 0.7;
                let x1 = pads
                    .iter()
                    .map(|p| p.at.x)
                    .fold(f64::NEG_INFINITY, f64::max)
                    + 0.5;
                let y1 = pads
                    .iter()
                    .map(|p| p.at.y)
                    .fold(f64::NEG_INFINITY, f64::max)
                    + 0.7;
                let height = if f.style.starts_with("SMD") { 0.8 } else { 3. };
                objects.push(box_mesh(
                    format!("body_{}", f.reference),
                    4,
                    Vertex3::new(x0, y0, t + 0.2),
                    Vertex3::new(x1, y1, t + 0.2 + height),
                ));
            }
        }
        Ok(BoardMesh { objects })
    }
}
impl BoardMesh {
    pub fn obj(&self) -> String {
        let mut out=String::from("# CircuitCraft PCB review mesh. Units: millimetres. Component bodies are approximate.\n");
        let mut index = 1;
        for o in &self.objects {
            out.push_str(&format!(
                "o {}\n",
                o.name
                    .replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_")
            ));
            for tri in &o.triangles {
                for p in tri {
                    out.push_str(&format!("v {:.6} {:.6} {:.6}\n", p.x, p.y, p.z));
                }
                out.push_str(&format!("f {index} {} {}\n", index + 1, index + 2));
                index += 3;
            }
        }
        out
    }
}
