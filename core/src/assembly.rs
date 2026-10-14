use crate::{csv_cell, Board, Copper};
impl Board {
    /// Coordinates follow the PCB editor: millimetres, origin bottom left.
    /// Includes every placed footprint; resolve actual parts before assembly.
    pub fn placement_csv(&self) -> String {
        let mut out = "Reference,Footprint,X_mm,Y_mm,Rotation_deg,Side\n".to_string();
        for f in &self.footprints {
            out.push_str(&format!(
                "{},{},{:.6},{:.6},{},Top\n",
                csv_cell(&f.reference),
                csv_cell(&f.style),
                f.at.x,
                f.at.y,
                (f.rotation % 4) * 90
            ));
        }
        out
    }
    /// Review drawing, not a replacement for Gerber manufacturing layers.
    pub fn svg(&self) -> Result<String, String> {
        self.validate()?;
        let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-2 -2 {} {}\" width=\"1200\"><rect x=\"-2\" y=\"-2\" width=\"{}\" height=\"{}\" fill=\"#111b29\"/><g transform=\"translate(0,{}) scale(1,-1)\"><rect width=\"{}\" height=\"{}\" rx=\"0.4\" fill=\"#142e34\" stroke=\"#40dcb6\" stroke-width=\"0.15\"/>",self.width+4.,self.height+4.,self.width+4.,self.height+4.,self.height,self.width,self.height);
        for t in &self.tracks {
            let color = if t.layer == Copper::Top {
                "#ffb969"
            } else {
                "#68b3ff"
            };
            let ps = t
                .points
                .iter()
                .map(|p| format!("{},{}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ");
            out.push_str(&format!("<polyline points=\"{ps}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",t.width));
        }
        for p in self.pads() {
            if p.rectangular {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f9d77e\"/>",
                    p.at.x - p.size.x / 2.,
                    p.at.y - p.size.y / 2.,
                    p.size.x,
                    p.size.y
                ));
            } else {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#f9d77e\"/>",
                    p.at.x,
                    p.at.y,
                    p.size.x / 2.
                ));
            }
            if p.drill > 0. {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#111b29\"/>",
                    p.at.x,
                    p.at.y,
                    p.drill / 2.
                ));
            }
        }
        for v in &self.vias {
            out.push_str(&format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#f9d77e\"/><circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#111b29\"/>",v.at.x,v.at.y,v.diameter/2.,v.at.x,v.at.y,v.drill/2.));
        }
        out.push_str("</g></svg>");
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use crate::{FootprintDefinition, Mm, PadSpec, Sheet};
    #[test]
    fn custom_footprints_roundtrip_rotate_and_export() {
        let mut s = Sheet::divider();
        let f = FootprintDefinition {
            name: "MY-SMD-2".into(),
            pads: vec![
                PadSpec {
                    at: Mm::new(-2., 0.),
                    size: Mm::new(1.2, 1.8),
                    rectangular: true,
                    drill: 0.,
                },
                PadSpec {
                    at: Mm::new(2., 0.),
                    size: Mm::new(1.2, 1.8),
                    rectangular: true,
                    drill: 0.,
                },
            ],
        };
        let f = FootprintDefinition::from_json(&f.to_json().unwrap()).unwrap();
        s.components[1].footprint = f.name.clone();
        s.components[1].custom_pads = f.pads.clone();
        assert!(s
            .pcb_drc()
            .iter()
            .any(|i| i.message.contains("out of sync")));
        s.sync_board();
        s.board.footprints[1].rotation = 1;
        let pad = &s.board.footprints[1].pads()[0];
        assert_eq!(pad.size, Mm::new(1.8, 1.2));
        assert_eq!(
            pad.at,
            Mm::new(s.board.footprints[1].at.x, s.board.footprints[1].at.y - 2.)
        );
        assert_eq!(Sheet::from_json(&s.to_json().unwrap()).unwrap(), s);
        assert!(s
            .board
            .gerbers()
            .unwrap()
            .iter()
            .find(|(n, _)| n == "board-F_Paste.gbr")
            .unwrap()
            .1
            .contains("D03"));
        assert!(s.board.placement_csv().contains("MY-SMD-2"));
        assert!(s.board.svg().unwrap().contains("<svg"));
        let mut bad = f;
        bad.pads[0].drill = 2.;
        assert!(bad.validate().is_err());
    }
}
