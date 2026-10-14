"""Unpack the pinned original Windows ngspice engine. No download or execution."""
from pathlib import Path
import hashlib
import py7zr

root = Path(__file__).resolve().parents[1]
archive = root / "third-party/ngspice-win64.7z"
expected = "59225971bd68cdd1199443649aa4615a9e6d684933f205ab49006a3942518f5a"
if not archive.is_file():
    existing=root / "windows/ngspice/Spice64/bin/ngspice_con.exe"
    if existing.is_file() and hashlib.sha256(existing.read_bytes()).hexdigest()=="22d5cae2bd32b2e39157a8d27bf457122f68285b72a9ebefdf41551b628233ab":
        print("Pinned engine already unpacked in this Windows package.")
        raise SystemExit(0)
    from assemble_packages import assemble
    assemble("third-party/ngspice-win64.7z")
if hashlib.sha256(archive.read_bytes()).hexdigest() != expected:
    raise SystemExit("ngspice archive checksum mismatch")
with py7zr.SevenZipFile(archive) as package:
    package.extractall(root / "windows/ngspice")
if not (root / "windows/ngspice/Spice64/bin/ngspice_con.exe").is_file():
    raise SystemExit("Engine executable missing after extraction")
print("Pinned ngspice engine prepared.")
