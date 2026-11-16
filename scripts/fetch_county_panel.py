#!/usr/bin/env python3
"""Build the county-year panel the baseline permit model is fitted on.

Outputs (committed; small):

* data/derived/county_panel.csv    one row per county and year, 2010-2025:
  permitted units by building type (1 unit, 2-4, 5+), population, net
  migration, house-price change
* data/derived/national_series.csv one row per year: 10-year Treasury yield
  (annual average of daily par yields)

Sources (all U.S. government works, public domain; provenance in
research/data/):

* Building Permits Survey, annual county files co{YEAR}a.txt, 2010-2025
  -> census-building-permits-survey.md
* Population Estimates, county totals, vintages 2020 (2010-2019) and 2025
  (2020-2025): POPESTIMATE and NETMIG -> census-population-estimates.md
* FHFA House Price Index, annual county all-transactions index
  (hpi_at_county.xlsx), annual change -> fhfa-hpi.md
* U.S. Treasury daily par yield curve rates, 10-year -> treasury-yield-curve.md

Raw downloads go to research/data/raw/ (gitignored), checksums to
research/data/raw/CHECKSUMS-panel.txt. Standard library only.

    python3 scripts/fetch_county_panel.py
"""

import csv
import hashlib
import io
import sys
import time
import urllib.request
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import census_xl  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "research" / "data" / "raw"
OUT = ROOT / "data" / "derived" / "county_panel.csv"
OUT_NAT = ROOT / "data" / "derived" / "national_series.csv"

YEARS = list(range(2010, 2026))
BPS_COUNTY = "https://www2.census.gov/econ/bps/County/co{year}a.txt"
PEP_2020 = "https://www2.census.gov/programs-surveys/popest/datasets/2010-2020/counties/totals/co-est2020-alldata.csv"
PEP_2025 = "https://www2.census.gov/programs-surveys/popest/datasets/2020-2025/counties/totals/co-est2025-alldata.csv"
FHFA_COUNTY = "https://www.fhfa.gov/hpi/download/annual/hpi_at_county.xlsx"
TREASURY = ("https://home.treasury.gov/resource-center/data-chart-center/interest-rates/daily-treasury-rates.csv/"
            "{year}/all?type=daily_treasury_yield_curve&field_tdr_date_value={year}&page&_format=csv")

STATES = {
    "01", "02", "04", "05", "06", "08", "09", "10", "11", "12", "13", "15", "16", "17", "18", "19", "20",
    "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37",
    "38", "39", "40", "41", "42", "44", "45", "46", "47", "48", "49", "50", "51", "53", "54", "55", "56",
}

checksums = []


def fetch(url: str, name: str) -> bytes:
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / name
    if not path.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 housing-challenge-panel/1.0"})
        for attempt in range(5):
            try:
                time.sleep(0.5)
                with urllib.request.urlopen(req, timeout=180) as resp:
                    path.write_bytes(resp.read())
                break
            except urllib.error.HTTPError as e:
                if e.code != 429 or attempt == 4:
                    raise
                time.sleep(15 * (attempt + 1))
    data = path.read_bytes()
    checksums.append((name, hashlib.sha256(data).hexdigest()))
    return data


def main() -> int:
    panel = {}  # (state, county, year) -> dict

    def row(st, co, y):
        return panel.setdefault((st, co, y), {})

    # Permits by building type (imputed totals; see fetch_state_baseline.py).
    for y in YEARS:
        text = fetch(BPS_COUNTY.format(year=y), f"bps_co{y}a.txt").decode("latin-1")
        for r in csv.reader(io.StringIO(text)):
            if len(r) < 18 or not r[0].strip().isdigit():
                continue
            st, co = r[1].strip(), r[2].strip()
            if st not in STATES:
                continue
            u = [int(r[7 + 3 * k]) for k in range(4)]
            d = row(st, co, y)
            d["p1"], d["p24"], d["p5"] = u[0], u[1] + u[2], u[3]

    # Population and net migration (July 1 to June 30). Vintage 2020 gives
    # population 2010-2019 and migration 2011-2020; vintage 2025 gives
    # population 2020-2025 and migration 2021-2025 and carries Connecticut's
    # planning regions. NETMIG2010 (vintage 2020) and NETMIG2020 (vintage
    # 2025) cover only April-July, so they are not used.
    for url, name, pop_years, mig_years in ((PEP_2020, "co-est2020-alldata.csv", range(2010, 2020), range(2011, 2021)),
                                            (PEP_2025, "co-est2025-alldata.csv", range(2020, 2026), range(2021, 2026))):
        for r in csv.DictReader(io.StringIO(fetch(url, name).decode("latin-1"))):
            if r["SUMLEV"] != "050" or r["STATE"] not in STATES:
                continue
            for y in pop_years:
                if r.get(f"POPESTIMATE{y}"):
                    row(r["STATE"], r["COUNTY"], y)["pop"] = int(r[f"POPESTIMATE{y}"])
            for y in mig_years:
                if r.get(f"NETMIG{y}") not in (None, ""):
                    row(r["STATE"], r["COUNTY"], y)["mig"] = int(r[f"NETMIG{y}"])

    # FHFA annual county HPI: annual change (%).
    sh = census_xl.sheets(fetch(FHFA_COUNTY, "fhfa_hpi_at_county.xlsx"))
    cells = next(iter(sh.values()))
    hpi_rows = 0
    for r in census_xl.rows(cells):
        if len(r) < 5:
            continue
        # FIPS codes are stored as text in some rows and as numbers in others.
        fips = f"{int(r[2]):05d}" if isinstance(r[2], float) else str(r[2]).strip()
        if len(fips) != 5 or not fips.isdigit():
            continue
        try:
            y = int(float(r[3]))
        except (TypeError, ValueError):
            continue
        if y not in YEARS or not isinstance(r[4], float):
            continue
        st, co = fips[:2], fips[2:]
        if st in STATES and (st, co, y) in panel:
            panel[(st, co, y)]["hpi"] = r[4]
            hpi_rows += 1

    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["state_fips", "county_fips", "year", "permits_1unit", "permits_2to4", "permits_5plus",
                    "population", "net_migration", "hpi_change_pct"])
        for (st, co, y) in sorted(panel):
            d = panel[(st, co, y)]
            if "pop" not in d and "p1" not in d:
                continue
            w.writerow([st, co, y, d.get("p1", ""), d.get("p24", ""), d.get("p5", ""),
                        d.get("pop", ""), d.get("mig", ""), "" if "hpi" not in d else f"{d['hpi']:.2f}"])
    print(f"wrote {OUT.relative_to(ROOT)} ({len(panel)} county-years; {hpi_rows} with FHFA price change)")

    # 10-year Treasury: annual average of daily par yields.
    lines = ["year,treasury_10y_pct,days"]
    for y in YEARS:
        rows = list(csv.DictReader(io.StringIO(fetch(TREASURY.format(year=y), f"treasury_par_yield_{y}.csv").decode("utf-8-sig"))))
        vals = [float(r["10 Yr"]) for r in rows if r.get("10 Yr") not in (None, "", "N/A")]
        lines.append(f"{y},{sum(vals) / len(vals):.3f},{len(vals)}")
    OUT_NAT.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT_NAT.relative_to(ROOT)}")
    print("\n".join(lines))

    seen = sorted(set(checksums))
    (RAW / "CHECKSUMS-panel.txt").write_text(
        f"accessed {date.today().isoformat()}\n" + "".join(f"{h}  {n}\n" for n, h in seen)
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
