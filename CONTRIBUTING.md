# Contributing

Use Rust 1.88 or later. Install ngspice to exercise numerical tests.

```sh
cargo fmt --all --check
CIRCUITCRAFT_NGSPICE_TESTS=1 cargo test --workspace
cargo build --workspace --profile dist
```

Keep numerical and manufacturing changes backed by independent expected values or a format parser. Add a small project or netlist that reproduces a bug. Report the OS, application revision, engine version, expected result and actual result. Never replace an unsupported feature with a success message.

Large third-party engine packages are pinned by SHA256. Retain their original licenses and notices. CircuitCraft changes use MIT. API/schema changes should preserve older project files or document a migration.
