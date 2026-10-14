use circuitcraft_core::*;
use std::sync::atomic::AtomicBool;

fn endpoint(id: u64, x: f64, y: f64) -> Footprint {
    Footprint {
        component_id: id,
        reference: format!("J{id}"),
        style: "ONE-PAD".into(),
        at: Mm::new(x, y),
        rotation: 0,
        nets: vec!["SIGNAL".into()],
        custom_pads: vec![PadSpec {
            at: Mm::default(),
            size: Mm::new(1., 1.),
            rectangular: true,
            drill: 0.,
        }],
    }
}
fn blocked_board() -> Board {
    let mut b = Board {
        width: 30.,
        height: 20.,
        ..Default::default()
    };
    b.footprints = vec![endpoint(1, 3.13, 10.27), endpoint(2, 26.17, 10.32)];
    b.tracks.push(Track {
        id: 50,
        net: "BLOCK".into(),
        layer: Copper::Top,
        width: 1.,
        points: vec![Mm::new(15., 1.), Mm::new(15., 19.)],
    });
    b
}
#[test]
fn router_crosses_top_barrier_using_vias_and_preserves_copper() {
    let mut b = blocked_board();
    let original = b.tracks[0].clone();
    let r = b.autoroute(&RouteOptions::default(), 100).unwrap();
    assert_eq!(r.remaining, 0, "{r:?}");
    assert!(r.added_vias >= 2);
    assert_eq!(b.tracks[0], original);
    assert!(b.drc().is_empty(), "{:?}", b.drc());
    assert!(b.tracks.iter().any(|t| t.layer == Copper::Bottom));
    assert!(b
        .tracks
        .iter()
        .flat_map(|t| t.points.iter())
        .any(|p| p.distance(Mm::new(3.13, 10.27)) < 1e-9));
}
#[test]
fn blocked_or_cancelled_router_never_creates_invalid_copper() {
    let mut b = blocked_board();
    let original = b.clone();
    let r = b
        .autoroute(
            &RouteOptions {
                allow_vias: false,
                ..Default::default()
            },
            100,
        )
        .unwrap();
    assert_eq!(r.remaining, 1);
    assert_eq!(b, original);
    assert!(b
        .autoroute_cancel(&RouteOptions::default(), 100, &AtomicBool::new(true))
        .is_err());
    assert_eq!(b, original);
    assert!(b
        .autoroute(
            &RouteOptions {
                grid: f64::NAN,
                ..Default::default()
            },
            100
        )
        .is_err());
    b.footprints[0].at.x = 0.;
    let outside = b.clone();
    assert!(b.autoroute(&RouteOptions::default(), 100).is_err());
    assert_eq!(b, outside);
}
#[test]
fn router_routes_reference_boards_and_saved_ids_remain_unique() {
    for mut s in [
        Sheet::rc(),
        Sheet::divider(),
        Sheet::rectifier(),
        hierarchical_divider(),
    ] {
        s.board.tracks.clear();
        s.board.vias.clear();
        let r = s.autoroute(&RouteOptions::default()).unwrap();
        assert_eq!(r.remaining, 0, "{}: {r:?}", s.title);
        assert!(s.pcb_drc().is_empty(), "{}: {:?}", s.title, s.pcb_drc());
        assert_eq!(s, Sheet::from_json(&s.to_json().unwrap()).unwrap());
    }
}
#[test]
fn hierarchical_ports_flatten_deterministically_and_simulate() {
    let mut s = hierarchical_divider();
    let f = flatten(&s).unwrap();
    assert_eq!(f, flatten(&s).unwrap());
    assert_eq!(f.components.len(), 3);
    assert!(netlist(&s).unwrap().contains("R_LOAD_R1 OUT 0"));
    let sim = linear_dc(&s).unwrap();
    assert!((sim.trace("v(OUT)").unwrap().real[0] - 5.).abs() < 1e-10);
    let id = f.components[2].id;
    let mut other = s.children[0].clone();
    other.instance = "OTHER".into();
    s.children.insert(0, other);
    let f = flatten(&s).unwrap();
    assert_eq!(
        f.components
            .iter()
            .find(|c| c.reference == "R_LOAD_R1")
            .unwrap()
            .id,
        id
    );
    assert!((linear_dc(&s).unwrap().trace("v(OUT)").unwrap().real[0] - 10. / 3.).abs() < 1e-10);
    s.try_sync_board().unwrap();
    assert_eq!(s.board.footprints.len(), 4);
    assert_eq!(s, Sheet::from_json(&s.to_json().unwrap()).unwrap());
    if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_ok() {
        let sim = run_spice(
            &netlist(&s).unwrap(),
            &Analysis::OperatingPoint,
            std::path::Path::new("ngspice"),
            None,
        )
        .unwrap();
        assert!((sim.trace("v(out)").unwrap().real[0] - 10. / 3.).abs() < 1e-8);
    }
}
#[test]
fn invalid_ports_and_hierarchy_limits_are_rejected() {
    let mut s = hierarchical_divider();
    s.children[0].ports[0].label = "MISSING".into();
    assert!(flatten(&s).is_err());
    s.children[0].ports[0].label = "0".into();
    assert!(flatten(&s).is_err());
    let mut duplicate = s.children[0].clone();
    duplicate.instance = "load".into();
    s.children.push(duplicate);
    assert!(validate_hierarchy(&s).is_err());
    let mut deep = Sheet::default();
    for _ in 0..9 {
        let mut p = Sheet::default();
        p.children.push(ChildSheet {
            instance: "H1".into(),
            ports: vec![],
            sheet: Box::new(deep),
        });
        deep = p;
    }
    assert!(Sheet::from_json(&deep.to_json().unwrap()).is_err());
}
#[test]
fn private_labels_do_not_connect_sibling_pages() {
    let mut s = hierarchical_divider();
    s.children[0].ports.clear();
    let mut other = s.children[0].clone();
    other.instance = "SECOND".into();
    s.children.push(other);
    let f = flatten(&s).unwrap();
    let n = connect(&f);
    assert_ne!(
        n.pin_net[&(f.components[2].id, 0)],
        n.pin_net[&(f.components[3].id, 0)]
    );
    assert_eq!(n.pin_net[&(f.components[2].id, 1)], "0");
}
#[test]
fn mesh_matches_rotated_pad_track_layer_and_obj_indices() {
    let s = Sheet::rc();
    let mut b = s.board.clone();
    b.thickness = 2.;
    b.footprints[1].rotation = 1;
    let mesh = b.mesh(true).unwrap();
    assert!(mesh.objects.iter().any(|o| o.name.starts_with("body_")));
    assert!(mesh.objects.iter().any(|o| o.material == 2));
    let substrate = &mesh.objects[0];
    assert_eq!(
        substrate
            .triangles
            .iter()
            .flatten()
            .map(|v| v.z)
            .fold(0., f64::max),
        2.
    );
    let pad = &b.footprints[1].pads()[0];
    let object = mesh
        .objects
        .iter()
        .find(|o| o.name == format!("pad_{}_1_top", pad.component_id))
        .unwrap();
    let x0 = object
        .triangles
        .iter()
        .flatten()
        .map(|v| v.x)
        .fold(f64::INFINITY, f64::min);
    assert!((x0 - (pad.at.x - pad.size.x / 2.)).abs() < 1e-9);
    let obj = mesh.obj();
    let vertices = obj.lines().filter(|s| s.starts_with("v ")).count();
    for line in obj.lines().filter(|s| s.starts_with("f ")) {
        for index in line.split_whitespace().skip(1) {
            assert!((1..=vertices).contains(&index.parse().unwrap()));
        }
    }
    assert!(!b
        .mesh(false)
        .unwrap()
        .objects
        .iter()
        .any(|o| o.name.starts_with("body_")));
    let old = s.to_json().unwrap().replace("\"thickness\": 1.6,", "");
    assert_eq!(Sheet::from_json(&old).unwrap().board.thickness, 1.6);
    let old = old.replace("\"format_version\": 2", "\"format_version\": 1");
    assert_eq!(Sheet::from_json(&old).unwrap().format_version, 2);
}
