use crate::{csv_cell, Simulation, Trace};

#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: String,
    pub value: f64,
    pub unit: String,
}
pub fn trace_unit(t: &Trace) -> &str {
    match t.unit.as_str() {
        "voltage" => "V",
        "current" => "A",
        "voltage-density" => "V/√Hz",
        "current-density" => "A/√Hz",
        "power" => "W",
        "ratio" => "1",
        u => u,
    }
}
/// Statistics on the complete trace. Time averages integrate the piecewise
/// linear waveform, so adaptive ngspice timesteps do not bias mean and RMS.
pub fn measurements(sim: &Simulation, t: &Trace) -> Vec<Measurement> {
    let mut out = vec![];
    let n = sim.axis.len();
    if n == 0 || t.real.len() != n || t.imag.len() != n {
        return out;
    }
    let unit = trace_unit(t);
    let mut add = |name: &str, value: f64, unit: &str| {
        if value.is_finite() {
            out.push(Measurement {
                name: name.into(),
                value,
                unit: unit.into(),
            });
        }
    };
    let min = t.real.iter().copied().fold(f64::INFINITY, f64::min);
    let max = t.real.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if sim.complex {
        let (peak_index, peak) = (0..n)
            .map(|i| (i, t.magnitude(i)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        add("Peak magnitude", peak, unit);
        add("Peak frequency", sim.axis[peak_index], "Hz");
        add("Magnitude at first frequency", t.magnitude(0), unit);
        add("Phase at first frequency", t.phase(0), "°");
        let threshold = t.magnitude(0) / 2f64.sqrt();
        if sim.axis_name == "frequency" && threshold > 0. {
            for i in 1..n {
                let a = t.magnitude(i - 1);
                let b = t.magnitude(i);
                if a > threshold && b <= threshold && b > 0. && sim.axis[i - 1] > 0. {
                    let fraction = (threshold.ln() - a.ln()) / (b.ln() - a.ln());
                    let hz = (sim.axis[i - 1].ln()
                        + fraction * (sim.axis[i].ln() - sim.axis[i - 1].ln()))
                    .exp();
                    add("First −3.0103 dB crossing (relative to first)", hz, "Hz");
                    break;
                }
            }
        }
    } else {
        add("Minimum", min, unit);
        add("Maximum", max, unit);
        add("Peak to peak", max - min, unit);
        if sim.axis_name == "frequency"
            && t.unit.ends_with("-density")
            && n > 1
            && sim.axis.windows(2).all(|ab| ab[1] > ab[0])
        {
            let variance = (1..n)
                .map(|i| {
                    (sim.axis[i] - sim.axis[i - 1])
                        * (t.real[i - 1] * t.real[i - 1]
                            + t.real[i - 1] * t.real[i]
                            + t.real[i] * t.real[i])
                        / 3.
                })
                .sum::<f64>();
            add(
                "Integrated noise over sampled band",
                variance.max(0.).sqrt(),
                if t.unit == "current-density" {
                    "A RMS"
                } else {
                    "V RMS"
                },
            );
        }
        let time = sim.axis_name == "time" && n > 1 && sim.axis.windows(2).all(|ab| ab[1] > ab[0]);
        let mut sum = 0.;
        let mut square = 0.;
        if time {
            for i in 1..n {
                let dt = sim.axis[i] - sim.axis[i - 1];
                let (a, b) = (t.real[i - 1], t.real[i]);
                sum += dt * (a + b) / 2.;
                square += dt * (a * a + a * b + b * b) / 3.;
            }
            let duration = sim.axis[n - 1] - sim.axis[0];
            add("Time weighted mean", sum / duration, unit);
            add(
                "Time weighted RMS",
                (square / duration).max(0.).sqrt(),
                unit,
            );
            add("Integral", sum, &format!("{unit}·s"));
            let first = t.real[0];
            let last = t.real[n - 1];
            let delta = last - first;
            if delta.abs() > 1e-15 {
                let crossing = |fraction: f64| -> Option<f64> {
                    let level = first + delta * fraction;
                    t.real.windows(2).enumerate().find_map(|(i, ab)| {
                        if (ab[0] - level) * delta <= 0.
                            && (ab[1] - level) * delta >= 0.
                            && ab[0] != ab[1]
                        {
                            Some(
                                sim.axis[i]
                                    + (level - ab[0]) / (ab[1] - ab[0])
                                        * (sim.axis[i + 1] - sim.axis[i]),
                            )
                        } else {
                            None
                        }
                    })
                };
                if let (Some(a), Some(b)) = (crossing(0.1), crossing(0.9)) {
                    add("10–90% transition (first to final sample)", b - a, "s");
                }
                let band = delta.abs() * 0.02;
                if let Some(i) = t.real.iter().rposition(|v| (v - last).abs() > band) {
                    if i + 1 < n {
                        let a = (t.real[i] - last).abs();
                        let b = (t.real[i + 1] - last).abs();
                        let x = sim.axis[i]
                            + ((a - band) / (a - b)).clamp(0., 1.)
                                * (sim.axis[i + 1] - sim.axis[i]);
                        add("2% settling to final sample", x - sim.axis[0], "s");
                    }
                }
            }
        } else {
            add("Sample mean", t.real.iter().sum::<f64>() / n as f64, unit);
            add(
                "Sample RMS",
                (t.real.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt(),
                unit,
            );
        }
    }
    out
}
pub fn measurements_csv(sim: &Simulation) -> String {
    let mut out = "Signal,Measurement,Value,Unit\n".to_string();
    for t in &sim.traces {
        for m in measurements(sim, t) {
            out.push_str(&format!(
                "{},{},{:.12e},{}\n",
                csv_cell(&t.name),
                csv_cell(&m.name),
                m.value,
                csv_cell(&m.unit)
            ));
        }
    }
    out
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraceMath {
    Difference,
    Ratio,
    Product,
}
/// Complex arithmetic preserves phase; a division by zero is an error,
/// rather than silently creating infinities in a gain curve.
pub fn derived_trace(sim: &Simulation, a: &str, b: &str, op: TraceMath) -> Result<Trace, String> {
    let a = sim.trace(a).ok_or("First signal not found")?;
    let b = sim.trace(b).ok_or("Second signal not found")?;
    if [a.real.len(), a.imag.len(), b.real.len(), b.imag.len()]
        .iter()
        .any(|n| *n != sim.axis.len())
    {
        return Err("Signal sample count differs from its axis.".into());
    }
    if op == TraceMath::Difference && a.unit != b.unit {
        return Err("Difference requires matching signal units.".into());
    }
    let symbol = match op {
        TraceMath::Difference => "−",
        TraceMath::Ratio => "/",
        TraceMath::Product => "×",
    };
    let unit = match op {
        TraceMath::Difference => a.unit.clone(),
        TraceMath::Ratio if a.unit == b.unit => "ratio".into(),
        TraceMath::Ratio => format!("{}/{}", trace_unit(a), trace_unit(b)),
        TraceMath::Product
            if (a.unit == "voltage" && b.unit == "current")
                || (a.unit == "current" && b.unit == "voltage") =>
        {
            "power".into()
        }
        TraceMath::Product => format!("{}·{}", trace_unit(a), trace_unit(b)),
    };
    let mut t = Trace {
        name: format!("({} {symbol} {})", a.name, b.name),
        unit,
        real: vec![],
        imag: vec![],
    };
    for i in 0..sim.axis.len() {
        let (ar, ai, br, bi) = (a.real[i], a.imag[i], b.real[i], b.imag[i]);
        let (re, im) = match op {
            TraceMath::Difference => (ar - br, ai - bi),
            TraceMath::Product => (ar * br - ai * bi, ar * bi + ai * br),
            TraceMath::Ratio => {
                let d = br * br + bi * bi;
                if d <= 1e-300 {
                    return Err(format!("Division by zero at sample {i}."));
                }
                ((ar * br + ai * bi) / d, (ai * br - ar * bi) / d)
            }
        };
        if !re.is_finite() || !im.is_finite() {
            return Err("Non-finite derived signal.".into());
        }
        t.real.push(re);
        t.imag.push(im);
    }
    Ok(t)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_time_statistics_are_integrals() {
        let sim = Simulation {
            title: "Ramp".into(),
            axis_name: "time".into(),
            axis: vec![0., 0.01, 1.],
            traces: vec![],
            complex: false,
            log: String::new(),
            elapsed: 0.,
        };
        let t = Trace {
            name: "ramp".into(),
            unit: "voltage".into(),
            real: sim.axis.clone(),
            imag: vec![0.; 3],
        };
        let m = measurements(&sim, &t);
        let get = |s: &str| m.iter().find(|m| m.name == s).unwrap().value;
        assert!((get("Time weighted mean") - 0.5).abs() < 1e-12);
        assert!((get("Time weighted RMS") - (1f64 / 3.).sqrt()).abs() < 1e-12);
        assert!((get("10–90% transition (first to final sample)") - 0.8).abs() < 1e-12);
        assert!((get("2% settling to final sample") - 0.98).abs() < 1e-12);
    }
    #[test]
    fn complex_gain_and_zero_denominator() {
        let a = Trace {
            name: "a".into(),
            unit: "voltage".into(),
            real: vec![1.],
            imag: vec![1.],
        };
        let b = Trace {
            name: "b".into(),
            unit: "voltage".into(),
            real: vec![1.],
            imag: vec![-1.],
        };
        let mut sim = Simulation {
            title: String::new(),
            axis_name: "frequency".into(),
            axis: vec![1.],
            traces: vec![a, b],
            complex: true,
            log: String::new(),
            elapsed: 0.,
        };
        let t = derived_trace(&sim, "a", "b", TraceMath::Ratio).unwrap();
        assert!(t.real[0].abs() < 1e-12 && (t.imag[0] - 1.).abs() < 1e-12);
        sim.traces[1].real[0] = 0.;
        sim.traces[1].imag[0] = 0.;
        assert!(derived_trace(&sim, "a", "b", TraceMath::Ratio).is_err());
    }
}
