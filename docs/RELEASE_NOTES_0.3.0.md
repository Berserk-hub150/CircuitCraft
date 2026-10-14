# CircuitCraft 0.3.0

Adds three engineering workflows to the existing Rust desktop editor:

- Bounded two-layer A* autorouter with exact pad endpoints, copper/edge clearance, plated vias, preservation of manual routing, cancellation and undo. Accepted routes pass the implemented geometry DRC; unresolved connections remain visible.
- Embedded hierarchical schematic pages, editable instance names and explicit child-label/parent-net port bindings. Local labels are isolated between pages, ground is global, and deterministic flattening feeds simulation, sweeps, BOM and the shared PCB.
- Interactive PCB 3D review with orbit/zoom/pan, top/bottom/isometric views, editable substrate thickness, copper and component-envelope visibility, and millimetre-scale OBJ export.

See docs/ENGINEERING_WORKFLOWS.md for controls, bounds and examples. The 3D component bodies are approximate footprint envelopes; the substrate is a rectangular solid without drilled cutouts. STEP, vendor mechanical models, copper pours and push/shove remain future work.

Local verification: 28 tests passed with real Linux ngspice 42 runs, including forced layer transitions, hierarchy electrical checks, save/undo and stale-worker rejection. GUI screenshots are real egui frames, rendered in software. The release job additionally runs native Windows compilation, tests and bundled ngspice 47 CLI checks before publishing. Interactive OS desktop launch is not covered by those checks.

Extract the entire Windows ZIP and launch Start-CircuitCraft.bat. The ZIP includes current desktop/CLI executables, the pinned ngspice engine, source, examples, documentation and dependency notices. SHA256SUMS-0.3.0.txt verifies the ZIP; PACKAGE_SHA256SUMS.txt verifies its contents.
