# CircuitCraft 0.3.1

Maintenance release based on the 0.3 application series.

- Remove the unpublished third-party partnership proposal from current distribution sources.
- Clarify independent branding and remove third-party references from the logo documentation.
- Improve Windows release reproducibility, published-tag protection and package safeguards.
- Add a dependency audit, Clippy, Dependabot configuration and a security reporting policy.
- Retain the 0.3 features: circuit schematic editor, ngspice simulations, two-layer autorouting, hierarchical pages, PCB 3D review and manufacturing exports.

**Limitations:** The component library is limited; autorouting may remain partial and 3D component bodies approximate. Windows CI tests compilation, the CLI and ngspice but does not prove GUI launch or native file-dialog behavior.

Extract the entire Windows ZIP and start `Start-CircuitCraft.bat`; follow `docs/FIRST_CIRCUIT.md`. Compare its SHA256 checksum against the published manifest. No signed installer is claimed. ngspice is a separately licensed component.
