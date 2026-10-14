# Downloads

Get the complete ready-to-run Windows ZIP from the repository's **Releases** page. The publication workflow reconstructs and verifies the packages stored here as parts.

For an offline source checkout:
```sh
python scripts/assemble_packages.py
```
This produces the same complete Windows ZIP and the original ngspice Windows/source archives, checking the SHA256 of every part and final file. It does not execute downloaded binaries.

Package metadata: `packages.json`. SHA256 checksums: `SHA256SUMS.txt`.
