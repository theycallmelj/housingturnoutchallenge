#!/usr/bin/env python3
"""Build data/derived/construction_curves.csv: how a year's permits turn into
construction starts and completed homes, year by year, from the Census
Survey of Construction (SOC).

For each Census region and building type (1 unit, 2-4 units, 5+ units):

1. **Permit -> start (realization and timing).** The monthly "Starts Rates"
   files give, for each permit-authorization month, the share of its units
   started in a given month. Following one authorization month across later
   files gives its monthly start-lag distribution (lags -2 to 11 months
   individually; 12-17 and 18-65 months as pooled rows, spread evenly). The
   sum over all lags is the realization rate: the share of permitted units
   that ever start. These come by region for 1-unit and 2+-unit buildings.
2. **Start -> completion timing.** The annual "Percent Distribution of
   Buildings Completed by Number of Months from Start" tables give five
   buckets (3 months or less ... 13 months or more) by region and building
   size. Within a bucket months are spread evenly; the open 13+ bucket gets a
   geometric tail whose mean makes the overall mean equal Census's published
   average months from start to completion for that region, year and size.
3. **Convolution.** Permit -> completion months = start lag + build months.
   Permits are spread evenly over their issue year, then each start and
   completion is placed in year 1, 2, ... after issue.
4. **Averaging.** One curve per permit cohort year 2015-2024 (each using the
   build-time table for the following year, when most of the cohort
   finishes), averaged with equal weight. The `excl_covid` variant drops the
   2020 and 2021 cohorts.

A back-test applies the curves to actual regional permits and compares the
implied completions with Census's published completions (printed, and
written to data/derived/construction_curves_backtest.csv).

Sources (U.S. government works, public domain; provenance file
research/data/census-survey-of-construction.md):

* https://www.census.gov/construction/nrc/starts_rates.html  (startsrates_YYYYMM.xls[x])
* https://www.census.gov/construction/nrc/data/time/historic.html  (pct_start_to_comp_YYYY.pdf)
* https://www.census.gov/construction/nrc/xls/avg_starttocomp_cust.xlsx
* https://www.census.gov/construction/nrc/xls/permits_cust.xlsx, comps_cust.xlsx  (back-test)

Raw downloads go to research/data/raw/nrc/ (gitignored), checksums to
research/data/raw/CHECKSUMS-timing.txt. Standard library only.

    python3 scripts/fetch_construction_timing.py
"""

import hashlib
import re
import sys
import time
import urllib.error
import urllib.request
import zlib
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import census_xl  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "research" / "data" / "raw" / "nrc"
OUT = ROOT / "data" / "derived" / "construction_curves.csv"
OUT_BACKTEST = ROOT / "data" / "derived" / "construction_curves_backtest.csv"
NRC = "https://www.census.gov/construction/nrc/"

REGIONS = ["northeast", "midwest", "south", "west"]
REGION_SHEET = {"united_states": "United States", "northeast": "Northeast", "midwest": "Midwest", "south": "South", "west": "West"}
COHORTS = list(range(2015, 2025))
COVID = {2020, 2021}
YEARS_OUT = 6  # year offsets reported (1..6); later completions go to "after"
# Building types: (starts-rate column, completion-table column)
# Completion-table columns: 0 Total 1-unit, 1-3 by builder, 4 Total 2+, 5 2-4,
# 6 5-9, 7 10-19, 8 20+. 5+ uses the 20+ column (see provenance file).
TYPES = {"1unit": ("1 unit", 0), "2to4": ("2+ units", 5), "5plus": ("2+ units", 8)}
MAX_MONTHS = 120
# Months the Census starts-rates page does not list (checked 2026-11-16).
# The next month's file supplies their revised estimates.
NOT_PUBLISHED = {"202509", "202511", "202602"}
LAST_FILE = (2026, 9)

checksums = []


def fetch(url: str, name: str, optional=False):
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / name
    if not path.exists():
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 housing-challenge-timing/2.0"})
        for attempt in range(6):
            try:
                time.sleep(1.0)  # census.gov rate-limits bursts
                with urllib.request.urlopen(req, timeout=120) as resp:
                    path.write_bytes(resp.read())
                break
            except urllib.error.HTTPError as e:
                if optional and e.code == 404:
                    return None
                if e.code != 429 or attempt == 5:
                    raise
                time.sleep(15 * (attempt + 1))
    data = path.read_bytes()
    checksums.append((name, hashlib.sha256(data).hexdigest()))
    return data


def month_add(y: int, m: int, k: int):
    i = y * 12 + (m - 1) + k
    return i // 12, i % 12 + 1


def serial_to_month(v: float):
    d = date(1899, 12, 30) + timedelta(days=int(v))
    return d.year, d.month


MON = {m: i + 1 for i, m in enumerate(["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"])}


def label_month(v):
    """'Jun-15' or an Excel date serial -> (year, month)."""
    if isinstance(v, float):
        return serial_to_month(v)
    m = re.match(r"([A-Z][a-z]{2})-(\d{2})$", str(v).strip())
    return (2000 + int(m.group(2)), MON[m.group(1)]) if m else None


# ------------------------------------------------------------ permit -> start

def parse_starts_sheet(cells):
    """One starts-rate sheet -> (current month, {(region, type): {auth month: ratio}},
    {(region, type): (pooled 12-17 ratio, pooled 18-65 ratio)})."""
    rows = census_xl.rows(cells)
    current = None
    for r in rows[:4]:
        for v in r:
            if isinstance(v, float) and v > 30000:
                current = serial_to_month(v)
                break
        if current:
            break
    # Region block: header row with 'Northeast' ... 'West', then '1 unit'/'2+ units'.
    head = next(i for i, r in enumerate(rows) if "Northeast" in r and "West" in r)
    cols = {}
    for c, v in enumerate(rows[head]):
        if v in ("Northeast", "Midwest", "South", "West"):
            cols[(v.lower(), "1 unit")] = c
            cols[(v.lower(), "2+ units")] = c + 1
    lab = rows[head].index("Authorization") if "Authorization" in rows[head] else 0
    by, pooled = {k: {} for k in cols}, {k: [0.0, 0.0] for k in cols}
    for r in rows[head + 2:]:
        if lab >= len(r) or r[lab] in ("", None):
            break
        am = label_month(r[lab])
        if am is None:
            break
        flag = r[lab + 1] if lab + 1 < len(r) else ""
        for k, c in cols.items():
            v = r[c] if c < len(r) else ""
            v = float(v) if isinstance(v, float) else 0.0
            if flag in (1.0, "1"):
                pooled[k][0] = v
            elif flag in (2.0, "2"):
                pooled[k][1] = v
            else:
                by[k][am] = v
    return current, by, pooled


def starts_data():
    """{current month: parsed sheet} using the best revision available for each month."""
    best = {}  # current month -> (rank, by, pooled)
    y, m = 2015, 1
    while (y, m) <= LAST_FILE:
        tag = f"{y}{m:02d}"
        ext = "xls" if y <= 2023 else "xlsx"
        data = None if tag in NOT_PUBLISHED else fetch(f"{NRC}xls/startsrates_{tag}.{ext}", f"startsrates_{tag}.{ext}")
        if data:
            sh = census_xl.sheets(data)
            for name, cells in sh.items():
                rank = 3 if "Second" in name else 2 if "First" in name else 1 if "Preliminary" in name else 0
                if not rank:
                    continue
                cur, by, pooled = parse_starts_sheet(cells)
                if cur and best.get(cur, (0,))[0] < rank:
                    best[cur] = (rank, by, pooled)
        y, m = month_add(y, m, 1)
    return best


def start_lags(best, region, stype, auth):
    """Monthly start-lag distribution {lag: share} for one authorization month,
    or None if any individual lag is missing."""
    key = (region, stype)
    lags = {}
    for k in range(-2, 12):
        cur = month_add(*auth, k)
        if cur not in best:
            return None
        lags[k] = best[cur][1][key].get(auth, 0.0)
    pooled18 = []
    for k in range(12, 66):
        cur = month_add(*auth, k)
        if cur in best:
            p = best[cur][2][key]
            if k < 18:
                lags[k] = p[0]
            else:
                lags[k] = p[1]
                pooled18.append(p[1])
        elif k < 18:
            return None
    # Lags 18-65 not yet observed: the mean of the observed pooled rows.
    fill = sum(pooled18) / len(pooled18) if pooled18 else 0.0
    for k in range(18, 66):
        lags.setdefault(k, fill)
    return lags


# ------------------------------------------------------------ start -> completion

def pdf_text(data: bytes) -> str:
    out = []
    for m in re.finditer(rb"stream\r?\n(.*?)endstream", data, re.S):
        try:
            s = zlib.decompress(m.group(1))
        except zlib.error:
            continue
        for t in re.findall(rb"\[(.*?)\]\s*TJ|\((.*?)\)\s*Tj", s, re.S):
            seg = b"".join(re.findall(rb"\((.*?)(?<!\\)\)", t[0])) if t[0] else t[1]
            out.append(seg.decode("latin-1").replace("\\(", "(").replace("\\)", ")"))
    return "|".join(out)


BUCKETS = ["3 months or less", "4 to 6 months", "7 to 9 months", "10 to 12 months", "13 months or more"]


def parse_build_pdf(text: str):
    """{region: [5 buckets x 9 columns]} from the percent-distribution page.
    Works on PDFs that store whole lines and on ones that store single words:
    tokens are rejoined with spaces and the first table after each region
    name is read (the relative-standard-error table comes later)."""
    flat = re.sub(r"\s+", " ", text.replace("|", " "))
    flat = re.sub(r"(less|months|more)(\d|\()", r"\1 \2", flat)
    out = {}
    for reg, name in REGION_SHEET.items():
        i = flat.find(" " + name + " 3 months")
        if i < 0:
            continue
        seg = flat[i:]
        table = []
        for b in BUCKETS:
            j = seg.find(b)
            vals = seg[j + len(b):].split()[:9]
            table.append([0.25 if v == "(Z)" else None if v.startswith("(") else float(v) for v in vals])
            seg = seg[j + len(b):]
        out[reg] = table
    return out


def pdf_year(text: str):
    m = re.findall(r"\b(20\d\d)\b", re.sub(r"\s+", " ", text.replace("|", " ")))
    return int(m[0]) if m else None


def averages():
    """{region: {year: [9 average months]}} from avg_starttocomp_cust.xlsx."""
    sh = census_xl.sheets(fetch(NRC + "xls/avg_starttocomp_cust.xlsx", "avg_starttocomp_cust.xlsx"))
    out = {}
    for reg, name in REGION_SHEET.items():
        cells = next(c for n, c in sh.items() if n.strip() == name)
        d = {}
        for r in census_xl.rows(cells):
            if r and isinstance(r[0], (float, str)) and str(r[0])[:4].isdigit():
                vals = [x if isinstance(x, float) else None for x in r[1:10]]
                d[int(float(r[0]) if isinstance(r[0], float) else r[0][:4])] = vals
        out[reg] = d
    return out


def build_pmf(buckets, mean):
    """Monthly start->completion distribution {months: share} from five bucket
    percentages and the published average months."""
    p = [b or 0.0 for b in buckets]
    tot = sum(p)
    p = [x / tot for x in p]
    pmf = {}
    for share, months in zip(p[:4], [(1, 2, 3), (4, 5, 6), (7, 8, 9), (10, 11, 12)]):
        for mo in months:
            pmf[mo] = share / 3
    head_mean = sum(s * mid for s, mid in zip(p[:4], (2, 5, 8, 11)))
    tail_mean = (mean - head_mean) / p[4] if p[4] > 0 and mean else 18.0
    tail_mean = min(max(tail_mean, 13.5), 60.0)
    q = (tail_mean - 13) / (tail_mean - 12)
    tail = [(1 - q) * q**j for j in range(MAX_MONTHS - 12)]
    t = sum(tail)
    for j, v in enumerate(tail):
        pmf[13 + j] = p[4] * v / t
    return pmf


# ------------------------------------------------------------ curves

def annual(lags: dict, build: dict = None):
    """Shares of a permit-year cohort started (build=None) or completed, by
    year after issue (1..YEARS_OUT, then 'after'). Permits spread evenly over
    the 12 issue months."""
    event = {}
    if build is None:
        event = dict(lags)
    else:
        for k, pk in lags.items():
            for d, pd in build.items():
                event[k + d] = event.get(k + d, 0.0) + pk * pd
    years = [0.0] * (YEARS_OUT + 1)
    for a in range(12):
        for t, p in event.items():
            j = max(0, (a + t) // 12)
            years[min(j, YEARS_OUT)] += p / 12
    return years


def mean_curves(curves):
    n = len(curves)
    return [sum(c[i] for c in curves) / n for i in range(len(curves[0]))]


def main() -> int:
    best = starts_data()
    print(f"starts-rate months parsed: {len(best)} ({min(best)} to {max(best)})")
    avg = averages()
    build_tables = {}
    for y in range(2015, 2026):
        txt = pdf_text(fetch(f"{NRC}pdf/pct_start_to_comp_{y}.pdf", f"pct_start_to_comp_{y}.pdf"))
        build_tables[y] = parse_build_pdf(txt)

    rows = []
    curves_by = {}
    for reg in REGIONS:
        for btype, (scol, bcol) in TYPES.items():
            per_cohort = {}
            for cy in COHORTS:
                lag_list = [start_lags(best, reg, scol, (cy, m)) for m in range(1, 13)]
                lag_list = [x for x in lag_list if x]
                if len(lag_list) < 6:
                    continue
                lags = {k: sum(x[k] for x in lag_list) / len(lag_list) for k in lag_list[0]}
                raw_rate = sum(lags.values())
                if raw_rate > 1.0:
                    # SOC starts and BPS permits are estimated separately, so the
                    # 1-unit ratio can exceed 1; no more than every permit can start.
                    lags = {k: v / raw_rate for k, v in lags.items()}
                by = min(cy + 1, 2025)
                table = build_tables[by].get(reg) or build_tables[by]["united_states"]
                buckets = [row[bcol] for row in table]
                if any(b is None for b in buckets):
                    buckets = [row[bcol] for row in build_tables[by]["united_states"]]
                mean = (avg[reg].get(by) or [None] * 9)[bcol] or (avg["united_states"].get(by) or [None] * 9)[bcol]
                pmf = build_pmf(buckets, mean)
                per_cohort[cy] = (sum(lags.values()), annual(lags), annual(lags, pmf), sum(k * v for k, v in lags.items()) / sum(lags.values()), sum(d * v for d, v in pmf.items()), raw_rate)
            for variant, keep in (("all", COHORTS), ("excl_covid", [c for c in COHORTS if c not in COVID])):
                cs = [per_cohort[c] for c in keep if c in per_cohort]
                rate = sum(c[0] for c in cs) / len(cs)
                raw = sum(c[5] for c in cs) / len(cs)
                started = mean_curves([c[1] for c in cs])
                done = mean_curves([c[2] for c in cs])
                m_start = sum(c[3] for c in cs) / len(cs)
                m_build = sum(c[4] for c in cs) / len(cs)
                curves_by[(reg, btype, variant)] = done
                rows.append([reg, btype, variant, f"{min(keep)}-{max(keep)}" + ("" if variant == "all" else " excl 2020-2021"), len(cs),
                             f"{rate:.4f}", f"{raw:.4f}", f"{m_start:.2f}", f"{m_build:.2f}"]
                            + [f"{v:.4f}" for v in started] + [f"{v:.4f}" for v in done] + [f"{1 - rate:.4f}"])
    head = (["region", "building_type", "variant", "cohorts", "n_cohorts", "start_rate", "start_rate_uncapped", "mean_months_permit_to_start", "mean_months_start_to_completion"]
            + [f"started_y{j}" for j in range(1, YEARS_OUT + 1)] + ["started_after"]
            + [f"completed_y{j}" for j in range(1, YEARS_OUT + 1)] + ["completed_after", "never_started"])
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(",".join(head) + "\n" + "".join(",".join(map(str, r)) + "\n" for r in rows))
    print(f"wrote {OUT.relative_to(ROOT)} ({len(rows)} rows)")
    for r in rows:
        if r[2] == "all":
            print(f"  {r[0]:<9} {r[1]:<5} start rate {r[5]} (uncapped {r[6]}); completed y1-y6 {', '.join(r[16:22])}; never started {r[-1]}")

    # Back-test: implied vs. published completions, by region, 2019-2025.
    permits = census_xl.sheets(fetch(NRC + "xls/permits_cust.xlsx", "permits_cust.xlsx"))["Annual"]
    comps = census_xl.sheets(fetch(NRC + "xls/comps_cust.xlsx", "comps_cust.xlsx"))["Annual"]
    # Columns: Year, US total, US 1, US 2-4, US 5+, NE total, NE 1, MW total, MW 1, S total, S 1, W total, W 1
    col = {"northeast": (5, 6), "midwest": (7, 8), "south": (9, 10), "west": (11, 12)}

    def annual_table(cells):
        out = {}
        for r in census_xl.rows(cells):
            if r and isinstance(r[0], (float, str)) and str(r[0])[:4].isdigit() and len(r) > 12:
                try:
                    out[int(float(r[0]) if isinstance(r[0], float) else r[0][:4])] = [float(x) if isinstance(x, float) else float(str(x)) for x in r[1:13]]
                except ValueError:
                    pass
        return out

    P, C = annual_table(permits), annual_table(comps)
    lines = ["region,year,implied_completions_thousands,published_completions_thousands,ratio"]
    print("back-test (implied / published completions, thousands of units):")
    for reg, (ct, c1) in col.items():
        one = curves_by[(reg, "1unit", "all")]
        multi = curves_by[(reg, "5plus", "all")]
        for y in range(2019, 2026):
            implied = 0.0
            for j in range(YEARS_OUT):
                py = y - j
                if py in P:
                    tot, u1 = P[py][ct - 1], P[py][c1 - 1]
                    implied += u1 * one[j] + (tot - u1) * multi[j]
            pub = C[y][ct - 1]
            lines.append(f"{reg},{y},{implied:.1f},{pub:.1f},{implied / pub:.3f}")
        rs = [float(x.split(",")[-1]) for x in lines if x.startswith(reg + ",")]
        print(f"  {reg:<9} 2019-2025 ratio mean {sum(rs) / len(rs):.3f} (range {min(rs):.3f}-{max(rs):.3f})")
    OUT_BACKTEST.write_text("\n".join(lines) + "\n")
    print(f"wrote {OUT_BACKTEST.relative_to(ROOT)}")

    (RAW.parent / "CHECKSUMS-timing.txt").write_text(
        f"accessed {date.today().isoformat()}\n" + "".join(f"{h}  nrc/{n}\n" for n, h in sorted(set(checksums)))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
