use crate::{csv_cell, netlist, parse_value, run_spice, Analysis, Kind, Sheet, Simulation};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
#[derive(Debug, Clone)]
pub struct SweepResult {
    pub reference: String,
    pub runs: Vec<(f64, Simulation)>,
}
impl SweepResult {
    pub fn measurements_csv(&self) -> String {
        let mut out = "Component,ComponentValue,Signal,Measurement,Value,Unit\n".to_string();
        for (v, s) in &self.runs {
            for t in &s.traces {
                for m in crate::measurements(s, t) {
                    out.push_str(&format!(
                        "{},{v:.12e},{},{},{:.12e},{}\n",
                        csv_cell(&self.reference),
                        csv_cell(&t.name),
                        csv_cell(&m.name),
                        m.value,
                        csv_cell(&m.unit)
                    ));
                }
            }
        }
        out
    }
    /// Long-format CSV keeps each run's complete, possibly adaptive, axis.
    pub fn csv(&self) -> String {
        let mut out = "Component,Value,Axis,Signal,Real,Imaginary,Unit\n".to_string();
        for (v, sim) in &self.runs {
            for (i, x) in sim.axis.iter().enumerate() {
                for t in &sim.traces {
                    out.push_str(&format!(
                        "{},{:.12e},{:.12e},{},{:.12e},{:.12e},{}\n",
                        csv_cell(&self.reference),
                        v,
                        x,
                        csv_cell(&t.name),
                        t.real[i],
                        t.imag[i],
                        csv_cell(&t.unit)
                    ));
                }
            }
        }
        out
    }
    /// Overlay on the first run's axis, with linear interpolation for adaptive
    /// transient timesteps. Original data remain available in csv().
    pub fn overlay(&self, signal: &str) -> Result<Simulation, String> {
        let first = &self.runs.first().ok_or("No sweep results")?.1;
        let mut sim = first.clone();
        sim.traces.clear();
        sim.title = format!(
            "Parameter sweep · {} · {} runs",
            self.reference,
            self.runs.len()
        );
        sim.elapsed = self.runs.iter().map(|(_, s)| s.elapsed).sum();
        sim.log = self
            .runs
            .iter()
            .map(|(v, s)| format!("{}={v:e}\n{}", self.reference, s.log))
            .collect::<Vec<_>>()
            .join("\n");
        for (v, s) in &self.runs {
            let mut t = s
                .trace(signal)
                .ok_or(format!("Signal '{signal}' unavailable in sweep."))?
                .clone();
            t.name = format!("{signal} | {}={v:e}", self.reference);
            if s.axis != first.axis {
                if s.axis.len() < 2 {
                    return Err("Sweep axes do not match.".into());
                }
                if s.axis.windows(2).any(|ab| ab[1] <= ab[0]) {
                    return Err(
                        "Overlay requires increasing axes or identical sample points.".into(),
                    );
                }
                let interp = |data: &[f64], x: f64| -> Result<f64, String> {
                    if x < s.axis[0] - 1e-12 || x > s.axis[s.axis.len() - 1] + 1e-12 {
                        return Err("Sweep runs have different axis extents.".into());
                    }
                    let i = s
                        .axis
                        .partition_point(|v| *v < x)
                        .clamp(1, s.axis.len() - 1);
                    let f = ((x - s.axis[i - 1]) / (s.axis[i] - s.axis[i - 1])).clamp(0., 1.);
                    Ok(data[i - 1] + f * (data[i] - data[i - 1]))
                };
                t.real = first
                    .axis
                    .iter()
                    .map(|x| interp(&t.real, *x))
                    .collect::<Result<_, _>>()?;
                t.imag = first
                    .axis
                    .iter()
                    .map(|x| interp(&t.imag, *x))
                    .collect::<Result<_, _>>()?;
            }
            sim.traces.push(t);
        }
        Ok(sim)
    }
}
pub fn sweep_values(s: &str) -> Result<Vec<f64>, String> {
    let values = s
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(parse_value)
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() || values.len() > 24 || values.iter().any(|v| *v <= 0.) {
        return Err("Sweep: provide 1–24 positive R/C/L values, e.g. 500,1k,2k.".into());
    }
    Ok(values)
}
pub fn run_sweep(
    sheet: &Sheet,
    reference: &str,
    values: &[f64],
    analysis: &Analysis,
    engine: &Path,
    base: Option<&Path>,
    cancel: Arc<AtomicBool>,
) -> Result<SweepResult, String> {
    if sheet.raw_mode {
        return Err(
            "Component sweeps use the graphical schematic. Disable edited-netlist mode.".into(),
        );
    }
    if values.is_empty() || values.len() > 24 || values.iter().any(|v| !v.is_finite() || *v <= 0.) {
        return Err("Sweep requires 1–24 positive, finite values.".into());
    }
    let index = sheet
        .components
        .iter()
        .position(|c| c.reference.eq_ignore_ascii_case(reference))
        .ok_or("Sweep component not found")?;
    if !matches!(
        sheet.components[index].kind,
        Kind::Resistor | Kind::Capacitor | Kind::Inductor
    ) {
        return Err("Component sweeps support resistors, capacitors and inductors.".into());
    }
    let mut result = SweepResult {
        reference: sheet.components[index].reference.clone(),
        runs: vec![],
    };
    for value in values {
        if cancel.load(Ordering::Relaxed) {
            return Err("Sweep cancelled between runs.".into());
        }
        let mut s = sheet.clone();
        s.components[index].value = format!("{value:e}");
        let sim = run_spice(&netlist(&s)?, analysis, engine, base)
            .map_err(|e| format!("{}={value:e}: {e}", result.reference))?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Sweep cancelled between runs.".into());
        }
        result.runs.push((*value, sim));
        let total = result
            .runs
            .iter()
            .map(|(_, s)| s.axis.len().saturating_mul(s.traces.len()))
            .sum::<usize>();
        if total > 5_000_000 {
            return Err("Sweep exceeds 5 million signal samples; reduce the number of values or time samples.".into());
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sweep_validation_and_physical_rc_cutoffs() {
        assert_eq!(
            sweep_values("500, 1k; 2k").unwrap(),
            vec![500., 1000., 2000.]
        );
        assert!(sweep_values("0,1k").is_err());
        if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_err() {
            return;
        }
        let s = Sheet::rc();
        let original = s.clone();
        let r = run_sweep(
            &s,
            "R1",
            &[500., 1000., 2000.],
            &Analysis::Ac {
                points_per_decade: 100,
                start: 1.,
                stop: 100000.,
            },
            Path::new("ngspice"),
            None,
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert_eq!(s, original);
        for (res, sim) in &r.runs {
            let m = crate::measurements(sim, sim.trace("v(vout)").unwrap());
            let cutoff = m
                .iter()
                .find(|m| m.name.starts_with("First −3"))
                .unwrap()
                .value;
            let expected = 1. / (2. * std::f64::consts::PI * res * 1e-6);
            assert!(
                (cutoff / expected - 1.).abs() < 0.001,
                "{cutoff} {expected}"
            );
        }
        assert_eq!(r.overlay("v(vout)").unwrap().traces.len(), 3);
        assert!(r.csv().contains("R1"));
        assert!(run_sweep(
            &s,
            "R1",
            &[1000.],
            &Analysis::OperatingPoint,
            Path::new("ngspice"),
            None,
            Arc::new(AtomicBool::new(true))
        )
        .is_err());
    }
}
