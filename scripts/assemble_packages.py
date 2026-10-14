"""Reassemble pinned repository packages; validate every part and final SHA256."""
from pathlib import Path
import argparse
import hashlib
import json

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def local_path(name):
    path = (ROOT / name).resolve()
    if not path.is_relative_to(ROOT):
        raise ValueError("Package path outside project")
    return path

def assemble(only=None):
    manifest = json.loads((ROOT / "downloads/packages.json").read_text())
    selected = [p for p in manifest["packages"] if only is None or p["path"] == only]
    if not selected:
        raise ValueError("Unknown package")
    for package in selected:
        target = local_path(package["path"])
        if target.is_file():
            if digest(target) != package["sha256"]:
                raise ValueError(f"Existing package differs: {package['path']}")
            print(f"Verified existing {package['path']}")
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + ".assembling")
        try:
            with temporary.open("wb") as output:
                for part in package["parts"]:
                    source = local_path(part["path"])
                    if source.stat().st_size != part["bytes"] or digest(source) != part["sha256"]:
                        raise ValueError(f"Invalid package part: {part['path']}")
                    with source.open("rb") as source_file:
                        for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
                            output.write(chunk)
            if temporary.stat().st_size != package["bytes"] or digest(temporary) != package["sha256"]:
                raise ValueError(f"Reassembled checksum mismatch: {package['path']}")
            temporary.replace(target)
        except Exception:
            temporary.unlink(missing_ok=True)
            raise
        print(f"Assembled and verified {package['path']}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", help="One exact package path from the manifest")
    args = parser.parse_args()
    assemble(args.only)
