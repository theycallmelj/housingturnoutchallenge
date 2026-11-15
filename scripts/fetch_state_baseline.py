#!/usr/bin/env python3
"""Build data/derived/state_baseline.csv from public Census files.

Sources (both U.S. government works, public domain; see the provenance files
research/data/census-building-permits-survey.md and
research/data/census-housing-unit-estimates.md):

* Building Permits Survey, annual state files  st{YEAR}a.txt  (2021-2025)
* Population Estimates Program, vintage 2025   NST-EST2025-ALLDATA.csv

Raw downloads go to research/data/raw/ (gitignored) with SHA-256 checksums in
research/data/raw/CHECKSUMS.txt. Run from anywhere:

    python3 scripts/fetch_state_baseline.py

Standard library only.
"""

import csv
import hashlib
import io
import sys
import urllib.request
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "research" / "data" / "raw"
OUT = ROOT / "data" / "derived" / "state_baseline.csv"

YEARS = [2021, 2022, 2023, 2024, 2025]
BPS_URL = "https://www2.census.gov/econ/bps/State/st{year}a.txt"
PEP_URL = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/state/totals/NST-EST2025-ALLDATA.csv"

# FIPS -> USPS abbreviation for the 50 states and DC.
ABBR = {
    "01": "AL", "02": "AK", "04": "AZ", "05": "AR", "06": "CA", "08": "CO", "09": "CT", "10": "DE",
    "11": "DC", "12": "FL", "13": "GA", "15": "HI", "16": "ID", "17": "IL", "18": "IN", "19": "IA",
    "20": "KS", "21": "KY", "22": "LA", "23": "ME", "24": "MD", "25": "MA", "26": "MI", "27": "MN",
    "28": "MS", "29": "MO", "30": "MT", "31": "NE", "32": "NV", "33": "NH", "34": "NJ", "35": "NM",
    "36": "NY", "37": "NC", "38": "ND", "39": "OH", "40": "OK", "41": "OR", "42": "PA", "44": "RI",
    "45": "SC", "46": "SD", "47": "TN", "48": "TX", "49": "UT", "50": "VT", "51": "VA", "53": "WA",
    "54": "WV", "55": "WI", "56": "WY",
}


def fetch(url: str, name: str) -> bytes:
    """Download once into research/data/raw/ and return the bytes."""
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / name
    if not path.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "housing-challenge-baseline/1.0"})
        with urllib.request.urlopen(req, timeout=60) as resp:
            path.write_bytes(resp.read())
    return path.read_bytes()


def permits_by_state(text: str) -> dict:
    """Total units authorized per state from one annual BPS file.

    Columns 6-17 are the imputed estimates (reported plus imputed for
    non-reporting places); the later "rep" columns are reported-only.
    Units columns: 1-unit=6, 2-units=9, 3-4 units=12, 5+ units=15.
    """
    out = {}
    for row in csv.reader(io.StringIO(text)):
        if len(row) < 18 or not row[0].strip().isdigit():
            continue
        fips = row[1].strip()
        if fips not in ABBR:
            continue
        units = [int(row[i]) for i in (6, 9, 12, 15)]
        out[fips] = {"total": sum(units), "u1": units[0], "u2": units[1], "u3_4": units[2], "u5p": units[3]}
    return out


def main() -> int:
    checksums = []
    permits = {}
    for year in YEARS:
        name = f"bps_st{year}a.txt"
        data = fetch(BPS_URL.format(year=year), name)
        checksums.append((name, hashlib.sha256(data).hexdigest()))
        permits[year] = permits_by_state(data.decode("latin-1"))
        missing = set(ABBR) - set(permits[year])
        if missing:
            print(f"error: {year} file is missing states {sorted(missing)}", file=sys.stderr)
            return 1

    pep = fetch(PEP_URL, "NST-EST2025-ALLDATA.csv")
    checksums.append(("NST-EST2025-ALLDATA.csv", hashlib.sha256(pep).hexdigest()))
    population = {
        r["STATE"]: int(r["POPESTIMATE2025"])
        for r in csv.DictReader(io.StringIO(pep.decode("latin-1")))
        if r["SUMLEV"] == "040" and r["STATE"] in ABBR
    }

    OUT.parent.mkdir(parents=True, exist_ok=True)
    header = ["fips", "abbr", "name", "population_2025"]
    header += [f"permits_{y}" for y in YEARS]
    header += ["permits_2025_1unit", "permits_2025_2unit", "permits_2025_3to4unit", "permits_2025_5plus"]
    names = {}
    for line in fetch(BPS_URL.format(year=YEARS[-1]), f"bps_st{YEARS[-1]}a.txt").decode("latin-1").splitlines():
        parts = line.split(",")
        if len(parts) > 4 and parts[1].strip() in ABBR:
            names[parts[1].strip()] = parts[4].strip()
    with OUT.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(header)
        for fips in sorted(ABBR):
            latest = permits[YEARS[-1]][fips]
            w.writerow(
                [fips, ABBR[fips], names[fips], population[fips]]
                + [permits[y][fips]["total"] for y in YEARS]
                + [latest["u1"], latest["u2"], latest["u3_4"], latest["u5p"]]
            )

    (RAW / "CHECKSUMS.txt").write_text(
        f"accessed {date.today().isoformat()}\n" + "".join(f"{h}  {n}\n" for n, h in checksums)
    )
    print(f"wrote {OUT.relative_to(ROOT)} ({len(ABBR)} rows: 50 states + DC)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
