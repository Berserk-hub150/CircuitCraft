# CircuitCraft 0.2.0

Native Rust schematic editing, ngspice simulation and two-layer PCB layout.

New: component parameter sweeps, automatic measurements, noise spectra, complex signal math, custom footprint pad editing, placement CSV, PCB SVG and top SMD paste Gerber.

The complete Windows ZIP includes desktop/CLI executables, the original ngspice 47 distribution, source, examples and notices. Extract everything and launch Start-CircuitCraft.bat.

18 local tests passed, including real ngspice 42 numerical checks and egui interaction checks. Four example boards pass the implemented ERC/DRC rules; manufacturing layers were parsed independently. Windows binaries were cross-compiled; native GUI execution remains unverified.

ngspice is a separate C engine, with its original licenses. CircuitCraft code is MIT. Full parity with every professional EDA product is not claimed.

SHA256SUMS.txt verifies the complete Windows ZIP and the two original engine/source archives. The source repository stores checked package parts; scripts/assemble_packages.py reconstructs their exact original bytes.
