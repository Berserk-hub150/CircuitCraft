use crate::{connect, valid_node, Kind, Sheet};
use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
pub const MODELS: &str = r#".model DGEN D(IS=2.52n RS=0.568 N=1.73 CJO=4p BV=100 IBV=100u)
.model DLED D(IS=1e-20 N=2 RS=10)
.model QGEN NPN(IS=1e-14 BF=150 VAF=100)
.model QPGEN PNP(IS=1e-14 BF=100 VAF=100)
.model NGEN NMOS(LEVEL=1 VTO=2 KP=2m LAMBDA=0.02)
.model PGEN PMOS(LEVEL=1 VTO=-2 KP=1m LAMBDA=0.02)
.model SWGEN SW(RON=1 ROFF=1e9 VT=2.5 VH=0.1)"#;
pub fn parse_value(s: &str) -> Result<f64, String> {
    let t = s.trim().to_ascii_lowercase();
    let end = t
        .char_indices()
        .find_map(|(i, c)| {
            if c.is_ascii_digit() || matches!(c, '.' | '+' | '-' | 'e') {
                None
            } else {
                Some(i)
            }
        })
        .unwrap_or(t.len());
    let (n, suffix) = t.split_at(end);
    let value = n
        .parse::<f64>()
        .map_err(|_| format!("Invalid value '{s}'."))?;
    let scale = if suffix.starts_with("meg") {
        1e6
    } else {
        match suffix.chars().next() {
            None => 1.,
            Some('t') => 1e12,
            Some('g') => 1e9,
            Some('k') => 1e3,
            Some('m') => 1e-3,
            Some('u') => 1e-6,
            Some('n') => 1e-9,
            Some('p') => 1e-12,
            Some('f') => 1e-15,
            _ => return Err(format!("Invalid suffix '{suffix}'.")),
        }
    };
    let value = value * scale;
    if !value.is_finite() {
        return Err("Non-finite value.".into());
    }
    Ok(value)
}
pub fn netlist(s: &Sheet) -> Result<String, String> {
    if !s.children.is_empty() {
        return netlist(&crate::flatten(s)?);
    }
    let n = connect(s);
    let errors = n
        .issues
        .iter()
        .filter(|i| i.error)
        .map(|i| i.message.as_str())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let mut out = format!(
        "CircuitCraft - {}\n* Generated from electrical connectivity\n",
        s.title.replace(['\n', '\r'], " ")
    );
    for c in &s.components {
        let pins = (0..c.pins().len())
            .map(|p| n.pin_net[&(c.id, p)].clone())
            .collect::<Vec<_>>()
            .join(" ");
        if c.kind == Kind::Connector {
            out.push_str(&format!("* {} connector: {pins}\n", c.reference));
        } else {
            out.push_str(&format!(
                "{} {pins} {} {}\n",
                c.reference, c.value, c.params
            ));
        }
    }
    out.push_str("\n* Generic demonstration models; replace with vendor models for design\n");
    out.push_str(MODELS);
    out.push('\n');
    out.push_str(&s.spice_models);
    out.push('\n');
    Ok(out)
}
#[derive(Debug, Clone, PartialEq)]
pub enum Analysis {
    OperatingPoint,
    Transient {
        step: f64,
        stop: f64,
        uic: bool,
    },
    Ac {
        points_per_decade: u32,
        start: f64,
        stop: f64,
    },
    DcSweep {
        source: String,
        start: f64,
        stop: f64,
        step: f64,
    },
    Noise {
        output: String,
        source: String,
        points_per_decade: u32,
        start: f64,
        stop: f64,
    },
}
impl Analysis {
    fn command(&self) -> Result<String, String> {
        let positive = |v: f64| v.is_finite() && v > 0.;
        Ok(match self {
            Self::OperatingPoint => "op".into(),
            Self::Transient { step, stop, uic } => {
                if !positive(*step) || !positive(*stop) || step > stop || stop / step > 200000. {
                    return Err(
                        "Transient: positive step ≤ stop, at most 200,000 intervals.".into(),
                    );
                }
                format!(
                    "tran {step:e} {stop:e} 0 {step:e}{}",
                    if *uic { " uic" } else { "" }
                )
            }
            Self::Ac {
                points_per_decade,
                start,
                stop,
            } => {
                if !positive(*start)
                    || !positive(*stop)
                    || stop <= start
                    || *points_per_decade == 0
                    || *points_per_decade > 1000
                    || (*stop / *start).log10() * (*points_per_decade as f64) > 200000.
                {
                    return Err("AC: positive start < stop, 1–1,000 points per decade.".into());
                }
                format!("ac dec {points_per_decade} {start:e} {stop:e}")
            }
            Self::DcSweep {
                source,
                start,
                stop,
                step,
            } => {
                if !valid_node(source)
                    || !matches!(source.chars().next(), Some('V' | 'v' | 'I' | 'i'))
                    || !start.is_finite()
                    || !stop.is_finite()
                    || !step.is_finite()
                    || *step == 0.
                    || (*stop - *start) / *step <= 0.
                    || (*stop - *start) / *step > 200000.
                {
                    return Err(
                        "DC: choose a V/I source and a step towards stop; ≤200,000 intervals."
                            .into(),
                    );
                }
                format!("dc {source} {start:e} {stop:e} {step:e}")
            }
            Self::Noise {
                output,
                source,
                points_per_decade,
                start,
                stop,
            } => {
                if !valid_node(output)
                    || !valid_node(source)
                    || !matches!(source.chars().next(), Some('V' | 'v' | 'I' | 'i'))
                {
                    return Err("Noise: choose an output node and a V/I input source.".into());
                }
                Self::Ac {
                    points_per_decade: *points_per_decade,
                    start: *start,
                    stop: *stop,
                }
                .command()?;
                format!("noise v({output}) {source} dec {points_per_decade} {start:e} {stop:e}\nsetplot noise1")
            }
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub name: String,
    pub unit: String,
    pub real: Vec<f64>,
    pub imag: Vec<f64>,
}
impl Trace {
    pub fn magnitude(&self, i: usize) -> f64 {
        self.real[i].hypot(self.imag[i])
    }
    pub fn db(&self, i: usize) -> f64 {
        20. * self.magnitude(i).max(1e-300).log10()
    }
    pub fn phase(&self, i: usize) -> f64 {
        self.imag[i].atan2(self.real[i]).to_degrees()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Simulation {
    pub title: String,
    pub axis_name: String,
    pub axis: Vec<f64>,
    pub traces: Vec<Trace>,
    pub complex: bool,
    pub log: String,
    pub elapsed: f64,
}
impl Simulation {
    pub fn trace(&self, name: &str) -> Option<&Trace> {
        self.traces
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }
    pub fn csv(&self) -> String {
        let mut out = csv_cell(&self.axis_name);
        for t in &self.traces {
            out.push_str(&format!(",{}", csv_cell(&t.name)));
            if self.complex {
                out.push_str(&format!(
                    ",{},{}",
                    csv_cell(&format!("{} imaginary", t.name)),
                    csv_cell(&format!("{} magnitude", t.name))
                ));
            }
        }
        out.push('\n');
        for i in 0..self.axis.len() {
            out.push_str(&format!("{:.12e}", self.axis[i]));
            for t in &self.traces {
                out.push_str(&format!(",{:.12e}", t.real[i]));
                if self.complex {
                    out.push_str(&format!(",{:.12e},{:.12e}", t.imag[i], t.magnitude(i)));
                }
            }
            out.push('\n');
        }
        out
    }
}
pub fn csv_cell(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub fn bom_csv(s: &Sheet) -> String {
    let mut out = "Reference,Type,Value,Footprint\n".to_string();
    for c in &s.components {
        out.push_str(&format!(
            "{},{},{},{}\n",
            csv_cell(&c.reference),
            csv_cell(c.kind.name()),
            csv_cell(&c.value),
            csv_cell(&c.footprint)
        ));
    }
    out
}
pub fn parse_raw(s: &str) -> Result<Simulation, String> {
    let lines = s.lines().collect::<Vec<_>>();
    let mut count = 0usize;
    let mut np = 0usize;
    let mut complex = false;
    let mut title = String::new();
    let mut vars = None;
    let mut vals = None;
    for (i, l) in lines.iter().enumerate() {
        if let Some(v) = l.strip_prefix("No. Variables:") {
            count = v.trim().parse().map_err(|_| "Invalid variable count")?;
        }
        if let Some(v) = l.strip_prefix("No. Points:") {
            np = v.trim().parse().map_err(|_| "Invalid point count")?;
        }
        if let Some(v) = l.strip_prefix("Flags:") {
            complex = v.contains("complex");
        }
        if let Some(v) = l.strip_prefix("Plotname:") {
            title = v.trim().into();
        }
        if l.trim() == "Variables:" {
            vars = Some(i + 1);
        }
        if l.trim() == "Values:" {
            vals = Some(i + 1);
            break;
        }
    }
    if count == 0 || np == 0 || count > 20000 || np > 500000 || count.saturating_mul(np) > 20000000
    {
        return Err("Invalid or oversized raw data.".into());
    }
    let start = vars.ok_or("Missing Variables section")?;
    let mut variables = vec![];
    for l in lines.iter().skip(start).take(count) {
        let fields = l.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 3 {
            return Err("Malformed variable description.".into());
        }
        variables.push(Trace {
            name: fields[1].into(),
            unit: fields[2].into(),
            real: Vec::with_capacity(np),
            imag: Vec::with_capacity(np),
        });
    }
    let start = vals.ok_or("Missing Values section")?;
    let mut tokens = lines[start..].iter().flat_map(|l| l.split_whitespace());
    let number = |s: &str| {
        s.replace(['D', 'd'], "e")
            .parse::<f64>()
            .map_err(|_| format!("Invalid sample '{s}'."))
    };
    for i in 0..np {
        let index = tokens
            .next()
            .ok_or("Truncated raw data")?
            .parse::<usize>()
            .map_err(|_| "Invalid raw index")?;
        if index != i {
            return Err("Unexpected sample index.".into());
        }
        for t in &mut variables {
            let value = tokens.next().ok_or("Truncated raw data")?;
            let (re, im) = if complex {
                let (a, b) = value.split_once(',').ok_or("Malformed complex sample")?;
                (number(a)?, number(b)?)
            } else {
                (number(value)?, 0.)
            };
            if !re.is_finite() || !im.is_finite() {
                return Err("Non-finite simulation result.".into());
            }
            t.real.push(re);
            t.imag.push(im);
        }
    }
    let axis_index = variables.iter().position(|v| {
        matches!(
            v.name.as_str(),
            "time" | "frequency" | "v-sweep" | "i-sweep"
        )
    });
    let (axis_name, axis) = if let Some(i) = axis_index {
        let v = variables.remove(i);
        (v.name, v.real)
    } else {
        ("point".into(), (0..np).map(|i| i as f64).collect())
    };
    Ok(Simulation {
        title,
        axis_name,
        axis,
        traces: variables,
        complex,
        log: String::new(),
        elapsed: 0.,
    })
}
static JOB: AtomicU64 = AtomicU64::new(0);
pub fn find_engine() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            for relative in [
                "ngspice/Spice64/bin/ngspice_con.exe",
                "ngspice/Spice64/bin/ngspice.exe",
            ] {
                let path = p.join(relative);
                if path.is_file() {
                    return path;
                }
            }
        }
    }
    PathBuf::from(if cfg!(windows) {
        "ngspice_con.exe"
    } else {
        "ngspice"
    })
}
fn include_path(line: &str, base: Option<&Path>) -> Result<String, String> {
    let Some(base) = base else {
        return Ok(line.into());
    };
    let Some((directive, rest)) = line.trim().split_once(char::is_whitespace) else {
        return Ok(line.into());
    };
    if ![".include", ".inc", ".lib"].contains(&directive.to_lowercase().as_str()) {
        return Ok(line.into());
    }
    let rest = rest.trim_start();
    let (file, tail) = if rest.starts_with(['"', '\'']) {
        let quote = rest.chars().next().unwrap();
        let end = rest[1..].find(quote).ok_or("Unclosed include quote")? + 1;
        (&rest[1..end], &rest[end + 1..])
    } else {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        (&rest[..end], &rest[end..])
    };
    if directive.eq_ignore_ascii_case(".lib") && tail.trim().is_empty() {
        return Ok(line.into());
    }
    let p = Path::new(file);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    };
    let text = full.to_string_lossy().replace('\\', "/");
    if text.contains(['"', '\n', '\r']) {
        return Err("Unsupported include path.".into());
    }
    Ok(format!("{directive} \"{text}\"{tail}"))
}
pub fn run_spice(
    body: &str,
    a: &Analysis,
    engine: &Path,
    base: Option<&Path>,
) -> Result<Simulation, String> {
    let command = a.command()?;
    let mut clean = String::new();
    for line in body.lines() {
        let low = line.trim().to_lowercase();
        let directive = low.split_whitespace().next();
        if matches!(directive, Some(".control" | ".endc")) {
            return Err(
                "Remove .control/.endc scripts. Choose the analysis in CircuitCraft.".into(),
            );
        }
        if directive == Some(".end") {
            continue;
        }
        clean.push_str(&include_path(line, base)?);
        clean.push('\n');
    }
    if clean.trim().is_empty() {
        return Err("Empty netlist.".into());
    }
    let dir = std::env::temp_dir().join(format!(
        "circuitcraft-{}-{}",
        std::process::id(),
        JOB.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir(&dir).map_err(|e| e.to_string())?;
    let result = (|| {
        let input = dir.join("circuit.cir");
        let raw = dir.join("result.raw");
        let logpath = dir.join("engine.log");
        clean.push_str(&format!(
            "\n.control\nset filetype=ascii\n{command}\nwrite result.raw all\nquit\n.endc\n.end\n"
        ));
        fs::write(&input, clean).map_err(|e| e.to_string())?;
        let log = File::create(&logpath).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(engine);
        cmd.arg("-b")
            .arg(&input)
            .current_dir(&dir)
            .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log));
        if let Some(base) = base {
            cmd.env("NGSPICE_INPUT_DIR", base);
        }
        if let Some(root) = engine.parent().and_then(|p| p.parent()) {
            let scripts = root.join("share/ngspice/scripts");
            if scripts.join("spinit").is_file() {
                cmd.env("SPICE_SCRIPTS", scripts);
                cmd.env("SPICE_LIB_DIR", root.join("share/ngspice"));
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let start = Instant::now();
        let mut child=cmd.spawn().map_err(|e|format!("Cannot start ngspice '{}': {e}. Choose the executable in Engine settings, or use Linear DC.",engine.display()))?;
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if start.elapsed() > Duration::from_secs(45) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "ngspice exceeded the 45-second limit; reduce duration or sampling density."
                        .into(),
                );
            }
            thread::sleep(Duration::from_millis(20));
        };
        let log = fs::read_to_string(logpath).unwrap_or_default();
        if !status.success() {
            return Err(format!("ngspice exited with {status}\n{log}"));
        }
        let data = fs::read_to_string(raw)
            .map_err(|_| format!("ngspice produced no raw results.\n{log}"))?;
        let mut sim = parse_raw(&data).map_err(|e| format!("{e}\n{log}"))?;
        sim.log = log;
        sim.elapsed = start.elapsed().as_secs_f64();
        if let Analysis::Noise { source, .. } = a {
            for t in &mut sim.traces {
                t.unit = if t.name.starts_with("inoise")
                    && source.to_ascii_lowercase().starts_with('i')
                {
                    "current-density"
                } else {
                    "voltage-density"
                }
                .into();
            }
        }
        Ok(sim)
    })();
    let _ = fs::remove_dir_all(dir);
    result
}
fn source_dc(s: &str) -> Result<f64, String> {
    let p = s.split_whitespace().collect::<Vec<_>>();
    let first = p.first().ok_or("Source needs a DC value")?;
    if first.eq_ignore_ascii_case("dc") {
        parse_value(p.get(1).ok_or("DC needs a numeric value")?)
    } else {
        parse_value(first).map_err(|_| {
            "Linear DC requires a numeric value or DC value. Use ngspice for waveform sources."
                .into()
        })
    }
}
pub fn linear_dc(s: &Sheet) -> Result<Simulation, String> {
    if !s.children.is_empty() {
        return linear_dc(&crate::flatten(s)?);
    }
    let n = connect(s);
    if let Some(e) = n.issues.iter().find(|i| i.error) {
        return Err(e.message.clone());
    }
    let nodes = n
        .nets
        .iter()
        .filter(|n| n.name != "0")
        .map(|n| n.name.clone())
        .collect::<Vec<_>>();
    if nodes.len() > 300 {
        return Err("Linear DC limit: 300 nodes. Use ngspice.".into());
    }
    let lookup: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect();
    let branches = s
        .components
        .iter()
        .filter(|c| matches!(c.kind, Kind::Voltage | Kind::Inductor))
        .collect::<Vec<_>>();
    let size = nodes.len() + branches.len();
    if size == 0 {
        return Err("No circuit to solve.".into());
    }
    let mut a = vec![vec![0f64; size]; size];
    let mut b = vec![0f64; size];
    let node = |id: u64, p: usize| lookup.get(&n.pin_net[&(id, p)]).copied();
    for c in &s.components {
        let p = node(c.id, 0);
        let q = node(c.id, 1);
        match c.kind {
            Kind::Resistor => {
                let g = 1. / parse_value(&c.value)?;
                if let Some(p) = p {
                    a[p][p] += g;
                }
                if let Some(q) = q {
                    a[q][q] += g;
                }
                if let (Some(p), Some(q)) = (p, q) {
                    a[p][q] -= g;
                    a[q][p] -= g;
                }
            }
            Kind::Current => {
                let v = source_dc(&c.value)?;
                if let Some(p) = p {
                    b[p] -= v;
                }
                if let Some(q) = q {
                    b[q] += v;
                }
            }
            Kind::Voltage | Kind::Inductor => {
                let k = nodes.len() + branches.iter().position(|x| x.id == c.id).unwrap();
                if let Some(p) = p {
                    a[p][k] += 1.;
                    a[k][p] += 1.;
                }
                if let Some(q) = q {
                    a[q][k] -= 1.;
                    a[k][q] -= 1.;
                }
                b[k] = if c.kind == Kind::Voltage {
                    source_dc(&c.value)?
                } else {
                    0.
                };
            }
            Kind::Capacitor | Kind::Connector => {}
            _ => {
                return Err(format!(
                    "{} requires ngspice. Linear DC supports R/C/L/V/I.",
                    c.reference
                ))
            }
        }
    }
    let original = a.clone();
    let rhs = b.clone();
    for col in 0..size {
        let pivot = (col..size)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        if a[pivot][col].abs() < 1e-14 {
            return Err(
                "Singular circuit: check floating nets or contradictory voltage sources.".into(),
            );
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let div = a[col][col];
        for k in col..size {
            a[col][k] /= div;
        }
        b[col] /= div;
        for row in 0..size {
            if row != col {
                let g = a[row][col];
                for k in col..size {
                    a[row][k] -= g * a[col][k];
                }
                b[row] -= g * b[col];
            }
        }
    }
    let residual = original
        .iter()
        .zip(rhs)
        .map(|(row, r)| (row.iter().zip(&b).map(|(a, b)| a * b).sum::<f64>() - r).abs())
        .fold(0., f64::max);
    if b.iter().any(|v| !v.is_finite()) || residual > 1e-7 {
        return Err("Unstable linear solution. Use ngspice.".into());
    }
    let mut traces = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| Trace {
            name: format!("v({})", n.to_lowercase()),
            unit: "voltage".into(),
            real: vec![b[i]],
            imag: vec![0.],
        })
        .collect::<Vec<_>>();
    for (i, c) in branches.iter().enumerate() {
        traces.push(Trace {
            name: format!("i({})", c.reference.to_lowercase()),
            unit: "current".into(),
            real: vec![b[nodes.len() + i]],
            imag: vec![0.],
        });
    }
    for c in s.components.iter().filter(|c| c.kind == Kind::Resistor) {
        let voltage = |p| node(c.id, p).map(|i| b[i]).unwrap_or(0.);
        traces.push(Trace {
            name: format!("i({})", c.reference.to_lowercase()),
            unit: "current".into(),
            real: vec![(voltage(0) - voltage(1)) / parse_value(&c.value)?],
            imag: vec![0.],
        });
    }
    Ok(Simulation{title:"Operating point · Rust linear MNA".into(),axis_name:"point".into(),axis:vec![0.],traces,complex:false,log:format!("{} nodes, {} voltage branches; residual {residual:.3e}. Capacitors open, inductors short.",nodes.len(),branches.len()),elapsed:0.})
}
pub fn check_engine(engine: &Path) -> Result<String, String> {
    let divider = Sheet::divider();
    let rc = Sheet::rc();
    let op = run_spice(&netlist(&divider)?, &Analysis::OperatingPoint, engine, None)?;
    let dc = op.trace("v(vout)").ok_or("Missing DC output")?.real[0];
    let fc = 1. / (2. * std::f64::consts::PI * 0.001);
    let ac = run_spice(
        &netlist(&rc)?,
        &Analysis::Ac {
            points_per_decade: 100,
            start: fc,
            stop: fc * 10.,
        },
        engine,
        None,
    )?;
    let t = ac.trace("v(vout)").ok_or("Missing AC output")?;
    let mag = t.magnitude(0);
    let phase = t.phase(0);
    let tran = run_spice(
        &netlist(&rc)?,
        &Analysis::Transient {
            step: 1e-5,
            stop: 0.005,
            uic: true,
        },
        engine,
        None,
    )?;
    let i = tran
        .axis
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - 0.001).abs().total_cmp(&(*b - 0.001).abs()))
        .unwrap()
        .0;
    let observed = tran
        .trace("v(vout)")
        .ok_or("Missing transient output")?
        .real[i];
    let expected = 1. - (-tran.axis[i] / 0.001).exp();
    let sweep = run_spice(
        &netlist(&divider)?,
        &Analysis::DcSweep {
            source: "V1".into(),
            start: 0.,
            stop: 10.,
            step: 1.,
        },
        engine,
        None,
    )?;
    let error = sweep
        .axis
        .iter()
        .zip(&sweep.trace("v(vout)").ok_or("Missing sweep output")?.real)
        .map(|(x, y)| (y - x / 2.).abs())
        .fold(0., f64::max);
    if (dc - 5.).abs() > 1e-8
        || (mag - 1. / 2f64.sqrt()).abs() > 1e-6
        || (phase + 45.).abs() > 1e-4
        || (observed - expected).abs() > 0.003
        || error > 1e-8
    {
        return Err("Physics comparison failed.".into());
    }
    Ok(format!("PASS · engine {}\nDC divider {dc:.9} V\nRC cutoff {fc:.6} Hz; magnitude {mag:.9}; phase {phase:.6}°\nRC transient at {:.9} s: {observed:.9} V; analytic {expected:.9} V\nDC sweep maximum error {error:.3e} V\n{}",engine.display(),tran.axis[i],op.log))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_linear_dc() {
        assert_eq!(parse_value("1M").unwrap(), 0.001);
        assert_eq!(parse_value("1MEG").unwrap(), 1e6);
        let s = linear_dc(&Sheet::divider()).unwrap();
        assert!((s.trace("v(vout)").unwrap().real[0] - 5.).abs() < 1e-10);
        assert!((s.trace("i(r1)").unwrap().real[0] - 0.005).abs() < 1e-10);
    }
    #[test]
    fn raw_and_control_errors() {
        assert!(parse_raw("No. Variables: 2\nNo. Points: 3\nValues:\n").is_err());
        assert!(run_spice(
            "Title\n.control * comment\nquit\n.endc",
            &Analysis::OperatingPoint,
            Path::new("missing"),
            None
        )
        .unwrap_err()
        .contains("Remove .control"));
    }
    #[test]
    fn physical_ngspice_comparison() {
        if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_ok() {
            let report = check_engine(Path::new("ngspice")).unwrap();
            assert!(report.starts_with("PASS"));
        }
    }
    #[test]
    fn relative_include_with_spaces() {
        if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("circuitcraft include {}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("buffer model.lib"),
            ".subckt BUFFER INPUT OUTPUT\nE1 OUTPUT 0 INPUT 0 1\n.ends BUFFER\n",
        )
        .unwrap();
        let result=run_spice("Include test\nV1 vin 0 DC 3\nX1 vin out BUFFER\nR1 out 0 1k\n.include \"buffer model.lib\"\n.end",&Analysis::OperatingPoint,Path::new("ngspice"),Some(&dir)).unwrap();
        assert!((result.trace("v(out)").unwrap().real[0] - 3.).abs() < 1e-9);
        fs::remove_dir_all(dir).unwrap();
    }
}
