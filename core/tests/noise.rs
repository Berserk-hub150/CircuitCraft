use circuitcraft_core::*;
use std::path::Path;
#[test]
fn johnson_noise_matches_thermal_formula() {
    if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_err() {
        return;
    }
    let s = Sheet::rc();
    let sim = run_spice(
        &format!("{}\n.temp 27\n", netlist(&s).unwrap()),
        &Analysis::Noise {
            output: "VOUT".into(),
            source: "V1".into(),
            points_per_decade: 100,
            start: 1.,
            stop: 100000.,
        },
        Path::new("ngspice"),
        None,
    )
    .unwrap();
    let t = sim.trace("onoise_spectrum").unwrap();
    let expected = (4f64 * 1.380649e-23 * 300.15 * 1000.).sqrt()
        / (1. + (2. * std::f64::consts::PI * 1e-3 * sim.axis[0]).powi(2)).sqrt();
    assert!(
        (t.real[0] / expected - 1.).abs() < 1e-4,
        "{} != {expected}",
        t.real[0]
    );
    assert_eq!(t.unit, "voltage-density");
    assert!(measurements(&sim, t)
        .iter()
        .any(|m| m.name.starts_with("Integrated noise")));
}
