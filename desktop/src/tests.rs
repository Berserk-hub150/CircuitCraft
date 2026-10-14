use super::*;
fn frame(ctx: &egui::Context, a: &mut App, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 900.))),
            events,
            ..Default::default()
        },
        |ctx| {
            a.shortcuts(ctx);
            a.header(ctx);
            a.footer(ctx);
            a.left(ctx);
            a.right(ctx);
            a.center(ctx);
            a.dialogs(ctx);
        },
    )
}
fn click(ctx: &egui::Context, a: &mut App, p: Pos2) {
    frame(
        ctx,
        a,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    frame(
        ctx,
        a,
        vec![egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}
#[test]
fn placement_wiring_and_history() {
    let ctx = egui::Context::default();
    style(&ctx);
    let mut a = App::default();
    frame(&ctx, &mut a, vec![]);
    let before = a.sheet.components.len();
    a.set_tool(Tool::Place(Kind::Resistor));
    let p = a.camera.screen(a.canvas, Mm::new(-4., 0.));
    click(&ctx, &mut a, p);
    assert_eq!(a.sheet.components.len(), before + 1);
    assert_eq!(a.sheet.components.last().unwrap().at, Grid::new(-4, 0));
    a.set_tool(Tool::Select);
    a.rotate();
    assert_eq!(a.sheet.components.last().unwrap().rotation, 1);
    assert!(a.history.undo(&mut a.sheet));
    assert_eq!(a.sheet.components.last().unwrap().rotation, 0);
    let wires = a.sheet.wires.len();
    a.set_tool(Tool::Wire);
    let p = a.camera.screen(a.canvas, Mm::new(-2., 0.));
    let q = a.camera.screen(a.canvas, Mm::new(-2., 2.));
    click(&ctx, &mut a, p);
    click(&ctx, &mut a, q);
    assert_eq!(a.sheet.wires.len(), wires + 1);
    assert!(a.history.undo(&mut a.sheet));
    assert_eq!(a.sheet.wires.len(), wires);
}
#[test]
fn pcb_pad_snapping_and_routing() {
    let ctx = egui::Context::default();
    style(&ctx);
    let mut a = App::default();
    a.tab = Tab::Pcb;
    a.fit = true;
    a.sheet.board.tracks.clear();
    frame(&ctx, &mut a, vec![]);
    a.set_tool(Tool::Route);
    let pads = a.sheet.board.pads();
    let vin = pads.iter().filter(|p| p.net == "VIN").collect::<Vec<_>>();
    let p = a
        .pcb_camera
        .screen(a.canvas, Mm::new(vin[0].at.x, -vin[0].at.y));
    let q = a
        .pcb_camera
        .screen(a.canvas, Mm::new(vin[1].at.x, -vin[1].at.y));
    click(&ctx, &mut a, p);
    click(&ctx, &mut a, q);
    assert_eq!(a.sheet.board.tracks.len(), 1);
    assert_eq!(a.sheet.board.tracks[0].net, "VIN");
    assert_eq!(a.sheet.board.tracks[0].points[0], vin[0].at);
    assert!(a.route_start.is_none());
}
#[test]
fn hierarchical_edit_save_and_undo_preserve_the_whole_project() {
    let mut a = App::default();
    a.replace(hierarchical_divider());
    a.enter_page(0);
    a.checkpoint();
    a.sheet.components[0].value = "2k".into();
    let path = std::env::temp_dir().join(format!(
        "circuitcraft-hierarchy-{}.cforge",
        std::process::id()
    ));
    a.file_do(FileAction::SaveAs, &path);
    let saved = Sheet::from_json(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved.components.len(), 2);
    assert_eq!(saved.children[0].sheet.components[0].value, "2k");
    assert!(a.undo_project());
    assert!(a.parents.is_empty());
    assert_eq!(a.sheet.children[0].sheet.components[0].value, "1k");
    assert!(a.redo_project());
    assert_eq!(a.sheet.children[0].sheet.components[0].value, "2k");
    let _ = fs::remove_file(path);
}
#[test]
fn background_router_applies_once_and_undo_restores_routing() {
    let ctx = egui::Context::default();
    let mut a = App::default();
    a.sheet.board.tracks.clear();
    a.start_router();
    for _ in 0..300 {
        a.poll_router(&ctx);
        if !a.router_running() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!a.router_running());
    assert!(a.sheet.board.ratsnest().is_empty(), "{}", a.status);
    assert!(a.undo_project());
    assert!(a.sheet.board.tracks.is_empty());
    a.start_router();
    a.checkpoint();
    a.sheet.components[1].value = "3k".into();
    for _ in 0..300 {
        a.poll_router(&ctx);
        if !a.router_running() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(a.sheet.components[1].value, "3k");
    assert!(a.sheet.board.tracks.is_empty());
}
#[test]
fn render_hierarchy_with_real_frames() {
    let ctx = egui::Context::default();
    style(&ctx);
    let mut a = App::default();
    a.sheet = hierarchical_divider();
    let first = frame(&ctx, &mut a, vec![]);
    let mut out = frame(&ctx, &mut a, vec![]);
    let mut textures = first.textures_delta.set;
    textures.extend(out.textures_delta.set.clone());
    out.textures_delta.set = textures;
    assert!(out.shapes.len() > 100);
    if let Ok(dir) = std::env::var("CIRCUITCRAFT_REVIEW") {
        fs::create_dir_all(&dir).unwrap();
        render(&ctx, &out, &format!("{dir}/hierarchy.ppm"));
    }
    a.enter_page(0);
    assert_eq!(a.sheet.title, "Load");
    frame(&ctx, &mut a, vec![]);
    a.leave_page();
    assert_eq!(a.sheet.components.len(), 2);
}
#[test]
fn sweep_noise_and_custom_footprint_workflows() {
    if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_err() {
        return;
    }
    let mut a = App::default();
    a.sweep_enabled = true;
    let original = a.sheet.clone();
    a.run(false);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while a.job.is_some() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
        a.poll();
    }
    assert!(!a.status_error, "{}", a.log.join("\n"));
    assert_eq!(a.sheet, original);
    assert_eq!(a.sweep.as_ref().unwrap().runs.len(), 3);
    assert_eq!(a.result.as_ref().unwrap().traces.len(), 3);
    assert_eq!(a.enabled.len(), 3);
    for (name, noise) in [("parameter-sweep", false), ("noise", true)] {
        if noise {
            a.sweep_enabled = false;
            a.analysis = 4;
            a.run(false);
            while a.job.is_some() {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(20));
                a.poll();
            }
            assert!(a
                .result
                .as_ref()
                .unwrap()
                .trace("onoise_spectrum")
                .is_some());
        }
        let ctx = egui::Context::default();
        style(&ctx);
        let first = frame(&ctx, &mut a, vec![]);
        let mut out = frame(&ctx, &mut a, vec![]);
        let mut textures = first.textures_delta.set;
        textures.extend(out.textures_delta.set.clone());
        out.textures_delta.set = textures;
        assert!(out.shapes.len() > 100);
        if let Ok(dir) = std::env::var("CIRCUITCRAFT_REVIEW") {
            fs::create_dir_all(&dir).unwrap();
            render(&ctx, &out, &format!("{dir}/{name}.ppm"));
        }
    }
    let id = a.sheet.components[1].id;
    a.selected = Some(Selection::Component(id));
    let folder =
        std::env::temp_dir().join(format!("circuitcraft-ui-footprint-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let file = folder.join("part.json");
    a.file_do(FileAction::ExportFootprint, &file);
    assert!(!a.status_error);
    let mut f = FootprintDefinition::from_json(&fs::read_to_string(&file).unwrap()).unwrap();
    f.name = "TEST-SMD".into();
    for p in &mut f.pads {
        p.drill = 0.;
        p.rectangular = true;
    }
    fs::write(&file, f.to_json().unwrap()).unwrap();
    a.file_do(FileAction::ImportFootprint, &file);
    assert!(!a.status_error);
    assert_eq!(a.sheet.components[1].custom_pads.len(), 2);
    a.sheet.sync_board();
    assert_eq!(a.sheet.board.footprints[1].style, "TEST-SMD");
    a.history.undo(&mut a.sheet);
    assert!(a.sheet.components[1].custom_pads.is_empty());
    fs::remove_dir_all(folder).unwrap();
    // A selection removed by undo must not crash the custom editor.
    let ctx = egui::Context::default();
    a.selected = Some(Selection::Component(u64::MAX));
    a.tab = Tab::Schematic;
    frame(&ctx, &mut a, vec![]);
}
#[test]
fn render_all_workspaces() {
    for tab in [
        Tab::Schematic,
        Tab::Pcb,
        Tab::Pcb3d,
        Tab::Simulation,
        Tab::Spice,
        Tab::Bom,
    ] {
        let ctx = egui::Context::default();
        style(&ctx);
        let mut a = App::default();
        a.tab = tab;
        a.fit = true;
        if tab == Tab::Simulation {
            a.result = Some(if std::env::var("CIRCUITCRAFT_NGSPICE_TESTS").is_ok() {
                run_spice(
                    &netlist(&a.sheet).unwrap(),
                    &Analysis::Ac {
                        points_per_decade: 100,
                        start: 1.,
                        stop: 100000.,
                    },
                    Path::new("ngspice"),
                    None,
                )
                .unwrap()
            } else {
                linear_dc(&a.sheet).unwrap()
            });
        }
        if tab == Tab::Pcb {
            a.drc_issues = a.sheet.pcb_drc();
            a.drc_stale = false;
            assert!(a.drc_issues.is_empty());
        }
        let first = frame(&ctx, &mut a, vec![]);
        let mut out = frame(&ctx, &mut a, vec![]);
        let mut textures = first.textures_delta.set;
        textures.extend(out.textures_delta.set.clone());
        out.textures_delta.set = textures;
        assert!(out.shapes.len() > if tab == Tab::Pcb3d { 40 } else { 100 });
        if let Ok(dir) = std::env::var("CIRCUITCRAFT_REVIEW") {
            fs::create_dir_all(&dir).unwrap();
            let name = match tab {
                Tab::Schematic => "schematic",
                Tab::Pcb => "pcb",
                Tab::Pcb3d => "pcb-3d",
                Tab::Simulation => "waveforms",
                Tab::Spice => "spice",
                Tab::Bom => "bom",
            };
            render(&ctx, &out, &format!("{dir}/{name}.ppm"));
        }
    }
}
fn render(ctx: &egui::Context, out: &egui::FullOutput, path: &str) {
    use std::io::Write;
    let w = 1360usize;
    let h = 900usize;
    let mut pixels = vec![[0f32; 3]; w * h];
    let mut textures =
        std::collections::HashMap::<egui::TextureId, ([usize; 2], Vec<[u8; 4]>)>::new();
    for (id, delta) in &out.textures_delta.set {
        let size = delta.image.size();
        let rgba = match &delta.image {
            egui::ImageData::Color(i) => i.pixels.iter().map(|c| c.to_array()).collect::<Vec<_>>(),
            egui::ImageData::Font(i) => i.srgba_pixels(Some(1.)).map(|c| c.to_array()).collect(),
        };
        if let Some(pos) = delta.pos {
            if let Some((full, data)) = textures.get_mut(id) {
                for y in 0..size[1] {
                    for x in 0..size[0] {
                        data[(pos[1] + y) * full[0] + pos[0] + x] = rgba[y * size[0] + x];
                    }
                }
            }
        } else {
            textures.insert(*id, (size, rgba));
        }
    }
    for clip in ctx.tessellate(out.shapes.clone(), out.pixels_per_point) {
        let egui::epaint::Primitive::Mesh(mesh) = clip.primitive else {
            continue;
        };
        let Some((size, texture)) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for idx in mesh.indices.chunks_exact(3) {
            let a = mesh.vertices[idx[0] as usize];
            let b = mesh.vertices[idx[1] as usize];
            let c = mesh.vertices[idx[2] as usize];
            let edge =
                |a: Pos2, b: Pos2, p: Pos2| (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x);
            let area = edge(a.pos, b.pos, c.pos);
            if area.abs() < 0.00001 {
                continue;
            }
            let minx = a
                .pos
                .x
                .min(b.pos.x)
                .min(c.pos.x)
                .floor()
                .max(clip.clip_rect.min.x)
                .max(0.) as usize;
            let maxx = a
                .pos
                .x
                .max(b.pos.x)
                .max(c.pos.x)
                .ceil()
                .min(clip.clip_rect.max.x)
                .min(w as f32) as usize;
            let miny = a
                .pos
                .y
                .min(b.pos.y)
                .min(c.pos.y)
                .floor()
                .max(clip.clip_rect.min.y)
                .max(0.) as usize;
            let maxy = a
                .pos
                .y
                .max(b.pos.y)
                .max(c.pos.y)
                .ceil()
                .min(clip.clip_rect.max.y)
                .min(h as f32) as usize;
            let ca = a.color.to_array();
            let cb = b.color.to_array();
            let cc = c.color.to_array();
            for y in miny..maxy {
                for x in minx..maxx {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let wa = edge(b.pos, c.pos, p) / area;
                    let wb = edge(c.pos, a.pos, p) / area;
                    let wc = 1. - wa - wb;
                    if wa < -0.00001 || wb < -0.00001 || wc < -0.00001 {
                        continue;
                    }
                    let u = (a.uv.x * wa + b.uv.x * wb + c.uv.x * wc).clamp(0., 1.);
                    let v = (a.uv.y * wa + b.uv.y * wb + c.uv.y * wc).clamp(0., 1.);
                    let tx = ((u * size[0] as f32) as usize).min(size[0] - 1);
                    let ty = ((v * size[1] as f32) as usize).min(size[1] - 1);
                    let t = texture[ty * size[0] + tx];
                    let alpha = (ca[3] as f32 * wa + cb[3] as f32 * wb + cc[3] as f32 * wc) / 255.
                        * t[3] as f32
                        / 255.;
                    for k in 0..3 {
                        let src = (ca[k] as f32 * wa + cb[k] as f32 * wb + cc[k] as f32 * wc)
                            / 255.
                            * t[k] as f32
                            / 255.;
                        pixels[y * w + x][k] = src + pixels[y * w + x][k] * (1. - alpha);
                    }
                }
            }
        }
    }
    let mut file = fs::File::create(path).unwrap();
    write!(file, "P6\n{w} {h}\n255\n").unwrap();
    let bytes = pixels
        .into_iter()
        .flat_map(|p| p.map(|v| (v.clamp(0., 1.) * 255.).round() as u8))
        .collect::<Vec<_>>();
    file.write_all(&bytes).unwrap();
}
