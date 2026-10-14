# Engineering workflows — CircuitCraft 0.3

## Autorouter

1. Open **PCB layout** and **Synchronize from schematic**. Place/rotate the footprints inside the board.
2. Run DRC and fix geometry, footprint synchronization, shorts and clearance errors. Unrouted connections are expected.
3. Choose the track width, an autorouter grid of 0.1–2 mm and whether to allow plated vias. Click **Route unrouted connections**.
4. The worker retains existing tracks and vias. It routes on both layers, respects top-only SMD pads and uses plated vias for layer changes. Track endpoints remain at the actual pad centres, even off-grid.
5. Inspect the report and remaining ratsnest, then rerun DRC. **Cancel router** retains the original board. Undo restores the previous routing in one action. Editing the project discards an obsolete worker result.

A* uses orthogonal grid steps, exact copper/edge clearance tests and a 12-step via penalty. Defaults: 0.5 mm grid, 0.3 mm width, 0.8/0.4 mm via diameter/drill. No rip-up, push/shove, automatic placement or route-completion guarantee. It tries at most 64 connections by default; library caps are 128 connections, 500,000 grid states, 1,500 initial copper shapes and 500,000 search expansions per connection. Fine grids on large boards are rejected explicitly. Partially completed work is reported as partial, never as a clean completed board.

CLI: `circuitcraft-cli autoroute input.cforge routed.cforge --grid 0.5 --width 0.3`. A partial route is saved with an error exit status and remaining-count report.

## Hierarchical pages

1. Open **Examples → Hierarchical divider**. The root contains the supply and upper resistor; **LOAD** contains the lower resistor.
2. In the schematic sidebar, expand **HIERARCHICAL PAGES**, expand LOAD and click **Open page**. **↑ Parent page** returns.
3. Use **New page** to create an embedded child or **Import…** to reuse a graphical `.cforge` project. Each sibling instance name must be unique, using at most 24 letters/digits/underscores.
4. Place named labels on parent and child nets. In the parent, expand the child and click **Add port binding**. Enter the **Child label** and **Parent net**. The demo binds child IN to parent OUT. Child labels otherwise remain local; all electrical grounds are node 0.
5. Run simulation for the whole project. Save from any page: the full tree is saved. Undo/redo restores the full project and returns to the root. PCB, BOM and SPICE workspaces operate on the root and all descendants.

Nested limits: 8 levels, 64 pages, 10,000 combined components. Flattening gives stable component IDs and SPICE references such as `R_LOAD_R1`; unbound internal nets receive private names. Invalid/missing bindings and bindings that would short distinct parent nets are rejected. Imported pages use the root include directory; child textual-netlist mode is rejected. Model/parameter definitions share one SPICE namespace. Pages are embedded instances, not linked external sheets or graphical sheet-box symbols. Symbols still have the existing 2–4 pin definitions. Root PCB placement/routing is shared; child PCB data is not imported into it. Existing version-1 single-sheet projects still open and migrate to version 2, which older CircuitCraft builds reject rather than silently losing child pages.

The CLI `examples DIRECTORY` includes the hierarchical divider. `simulate ... linear`, ngspice simulations, sweeps, BOM exports and PCB synchronization use the same flattening.

## PCB 3D review

Open **PCB 3D**. Drag to orbit, use the wheel to zoom and middle-drag to pan. The sidebar provides **Isometric**, **Top**, **Bottom**, layer visibility and component envelopes. Thickness is saved, defaults to 1.6 mm and supports 0.2–10 mm. Choose **Export 3D OBJ…**, or `circuitcraft-cli pcb3d project.cforge board.obj`.

The shared triangle mesh uses the PCB's pad coordinates, footprint rotation, track widths, copper layers, vias and board dimensions. OBJ coordinates are millimetres. Copper is shown at nominal 35 μm thickness. The substrate is a rectangular solid; holes are not subtracted from it. Round drilled pads/vias have ring geometry; rectangular pads are solid envelopes. Component bodies are approximate footprint-derived envelopes without verified heights or manufacturer shapes. There is no STEP export or mechanical clearance validation. Review the electrical board using DRC and manufacturing layers independently.
