use circuitcraft_core::*;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
fn write(p: &Path, s: impl AsRef<[u8]>) -> Result<(), String> {
    fs::write(p, s).map_err(|e| e.to_string())
}
fn export(s: &Sheet, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    write(&dir.join("schematic.svg"), schematic_svg(s))?;
    write(&dir.join("bom.csv"), bom_csv(&flatten(s)?))?;
    write(&dir.join("placement.csv"), s.board.placement_csv())?;
    write(&dir.join("board.obj"), s.board.mesh(true)?.obj())?;
    write(&dir.join("board.svg"), s.board.svg()?)?;
    write(&dir.join("circuit.cir"), format!("{}\n.end\n", netlist(s)?))?;
    let fab = dir.join("fabrication");
    fs::create_dir_all(&fab).map_err(|e| e.to_string())?;
    for (p, txt) in s.board.gerbers()? {
        write(&fab.join(p), txt)?;
    }
    write(
        &dir.join("checks.txt"),
        format!(
            "ERC:\n{}\nPCB:\n{}\n",
            connect(&flatten(s)?)
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            s.pcb_drc()
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )?;
    Ok(())
}
fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" {
        println!("CircuitCraft CLI\nexamples DIRECTORY\nexport PROJECT DIRECTORY\nvalidate PROJECT\nautoroute PROJECT OUTPUT.cforge [--grid 0.5 --width 0.3]\npcb3d PROJECT OUTPUT.obj\ncheck-engine REPORT [--engine PATH]\nsimulate PROJECT op|linear|ac|tran|dc|noise OUTPUT.csv [--engine PATH --start VALUE --stop VALUE --step VALUE --points N --source V1 --output VOUT --uic --measurements REPORT.csv]\nsweep PROJECT op|ac|tran|dc|noise OUTPUT.csv --component R1 --values 500,1k,2k [--measurements REPORT.csv]\nSignal math: --math-a SIGNAL --math-b SIGNAL --math-op ratio|difference|product");
        return Ok(());
    }
    let arg = |i| {
        args.get(i)
            .cloned()
            .ok_or("Missing argument; use --help".to_string())
    };
    let mut options = HashMap::<String, String>::new();
    let mut i = 1;
    while i < args.len() {
        if args[i].starts_with("--") {
            if args[i] == "--uic" {
                options.insert(args[i].clone(), "true".into());
                i += 1;
            } else {
                options.insert(args[i].clone(), arg(i + 1)?);
                i += 2;
            }
        } else {
            i += 1;
        }
    }
    let engine = options
        .get("--engine")
        .map(PathBuf::from)
        .unwrap_or_else(find_engine);
    let value = |k: &str, d: &str| parse_value(options.get(k).map(String::as_str).unwrap_or(d));
    if args[0] == "examples" {
        let dir = PathBuf::from(arg(1)?);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (name, mut s) in [
            ("voltage-divider", Sheet::divider()),
            ("rc-lowpass", Sheet::rc()),
            ("half-wave-rectifier", Sheet::rectifier()),
            ("hierarchical-divider", hierarchical_divider()),
        ] {
            if name == "hierarchical-divider" {
                s.autoroute(&RouteOptions::default())?;
            }
            write(&dir.join(format!("{name}.cforge")), s.to_json()?)?;
            export(&s, &dir.join(name))?;
        }
        println!("Examples: {}", dir.display());
        return Ok(());
    }
    if args[0] == "check-engine" {
        let report = check_engine(&engine)?;
        write(Path::new(&arg(1)?), &report)?;
        println!("{report}");
        return Ok(());
    }
    let project = arg(1)?;
    let mut s = Sheet::from_json(&fs::read_to_string(&project).map_err(|e| e.to_string())?)?;
    match args[0].as_str() {
        "export" => export(&s, Path::new(&arg(2)?))?,
        "autoroute" => {
            let o = RouteOptions {
                grid: value("--grid", "0.5")?,
                width: value("--width", "0.3")?,
                ..Default::default()
            };
            let r = s.autoroute(&o)?;
            write(Path::new(&arg(2)?), s.to_json()?)?;
            println!(
                "Routed: {}, remaining: {}, tracks: {}, vias: {}",
                r.routed, r.remaining, r.added_tracks, r.added_vias
            );
            for b in r.blocked {
                println!("{b}");
            }
            if r.remaining > 0 {
                return Err("Partial route saved; unresolved connections remain. Reposition footprints or reduce the grid.".into());
            }
        }
        "pcb3d" => write(Path::new(&arg(2)?), s.board.mesh(true)?.obj())?,
        "validate" => {
            let n = connect(&flatten(&s)?);
            let drc = s.pcb_drc();
            println!("ERC: {}, PCB DRC: {}", n.issues.len(), drc.len());
            for i in &n.issues {
                println!("{}", i.message);
            }
            for i in &drc {
                println!("{}", i.message);
            }
            if n.issues.iter().any(|i| i.error) || !drc.is_empty() {
                return Err("Design checks reported issues.".into());
            }
        }
        "simulate" | "sweep" => {
            let kind = arg(2)?;
            let sim = if kind == "linear" {
                linear_dc(&s)?
            } else {
                let a = match kind.as_str() {
                    "op" => Analysis::OperatingPoint,
                    "ac" => Analysis::Ac {
                        points_per_decade: options
                            .get("--points")
                            .map(|s| s.parse::<u32>().map_err(|e| e.to_string()))
                            .transpose()?
                            .unwrap_or(100),
                        start: value("--start", "1")?,
                        stop: value("--stop", "100k")?,
                    },
                    "tran" => Analysis::Transient {
                        step: value("--step", "10u")?,
                        stop: value("--stop", "5m")?,
                        uic: options.contains_key("--uic"),
                    },
                    "dc" => Analysis::DcSweep {
                        source: options.get("--source").cloned().unwrap_or("V1".into()),
                        start: value("--start", "0")?,
                        stop: value("--stop", "10")?,
                        step: value("--step", "0.1")?,
                    },
                    "noise" => Analysis::Noise {
                        output: options.get("--output").cloned().unwrap_or("VOUT".into()),
                        source: options.get("--source").cloned().unwrap_or("V1".into()),
                        points_per_decade: options
                            .get("--points")
                            .map(|s| s.parse::<u32>().map_err(|e| e.to_string()))
                            .transpose()?
                            .unwrap_or(100),
                        start: value("--start", "1")?,
                        stop: value("--stop", "100k")?,
                    },
                    _ => return Err("Choose op, linear, ac, tran, dc or noise.".into()),
                };
                let body = if s.raw_mode {
                    s.raw_netlist.clone()
                } else {
                    netlist(&s)?
                };
                let base = if !s.include_directory.is_empty() {
                    Some(PathBuf::from(&s.include_directory))
                } else {
                    fs::canonicalize(project)
                        .ok()
                        .and_then(|p| p.parent().map(Path::to_path_buf))
                };
                if args[0] == "sweep" {
                    let reference = options
                        .get("--component")
                        .ok_or("Sweep needs --component R1")?;
                    let values = sweep_values(
                        options
                            .get("--values")
                            .ok_or("Sweep needs --values 500,1k,2k")?,
                    )?;
                    let sweep = run_sweep(
                        &s,
                        reference,
                        &values,
                        &a,
                        &engine,
                        base.as_deref(),
                        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    )?;
                    write(Path::new(&arg(3)?), sweep.csv())?;
                    if let Some(p) = options.get("--measurements") {
                        write(Path::new(p), sweep.measurements_csv())?;
                    }
                    println!(
                        "{}: {} runs, original circuit retained",
                        sweep.reference,
                        sweep.runs.len()
                    );
                    return Ok(());
                } else {
                    run_spice(&body, &a, &engine, base.as_deref())?
                }
            };
            let mut sim = sim;
            if args[0] == "sweep" {
                return Err("Use op, ac, tran, dc or noise for sweeps.".into());
            }
            if let Some(op) = options.get("--math-op") {
                let op = match op.as_str() {
                    "ratio" => TraceMath::Ratio,
                    "difference" => TraceMath::Difference,
                    "product" => TraceMath::Product,
                    _ => return Err("Math: choose ratio, difference or product.".into()),
                };
                let a = options
                    .get("--math-a")
                    .ok_or("Math needs --math-a SIGNAL")?;
                let b = options
                    .get("--math-b")
                    .ok_or("Math needs --math-b SIGNAL")?;
                let t = derived_trace(&sim, a, b, op)?;
                sim.traces.push(t);
            }
            if let Some(p) = options.get("--measurements") {
                write(Path::new(p), measurements_csv(&sim))?;
            }
            write(Path::new(&arg(3)?), sim.csv())?;
            println!("{}: {} samples", sim.title, sim.axis.len());
        }
        _ => return Err("Unknown command.".into()),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
