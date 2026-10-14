# CircuitCraft roadmap

The implemented release is documented in README and FEATURES. Items below are proposed work, not existing features or delivery promises.

| Priority | Milestone | Acceptance evidence |
|---|---|---|
| 1 | Native Windows validation and reproducible packages | Launch/file-picker checks on Windows 10/11; engine reports; signed release checksums |
| 2 | Hierarchical sheets and arbitrary-pin symbols | Cross-sheet net fixtures; deterministic netlist; symbol/footprint pin mapping tests |
| 3 | Copper zones and stronger PCB constraints | Zone connectivity and clearance fixtures; independent CAM review; differential-pair/length rules |
| 4 | Placement sides, richer footprints and assembly output | Top/bottom assembly examples; vendor footprint checks; paste/assembly layers and part metadata |
| 5 | Router and 3D review | Routing quality benchmarks on published boards; verified geometry and STEP export |
| 6 | Broader simulation workflows | Monte Carlo/tolerance studies, model compatibility corpus, FFT/THD and digital integration benchmarks |

For performance comparisons, publish the exact circuits, models, solver options, machine, versions, wall time and numerical tolerances. A broad superiority claim requires those reproducible comparisons.
