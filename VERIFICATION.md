# Verification — 2026-10-14
**18 tests passed, zero failures:** 13 core unit tests, 1 numerical integration test and 4 desktop tests, including actual ngspice runs.

| Check | Result |
|---|---|
| Rust linear DC | 10 V, 1 kΩ / 1 kΩ divider: 5 V, 5 mA |
| ngspice 42 operating point | 5.000000000 V output |
| ngspice 42 AC | At 159.154943 Hz: 0.707106781 magnitude and −45.000000° |
| ngspice 42 transient | At 0.001002583 s: 0.633072230 V; analytic 0.633069533 V |
| DC sweep | Maximum error versus VIN/2: 8.882e−16 V |
| Relative models | Include directory and filename containing spaces; 3 V buffer result |
| Nonlinear example | Rectifier transient: 2,014 samples over 200 ms |
| Electrical graph | Crossing separation/junctions, remote labels, divider topology, ERC |
| Project/history | JSON round-trip, unique IDs, undo/redo |
| PCB | Routing reduces unrouted nets; shorts detected; all three demo boards pass implemented rules; stale footprints detected |
| GUI | Pointer-event placement, wiring, undo, pad snapping and routing through actual egui frames |
| Rendering | All five workspaces plus sweep and noise scenarios rendered from real egui meshes and reviewed |
| Exchange | Independent gerbonara parser: all seven Gerber layers and correct hole counts/coordinates in four examples, including a custom SMD footprint; SVG and BOM also parsed |
| Windows | Desktop and CLI compiled for x86_64-pc-windows-gnu; imports inspected |
| Parameter sweep | R1 = 500 / 1,000 / 2,000 Ω: cutoff 318.295902 / 159.154945 / 79.588397 Hz; within 0.1% of analytic RC values |
| Noise | Johnson thermal-noise density agrees with sqrt(4 k T R) and RC transfer within 0.01%; output band-integrated noise 64.214251 nV RMS |
| Waveform statistics | Nonuniformly sampled unit ramp: mean 0.5, RMS sqrt(1/3), 10–90% time 0.8 s, settling 0.98 s |
| Complex math | Analytic complex ratio and zero-denominator rejection; CLI gain export |
| Custom pads | JSON round-trip, geometry rotation, stale-PCB check, import/export/undo in GUI, paste and assembly exports |
| CLI | Sweep, noise, gain and transient CSV + measurement reports generated; four reference designs pass ERC/DRC |
| Original ngspice 47 | Windows and source downloads match their pinned official SHA256 |

Reports are in `verification/`. Screenshots are actual interface renders, not mockups.

**Native Windows executable / file-picker execution has not been tested in this environment.** UI checks use real egui layouts and events with software rasterization, rather than an operating-system desktop run. macOS runtime is untested. ngspice numerical tests here use Linux version 42; the bundled Windows version is the official 47 release.

This validates the examples and implemented checks, not every possible circuit, vendor model, manufacturing process or commercial-suite feature.

Reproduce:
```sh
CIRCUITCRAFT_NGSPICE_TESTS=1 cargo test --workspace
cargo run -p circuitcraft-cli -- check-engine report.txt
python -m pip install gerbonara
python scripts/verify_fabrication.py
```

Windows:
```powershell
.\scripts\Test-Windows-Engine.ps1
```
