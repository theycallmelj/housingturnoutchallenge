#!/usr/bin/env python3
"""Build data/derived/completion_factors.csv from the Census Survey of Construction.

For each Census region (and the U.S.), how permits turn into completed homes:

* completion_ratio: units completed in 2016-2025 / units authorized in
  2015-2024 (completions lag permits about a year). Long windows smooth out
  cycles. Above 1 is possible because completions also count places that
  don't issue permits; the model caps it at 1.
* months_permit_to_completion_1unit / _2plus: average months from permit to
  start plus start to completion, averaged over 2021-2025, for buildings with
  one unit and with two or more units.

Sources (U.S. government works, public domain; provenance file
research/data/census-survey-of-construction.md): the New Residential
Construction spreadsheets at https://www.census.gov/construction/nrc/xls/.

Raw downloads go to research/data/raw/ (gitignored), checksums appended to
research/data/raw/CHECKSUMS-timing.txt. Standard library only (xlsx is read
with zipfile + xml).

    python3 scripts/fetch_construction_timing.py
"""

import hashlib
import sys
import urllib.request
import xml.etree.ElementTree as ET
import zipfile
from datetime import date
from io import BytesIO
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "research" / "data" / "raw"
OUT = ROOT / "data" / "derived" / "completion_factors.csv"
BASE = "https://www.census.gov/construction/nrc/xls/"
FILES = ["permits_cust.xlsx", "comps_cust.xlsx", "avg_authtostart_cust.xlsx", "avg_starttocomp_cust.xlsx"]
NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}
REGIONS = ["united_states", "northeast", "midwest", "south", "west"]
LAG_YEARS = [2021, 2022, 2023, 2024, 2025]
COMP_YEARS = range(2016, 2026)  # completions 2016-2025 vs permits one year earlier

checksums = []


def fetch(name: str) -> bytes:
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / f"nrc_{name}"
    if not path.exists():
        req = urllib.request.Request(BASE + name, headers={"User-Agent": "Mozilla/5.0 housing-challenge-timing/1.0"})
        with urllib.request.urlopen(req, timeout=120) as resp:
            path.write_bytes(resp.read())
    data = path.read_bytes()
    if data[:2] != b"PK":
        raise SystemExit(f"{name} is not an xlsx file")
    checksums.append((path.name, hashlib.sha256(data).hexdigest()))
    return data


def sheets(data: bytes) -> dict:
    """{sheet name: rows of cell strings} from an xlsx."""
    z = zipfile.ZipFile(BytesIO(data))
    shared = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")).findall("m:si", NS):
            shared.append("".join(t.text or "" for t in si.iter(f"{{{NS['m']}}}t")))
    names = [s.get("name") for s in ET.fromstring(z.read("xl/workbook.xml")).find("m:sheets", NS)]
    out = {}
    for i, name in enumerate(names, 1):
        rows = []
        for r in ET.fromstring(z.read(f"xl/worksheets/sheet{i}.xml")).iter(f"{{{NS['m']}}}row"):
            row = []
            for c in r.findall("m:c", NS):
                v = c.find("m:v", NS)
                val = v.text if v is not None else ""
                if c.get("t") == "s" and val:
                    val = shared[int(val)]
                row.append(val)
            rows.append(row)
        out[name] = rows
    return out


def annual_by_region(rows) -> dict:
    """Year -> {region: total units} from an 'Annual' sheet (thousands)."""
    # Columns: Year, US total, US 1 unit, US 2-4, US 5+, NE total, NE 1, MW total, MW 1, S total, S 1, W total
    cols = {"united_states": 1, "northeast": 5, "midwest": 7, "south": 9, "west": 11}
    out = {}
    for r in rows:
        if r and r[0][:4].isdigit() and len(r) > 11:
            out[int(r[0][:4])] = {k: float(r[i]) for k, i in cols.items()}
    return out


def months_by_type(rows) -> dict:
    """Year -> (1-unit months, 2+-unit months) from a timing sheet."""
    out = {}
    for r in rows:
        if r and r[0][:4].isdigit() and len(r) > 5:
            try:
                out[int(r[0][:4])] = (float(r[1]), float(r[5]))
            except ValueError:
                pass
    return out


def main() -> int:
    permits = annual_by_region(sheets(fetch("permits_cust.xlsx"))["Annual"])
    comps = annual_by_region(sheets(fetch("comps_cust.xlsx"))["Annual"])
    a2s = sheets(fetch("avg_authtostart_cust.xlsx"))
    s2c = sheets(fetch("avg_starttocomp_cust.xlsx"))
    sheet_name = {"united_states": "United States", "northeast": "Northeast", "midwest": "Midwest", "south": "South", "west": "West"}

    OUT.parent.mkdir(parents=True, exist_ok=True)
    lines = ["region,completion_ratio,months_permit_to_completion_1unit,months_permit_to_completion_2plus,ratio_window,months_window"]
    for reg in REGIONS:
        c = sum(comps[y][reg] for y in COMP_YEARS)
        p = sum(permits[y - 1][reg] for y in COMP_YEARS)
        ta = months_by_type(a2s[sheet_name[reg]])
        ts = months_by_type(s2c[sheet_name[reg]])
        m1 = sum(ta[y][0] + ts[y][0] for y in LAG_YEARS) / len(LAG_YEARS)
        m2 = sum(ta[y][1] + ts[y][1] for y in LAG_YEARS) / len(LAG_YEARS)
        lines.append(f"{reg},{c / p:.4f},{m1:.2f},{m2:.2f},completions 2016-2025 / permits 2015-2024,2021-2025 average")
    OUT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT.relative_to(ROOT)}")
    print("\n".join(lines))
    (RAW / "CHECKSUMS-timing.txt").write_text(
        f"accessed {date.today().isoformat()}\n" + "".join(f"{h}  {n}\n" for n, h in sorted(set(checksums)))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
