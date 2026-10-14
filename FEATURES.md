# CircuitCraft 0.2 — implemented scope

The working feature set is listed in README.md and exercised by the included tests. Source is provided for extension.

## Current limits
- One schematic sheet; one rectangular two-layer PCB.
- Fifteen schematic symbol kinds, seven base footprints and editable custom pad definitions. Graphical symbols expose 2–4 terminals.
- A footprint pad editor is implemented; no graphical symbol editor, automatic router, copper zones, push/shove, differential-pair constraints, length tuning, impedance rules or custom stack-up.
- No blind/buried vias, 3D PCB, STEP or native KiCad/Altium/EAGLE project import.
- No hierarchical schematic, digital/MCU co-simulation or S-parameter/Monte Carlo interface (noise has a graphical interface).
- Textual SPICE import does not perform graphical round-trip import. A four-pin wrapper exists; larger subcircuits use textual SPICE.
- Generic semiconductor models are demonstrations. Ideal op amp is a linear VCVS without rails or saturation.
- Built-in Rust simulation is linear DC only; advanced analyses run ngspice.
- Analysis settings / engine path are session settings.
- Symbol and footprint movement does not stretch existing wires/tracks.
- DRC implements the stated rules, not IPC certification or manufacturer signoff.
- Gerber export: copper top/bottom, mask top/bottom, top outlines and rectangular board edge. Plated drills only. Top SMD solder paste and footprint placement CSV are implemented. No paste reduction, assembly text or resolved purchasing/assembly part list.
- Project caps: 10,000 components / 30,000 wires. Geometry DRC refuses a clean result above 3,000 copper shapes; raw parser limits total data.
- Windows binaries are cross-compiled; their native execution and macOS execution remain unverified in this environment.

Full parity with all professional EDA products is not claimed. Missing features are future work, not placeholder buttons.
