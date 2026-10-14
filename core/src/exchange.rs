use crate::{Kind, Mm, Sheet};
#[derive(Clone)]
pub struct Symbol {
    pub paths: Vec<Vec<Mm>>,
    pub circles: Vec<(Mm, f64)>,
    pub labels: Vec<(Mm, &'static str)>,
}
pub fn symbol(kind: Kind) -> Symbol {
    let mut s = Symbol {
        paths: vec![],
        circles: vec![],
        labels: vec![],
    };
    let p = |x, y| Mm::new(x, y);
    let mut path = |ps: &[(f64, f64)]| s.paths.push(ps.iter().map(|(x, y)| p(*x, *y)).collect());
    match kind {
        Kind::Resistor => {
            path(&[(-2., 0.), (-1.2, 0.)]);
            path(&[(1.2, 0.), (2., 0.)]);
            path(&[
                (-1.2, -0.45),
                (1.2, -0.45),
                (1.2, 0.45),
                (-1.2, 0.45),
                (-1.2, -0.45),
            ]);
        }
        Kind::Capacitor => {
            path(&[(-2., 0.), (-0.3, 0.)]);
            path(&[(0.3, 0.), (2., 0.)]);
            path(&[(-0.3, -0.8), (-0.3, 0.8)]);
            path(&[(0.3, -0.8), (0.3, 0.8)]);
        }
        Kind::Inductor => {
            path(&[(-2., 0.), (-1.2, 0.)]);
            path(&[(1.2, 0.), (2., 0.)]);
            s.paths.push(
                (0..=80)
                    .map(|i| {
                        p(
                            -1.2 + i as f64 * 2.4 / 80.,
                            -(i as f64 / 80. * std::f64::consts::PI * 4.).sin().abs() * 0.6,
                        )
                    })
                    .collect(),
            );
        }
        Kind::Voltage | Kind::Current => {
            path(&[(0., -2.), (0., -1.)]);
            path(&[(0., 1.), (0., 2.)]);
            s.circles.push((p(0., 0.), 1.));
            if kind == Kind::Voltage {
                s.labels.push((p(0., -0.4), "+"));
                s.labels.push((p(0., 0.5), "−"));
            } else {
                s.paths.push(vec![p(0., 0.6), p(0., -0.6)]);
                s.paths.push(vec![p(-0.3, -0.2), p(0., -0.6), p(0.3, -0.2)]);
            }
        }
        Kind::Diode | Kind::Led => {
            path(&[(-2., 0.), (-0.6, 0.)]);
            path(&[(0.7, 0.), (2., 0.)]);
            path(&[(-0.6, -0.75), (-0.6, 0.75), (0.7, 0.), (-0.6, -0.75)]);
            path(&[(0.7, -0.75), (0.7, 0.75)]);
            if kind == Kind::Led {
                path(&[(0., -1.), (0.7, -1.7)]);
                path(&[(0.3, -1.7), (0.7, -1.7), (0.7, -1.3)]);
                path(&[(0.7, -0.8), (1.4, -1.5)]);
                path(&[(1., -1.5), (1.4, -1.5), (1.4, -1.1)]);
            }
        }
        Kind::Npn | Kind::Pnp => {
            path(&[(-2., 0.), (-0.6, 0.)]);
            path(&[(-0.6, -0.8), (-0.6, 0.8)]);
            path(&[(-0.6, -0.5), (0.7, -0.8), (2., -2.)]);
            path(&[(-0.6, 0.5), (0.7, 0.8), (2., 2.)]);
            s.circles.push((p(0., 0.), 1.35));
            s.paths.push(if kind == Kind::Npn {
                vec![p(0.5, 0.35), p(0.9, 1.), p(0.2, 0.9)]
            } else {
                vec![p(0.8, 0.25), p(0.1, 0.65), p(0.8, 0.95)]
            });
        }
        Kind::Nmos | Kind::Pmos => {
            path(&[(-2., 0.), (-0.7, 0.)]);
            path(&[(-0.7, -0.9), (-0.7, 0.9)]);
            path(&[(-0.35, -0.8), (-0.35, 0.8)]);
            path(&[(-0.35, -0.7), (0.5, -0.7), (2., -2.)]);
            path(&[(-0.35, 0.7), (0.5, 0.7), (2., 2.)]);
            path(&[(3., 1.), (1.5, 1.), (1.5, 0.), (-0.35, 0.)]);
            s.labels
                .push((p(0.5, 0.), if kind == Kind::Nmos { "N" } else { "P" }));
        }
        Kind::OpAmp => {
            path(&[(-1.5, -2.), (-1.5, 2.), (2., 0.), (-1.5, -2.)]);
            path(&[(-3., -1.), (-1.5, -1.)]);
            path(&[(-3., 1.), (-1.5, 1.)]);
            path(&[(2., 0.), (3., 0.)]);
            path(&[(0., 3.), (0., 1.15)]);
            s.labels.push((p(-1., -1.), "+"));
            s.labels.push((p(-1., 1.), "−"));
            s.labels.push((p(0.4, 2.1), "−"));
        }
        Kind::Switch => {
            path(&[(-2., 0.), (-0.7, 0.), (0.5, -0.8)]);
            path(&[(0.7, 0.), (2., 0.)]);
            path(&[(-1., 3.), (-1., 1.5), (1., 1.5), (1., 3.)]);
            s.circles.push((p(-0.7, 0.), 0.12));
            s.circles.push((p(0.7, 0.), 0.12));
        }
        Kind::Connector => {
            path(&[(-2., 0.), (-0.4, 0.)]);
            path(&[(0.4, 0.), (2., 0.)]);
            path(&[(-1., -0.8), (1., -0.8), (1., 0.8), (-1., 0.8), (-1., -0.8)]);
            s.circles.push((p(-0.4, 0.), 0.18));
            s.circles.push((p(0.4, 0.), 0.18));
        }
        Kind::Subcircuit => {
            path(&[(-2., -2.), (2., -2.), (2., 2.), (-2., 2.), (-2., -2.)]);
            path(&[(-3., -1.), (-2., -1.)]);
            path(&[(-3., 1.), (-2., 1.)]);
            path(&[(3., -1.), (2., -1.)]);
            path(&[(3., 1.), (2., 1.)]);
            s.labels.push((p(0., 0.), "SPICE"));
        }
    }
    s
}
pub fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn schematic_svg(s: &Sheet) -> String {
    let ps = s
        .components
        .iter()
        .flat_map(|c| c.pins().into_iter().map(|(_, p)| p))
        .chain(s.wires.iter().flat_map(|w| w.points.iter().copied()))
        .collect::<Vec<_>>();
    let x0 = ps.iter().map(|p| p.x).min().unwrap_or(0) as f64 - 5.;
    let y0 = ps.iter().map(|p| p.y).min().unwrap_or(0) as f64 - 5.;
    let x1 = ps.iter().map(|p| p.x).max().unwrap_or(10) as f64 + 5.;
    let y1 = ps.iter().map(|p| p.y).max().unwrap_or(10) as f64 + 5.;
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x0} {y0} {} {}\"><rect x=\"{x0}\" y=\"{y0}\" width=\"{}\" height=\"{}\" fill=\"white\"/><g fill=\"none\" stroke=\"#15576b\" stroke-width=\"0.09\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",x1-x0,y1-y0,x1-x0,y1-y0);
    for w in &s.wires {
        out.push_str(&format!(
            "<polyline points=\"{}\"/>",
            w.points
                .iter()
                .map(|p| format!("{},{}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    for c in &s.components {
        let sym = symbol(c.kind);
        out.push_str(&format!(
            "<g transform=\"translate({},{}) rotate({})\" stroke=\"#172437\">",
            c.at.x,
            c.at.y,
            c.rotation % 4 * 90
        ));
        for ps in sym.paths {
            out.push_str(&format!(
                "<polyline points=\"{}\"/>",
                ps.iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        for (p, r) in sym.circles {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{r}\"/>",
                p.x, p.y
            ));
        }
        for (p, t) in sym.labels {
            out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"0.5\" text-anchor=\"middle\" stroke=\"none\" fill=\"#172437\">{}</text>",p.x,p.y+0.18,xml(t)));
        }
        out.push_str("</g>");
        for (i, t) in [c.reference.as_str(), c.value.as_str()].iter().enumerate() {
            out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"0.45\" font-family=\"sans-serif\" stroke=\"none\" fill=\"#172437\">{}</text>",c.at.x as f64+1.7,c.at.y as f64-1.+i as f64*0.7,xml(t)));
        }
    }
    for p in &s.junctions {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"0.18\" fill=\"#15576b\"/>",
            p.x, p.y
        ));
    }
    for p in &s.grounds {
        out.push_str(&format!(
            "<path d=\"M {} {} v 0.5 m -0.65 0 h 1.3 m -1.1 0.25 h 0.9 m -0.65 0.25 h 0.4\"/>",
            p.x, p.y
        ));
    }
    out.push_str("</g>");
    for l in &s.labels {
        out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-family=\"sans-serif\" font-size=\"0.5\" fill=\"#0b8090\">{}</text>",l.at.x as f64+0.2,l.at.y as f64-0.4,xml(&l.name)));
    }
    out.push_str("</svg>");
    out
}
