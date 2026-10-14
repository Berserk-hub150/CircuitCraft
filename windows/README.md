# Windows binaries

The complete ready-to-run package is linked from the repository README and stored in `downloads/`. Extract it and run `Start-CircuitCraft.bat`.

For a source build, run `python scripts/prepare_engine.py` followed by `scripts/Build-Windows.ps1`. These create the executable and unpack the pinned original ngspice engine into this folder. Python/py7zr and Rust are build tools; users of the complete ZIP do not need them.
