"""Independent validation; install gerbonara as a development dependency."""
from pathlib import Path
from gerbonara import GerberFile, ExcellonFile
import csv, json, xml.etree.ElementTree as ET
root = Path(__file__).resolve().parents[1]
report = []
for p in (root / "examples").glob("*.cforge"):
    data = json.loads(p.read_text())
    base = p.with_suffix("")
    ET.parse(base / "schematic.svg")
    with (base / "bom.csv").open(newline="") as f:
        def component_count(page):
            return len(page["components"]) + sum(component_count(child["sheet"]) for child in page.get("children", []))
        assert len(list(csv.DictReader(f))) == component_count(data)
    layers = {g.name: len(GerberFile.open(g).objects)
              for g in (base / "fabrication").glob("*.gbr")}
    assert len(layers) == 7
    d = ExcellonFile.open(base / "fabrication/board-PTH.drl")
    expected_holes=[]
    for part in data["board"]["footprints"]:
        custom=part.get("custom_pads",[])
        offsets=[(p["at"]["x"],p["at"]["y"]) for p in custom if p["drill"]>0] if custom else [(-2.54,0),(2.54,0)]
        for x,y in offsets:
            for _ in range(part["rotation"]%4):x,y=-y,x
            expected_holes.append((part["at"]["x"]+x,part["at"]["y"]+y))
    expected_holes.extend((via["at"]["x"],via["at"]["y"]) for via in data["board"]["vias"])
    assert len(d.objects)==len(expected_holes)
    for x,y in expected_holes:
        assert any(abs(o.x-x)<1e-6 and abs(o.y-y)<1e-6 for o in d.objects)
    ET.parse(base / "board.svg")
    with (base / "placement.csv").open(newline="") as f:
        assert len(list(csv.DictReader(f)))==len(data["board"]["footprints"])
    report.append({"project": p.name, "layers": layers,
                   "holes": len(d.objects), "status": "PASS"})
print(json.dumps(report, indent=2))
