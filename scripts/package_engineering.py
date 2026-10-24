"""Package the current Windows build with its pinned engine, source and examples."""
from pathlib import Path
import hashlib
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
DEST = ROOT / "downloads" / f"CircuitCraft-{VERSION}-Windows-x64.zip"

def package():
    required = ["windows/circuitcraft.exe", "windows/circuitcraft-cli.exe",
                "windows/ngspice/Spice64/bin/ngspice_con.exe",
                "third-party/ngspice-47.tar.gz", "third-party/ngspice-win64.7z"]
    if any(not (ROOT / name).is_file() for name in required):
        raise SystemExit("Build Windows and prepare the pinned engine/source archives first.")
    files = [ROOT / name for name in ["Cargo.toml", "Cargo.lock", "Start-CircuitCraft.bat", "README.md",
             "LEGGIMI.md", "FEATURES.md", "ROADMAP.md", "VERIFICATION.md", "THIRD_PARTY.md", "LICENSE"]]
    for name in ["core", "desktop", "cli", "windows", "examples", "docs", "scripts", "verification"]:
        files.extend(p for p in (ROOT / name).rglob("*") if p.is_file() and "__pycache__" not in p.parts
                     and p.suffix not in [".pyc", ".ppm", ".assembling"])
    files.extend(ROOT / name for name in required if name.startswith("third-party/"))
    files.extend(ROOT / "third-party" / name for name in ["rust-dependencies.json", "rust-notices.zip"])
    files = sorted(set(files))
    # Prevent accidental distribution of internal partnership proposals.
    if any(p.name.upper().endswith('_PROPOSAL.MD') for p in files):
        raise SystemExit('Internal proposal in release inputs; remove before packaging.')
    hashes = "".join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(ROOT).as_posix()}\n" for p in files)
    DEST.parent.mkdir(exist_ok=True)
    with zipfile.ZipFile(DEST, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for p in files:
            z.write(p, p.relative_to(ROOT).as_posix())
        z.writestr("PACKAGE_SHA256SUMS.txt", hashes)
    checksum = hashlib.sha256(DEST.read_bytes()).hexdigest()
    (DEST.parent / f"SHA256SUMS-{VERSION}.txt").write_text(f"{checksum}  {DEST.name}\n", encoding="utf-8")
    with zipfile.ZipFile(DEST) as z:
        bad = z.testzip()
        if bad:
            raise SystemExit(f"ZIP verification failed: {bad}")
    print(f"Packaged {DEST.name}: {DEST.stat().st_size} bytes; SHA256 {checksum}")

if __name__ == "__main__":
    package()
