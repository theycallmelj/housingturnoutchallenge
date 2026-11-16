#!/usr/bin/env python3
"""Build the county and state baseline tables from public Census files.

Outputs (committed; small):

* data/derived/county_baseline.csv  one row per county: permits 2021-2025 and
  2025 by building size, 2025 population, housing units by structure type
* data/derived/state_baseline.csv   one row per state: the SUM of its counties'
  permits and housing units, plus state population and Census region

Sources (all U.S. government works, public domain; provenance files in
research/data/):

* Building Permits Survey, annual county files  co{YEAR}a.txt  (2021-2025)
  -> census-building-permits-survey.md
* Building Permits Survey, annual state files   st{YEAR}a.txt  (reconciliation
  check only: county sums must match the state totals)
* Population Estimates, vintage 2025, state and county totals
  -> census-population-estimates.md
* ACS 5-year 2020-2024, table B25024 (units in structure), table-based summary
  file -> acs.md

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
OUT_STATE = ROOT / "data" / "derived" / "state_baseline.csv"
OUT_COUNTY = ROOT / "data" / "derived" / "county_baseline.csv"

YEARS = [2021, 2022, 2023, 2024, 2025]
BPS_COUNTY = "https://www2.census.gov/econ/bps/County/co{year}a.txt"
BPS_STATE = "https://www2.census.gov/econ/bps/State/st{year}a.txt"
PEP_STATE = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/state/totals/NST-EST2025-ALLDATA.csv"
PEP_COUNTY = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/counties/totals/co-est2025-alldata.csv"
ACS_B25024 = "https://www2.census.gov/programs-surveys/acs/summary_file/2024/table-based-SF/data/5YRData/acsdt5y2024-b25024.dat"

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
REGION = {"1": "northeast", "2": "midwest", "3": "south", "4": "west"}

checksums = []


def fetch(url: str, name: str) -> bytes:
    """Download once into research/data/raw/, record its checksum, return the bytes."""
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / name
    if not path.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "housing-challenge-baseline/1.0"})
        with urllib.request.urlopen(req, timeout=120) as resp:
            path.write_bytes(resp.read())
    data = path.read_bytes()
    checksums.append((name, hashlib.sha256(data).hexdigest()))
    return data


def bps_rows(text: str, county: bool):
    """Rows of an annual BPS file. Columns after the geography hold the imputed
    estimates (reported plus imputed) for 1-unit, 2-unit, 3-4 unit and 5+ unit
    buildings as Bldgs, Units, Value triples; the later "rep" columns are
    reported-only and unused."""
    first = 6 if county else 5  # county files add a county FIPS column
    for row in csv.reader(io.StringIO(text)):
        if len(row) < first + 12 or not row[0].strip().isdigit():
            continue
        units = [int(row[first + 1 + 3 * k]) for k in range(4)]
        yield row, units


def main() -> int:
    counties = {}  # (state, county) -> dict
    for year in YEARS:
        text = fetch(BPS_COUNTY.format(year=year), f"bps_co{year}a.txt").decode("latin-1")
        for row, units in bps_rows(text, county=True):
            st, co = row[1].strip(), row[2].strip()
            if st not in ABBR:
                continue
            c = counties.setdefault(
                (st, co),
                {"name": row[5].strip(), "region": REGION.get(row[3].strip(), ""), "permits": {}},
            )
            c["permits"][year] = sum(units)
            if year == YEARS[-1]:
                c["mix"] = units
                c["name"] = row[5].strip()

    # Reconciliation: county sums must equal each state's own annual total.
    worst = 0
    for year in YEARS:
        text = fetch(BPS_STATE.format(year=year), f"bps_st{year}a.txt").decode("latin-1")
        for row, units in bps_rows(text, county=False):
            st = row[1].strip()
            if st not in ABBR:
                continue
            county_sum = sum(c["permits"].get(year, 0) for (s, _), c in counties.items() if s == st)
            worst = max(worst, abs(county_sum - sum(units)))
    print(f"county sums vs state totals: largest difference {worst} units")
    if worst > 0:
        print("error: county permits do not sum to state totals", file=sys.stderr)
        return 1

    for c in counties.values():
        c["permit_issuing"] = 1

    # Every county with residents, including the ~100 rural counties where no
    # place issues building permits (absent from the BPS county files). They
    # get zero permits and permit_issuing = 0 so populations add up.
    state_region = {st: c["region"] for (st, _), c in counties.items() if c["region"]}
    pep_c = fetch(PEP_COUNTY, "co-est2025-alldata.csv").decode("latin-1")
    for r in csv.DictReader(io.StringIO(pep_c)):
        key = (r["STATE"], r["COUNTY"])
        if r["SUMLEV"] != "050" or r["STATE"] not in ABBR:
            continue
        if key not in counties:
            counties[key] = {
                "name": r["CTYNAME"], "region": state_region.get(r["STATE"], ""),
                "permits": {}, "permit_issuing": 0,
            }
        counties[key]["population"] = int(r["POPESTIMATE2025"])

    pep_s = fetch(PEP_STATE, "NST-EST2025-ALLDATA.csv").decode("latin-1")
    state_pop = {
        r["STATE"]: int(r["POPESTIMATE2025"])
        for r in csv.DictReader(io.StringIO(pep_s))
        if r["SUMLEV"] == "040" and r["STATE"] in ABBR
    }

    # ACS B25024 (units in structure), county rows: GEO_ID 0500000USssccc.
    # E001 total, E002 1 detached, E003 1 attached, E004 2, E005 3-4,
    # E006-E009 5+, E010 mobile home, E011 boat/RV/van.
    acs = fetch(ACS_B25024, "acsdt5y2024-b25024.dat").decode("latin-1")
    lines = acs.splitlines()
    head = lines[0].split("|")
    col = {name: i for i, name in enumerate(head)}
    for line in lines[1:]:
        if not line.startswith("0500000US"):
            continue
        f = line.split("|")
        key = (f[0][9:11], f[0][11:14])
        if key not in counties:
            continue
        e = lambda k: int(f[col[f"B25024_E{k:03d}"]] or 0)  # noqa: E731
        counties[key]["stock"] = {
            "total": e(1),
            "sf_detached": e(2),
            "sf_attached": e(3),
            "u2to4": e(4) + e(5),
            "u5plus": e(6) + e(7) + e(8) + e(9),
        }

    OUT_COUNTY.parent.mkdir(parents=True, exist_ok=True)
    stock_cols = ["hu_total", "hu_sf_detached", "hu_sf_attached", "hu_2to4", "hu_5plus"]
    stock_keys = ["total", "sf_detached", "sf_attached", "u2to4", "u5plus"]
    mix_cols = ["permits_2025_1unit", "permits_2025_2unit", "permits_2025_3to4unit", "permits_2025_5plus"]
    missing_pop = missing_stock = 0
    with OUT_COUNTY.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["state_fips", "county_fips", "abbr", "name", "region", "permit_issuing", "population_2025"]
                   + [f"permits_{y}" for y in YEARS] + mix_cols + stock_cols)
        for (st, co) in sorted(counties):
            c = counties[(st, co)]
            missing_pop += "population" not in c
            missing_stock += "stock" not in c
            w.writerow([st, co, ABBR[st], c["name"], c["region"], c["permit_issuing"], c.get("population", "")]
                       + [c["permits"].get(y, 0) for y in YEARS]
                       + c.get("mix", [0, 0, 0, 0])
                       + [c.get("stock", {}).get(k, "") for k in stock_keys])
    no_permits = sum(1 for c in counties.values() if not c["permit_issuing"])
    print(f"wrote {OUT_COUNTY.relative_to(ROOT)} ({len(counties)} rows; {no_permits} counties issue no permits; "
          f"{missing_pop} permit records without population and {missing_stock} without ACS stock "
          f"(retired geographies and balance-of-state records))")

    with OUT_STATE.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["fips", "abbr", "name", "region", "population_2025", "counties"]
                   + [f"permits_{y}" for y in YEARS] + mix_cols + stock_cols)
        names = {}
        for line in fetch(BPS_STATE.format(year=YEARS[-1]), f"bps_st{YEARS[-1]}a.txt").decode("latin-1").splitlines():
            parts = line.split(",")
            if len(parts) > 4 and parts[1].strip() in ABBR:
                names[parts[1].strip()] = parts[4].strip()
        for st in sorted(ABBR):
            cs = [c for (s, _), c in counties.items() if s == st]
            region = next((c["region"] for c in cs if c["region"]), "")
            w.writerow([st, ABBR[st], names[st], region, state_pop[st], len(cs)]
                       + [sum(c["permits"].get(y, 0) for c in cs) for y in YEARS]
                       + [sum(c.get("mix", [0, 0, 0, 0])[k] for c in cs) for k in range(4)]
                       + [sum(c.get("stock", {}).get(k, 0) for c in cs) for k in stock_keys])
    print(f"wrote {OUT_STATE.relative_to(ROOT)} ({len(ABBR)} rows: 50 states + DC, each the sum of its counties)")

    # Checksums of everything this script read (deduplicated, sorted).
    seen = sorted(set(checksums), key=lambda x: x[0])
    (RAW / "CHECKSUMS.txt").write_text(
        f"accessed {date.today().isoformat()}\n" + "".join(f"{h}  {n}\n" for n, h in seen)
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
