# Third-party distribution
## ngspice 47
Redistributed separately as its original unmodified Windows engine distribution.
- Project: https://ngspice.sourceforge.io/download.html
- Included original archive: `third-party/ngspice-win64.7z` (unpack with `scripts/prepare_engine.py`).
- Windows archive SHA256: `59225971bd68cdd1199443649aa4615a9e6d684933f205ab49006a3942518f5a`
- Included corresponding source: `third-party/ngspice-47.tar.gz`
- Source: https://sourceforge.net/projects/ngspice/files/ng-spice-rework/47/ngspice-47.tar.gz/download
- Source SHA256: `894e649651f1838a14095e5a5439e7d3aa63e87ede14d283173fda4fcdef675f`
- Original notices: `windows/ngspice/Spice64/docs/COPYING` and original source.

The main ngspice license is Modified BSD; its distribution lists additional LGPL/GPL/MPL/public-domain components. The manual is CC-BY-SA 4.0. Original DLLs, examples and models retain their own terms. These are not relicensed under CircuitCraft's MIT license.

## Rust dependencies
Precise versions are locked in Cargo.lock. Notices are archived in `third-party/rust-notices.zip` (expanded under `third-party/rust-licenses` in the Windows download); metadata is in `third-party/rust-dependencies.json`. Rust/MinGW are build tools. gerbonara is an independent verification tool, not an app dependency.

Original archives are also published as release assets. Repository package parts are reconstructed with `python scripts/assemble_packages.py`; final SHA256 values match those listed above.
