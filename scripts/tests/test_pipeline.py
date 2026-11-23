"""Unit tests for the data pipeline's core logic (standard library only).

    python3 -m unittest discover -s scripts/tests
"""

import struct
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import census_xl  # noqa: E402
import fetch_construction_timing as t  # noqa: E402


class RkNumbers(unittest.TestCase):
    def test_integer_and_scaled(self):
        self.assertEqual(census_xl._rk((25 << 2) | 2), 25.0)          # integer
        self.assertEqual(census_xl._rk((1234 << 2) | 3), 12.34)       # integer / 100
        raw = struct.unpack("<Q", struct.pack("<d", 1.5))[0] >> 32
        self.assertEqual(census_xl._rk(raw & 0xFFFFFFFC), 1.5)       # float, high 30 bits

    def test_negative_integer(self):
        v = ((-7) & 0x3FFFFFFF) << 2 | 2
        self.assertEqual(census_xl._rk(v), -7.0)

    def test_rows_fill_gaps(self):
        self.assertEqual(census_xl.rows({(0, 1): "a", (1, 0): 2.0}), [["", "a"], [2.0, ""]])


class Curves(unittest.TestCase):
    def test_all_started_in_month_zero(self):
        # Permits spread evenly over 12 months, all start immediately: year 1.
        years = t.annual({0: 1.0})
        self.assertAlmostEqual(years[0], 1.0)
        self.assertAlmostEqual(sum(years), 1.0)

    def test_six_month_lag_splits_years(self):
        years = t.annual({6: 1.0})
        self.assertAlmostEqual(years[0], 0.5)
        self.assertAlmostEqual(years[1], 0.5)

    def test_convolution_adds_build_time(self):
        # Start at 0 months, finish 12 months later: all in year 2.
        years = t.annual({0: 1.0}, {12: 1.0})
        self.assertAlmostEqual(years[1], 1.0)
        # Never-started share is preserved (start probabilities sum to 0.8).
        years = t.annual({0: 0.8}, {3: 1.0})
        self.assertAlmostEqual(sum(years), 0.8)

    def test_late_events_go_to_after(self):
        years = t.annual({0: 1.0}, {100: 1.0})
        self.assertAlmostEqual(years[-1], 1.0)


class BuildTime(unittest.TestCase):
    def test_tail_matches_published_mean(self):
        buckets = [10, 40, 25, 15, 10]
        mean = 9.0
        pmf = t.build_pmf(buckets, mean)
        self.assertAlmostEqual(sum(pmf.values()), 1.0, places=9)
        got = sum(m * p for m, p in pmf.items())
        # The tail is cut at 120 months, which trims the mean slightly.
        self.assertLess(abs(got - mean), 0.1)
        # Bucket shares are kept.
        self.assertAlmostEqual(sum(p for m, p in pmf.items() if m >= 13), 0.10, places=9)

    def test_tail_mean_is_bounded(self):
        pmf = t.build_pmf([0, 0, 0, 0, 100], 1000.0)
        got = sum(m * p for m, p in pmf.items())
        self.assertLess(got, 61.0)


class PdfTables(unittest.TestCase):
    SAMPLE = (
        "Percent Distribution|United States|3 months or less15 16 12 7 1 4 1 1 (Z) |4 to 6 months52 58 38 24 10 21 13 6 3 |"
        "7 to 9 months19 17 27 25 21 26 27 22 13 |10 to 12 months7 5 12 16 27 23 27 36 25 |13 months or more7 4 12 28 41 26 33 36 60 |"
        "Northeast|3 months or less6 6 9 4 (Z) (Z) (Z) (Z) (Z) |4 to 6 months37 41 31 20 11 18 1 (Z) (Z) |7 to 9 months24 25 24 23 17 24 6 4 13 |"
        "10 to 12 months12 11 17 18 20 22 15 37 8 |13 months or more20 17 20 35 52 37 77 59 78 |Relative Standard Error|United States|3 months or less5 6 14 19 62 73 64 58 60 |"
    )

    def test_line_layout(self):
        d = t.parse_build_pdf(self.SAMPLE)
        self.assertEqual([r[0] for r in d["united_states"]], [15, 52, 19, 7, 7])
        self.assertEqual(d["united_states"][0][8], 0.25)  # (Z) = less than 0.5
        self.assertEqual(d["northeast"][4][8], 78)

    def test_word_layout(self):
        words = self.SAMPLE.replace(" ", "|")
        d = t.parse_build_pdf(words)
        self.assertEqual([r[8] for r in d["united_states"]], [0.25, 3, 13, 25, 60])


class StartsSheet(unittest.TestCase):
    def test_current_month_rows_and_pooled(self):
        cells = {
            (1, 0): 45566.0,  # 2024-10-01
            (8, 0): "Authorization", (8, 2): "Northeast", (8, 4): "Midwest", (8, 6): "South", (8, 8): "West",
            (10, 0): "Oct-24", (10, 2): 0.5, (10, 3): 0.2,
            (11, 0): "Sep-24", (11, 2): 0.3, (11, 3): 0.1,
            (12, 0): "Oct-23", (12, 1): 1.0, (12, 2): 0.01, (12, 3): 0.02,
            (13, 0): "Apr-23", (13, 1): 2.0, (13, 2): 0.001, (13, 3): 0.003,
        }
        cur, by, pooled = t.parse_starts_sheet(cells)
        self.assertEqual(cur, (2024, 10))
        self.assertEqual(by[("northeast", "1 unit")][(2024, 10)], 0.5)
        self.assertEqual(by[("northeast", "2+ units")][(2024, 9)], 0.1)
        self.assertEqual(pooled[("northeast", "1 unit")], [0.01, 0.001])

    def test_start_lags_assemble_and_fill(self):
        best = {}
        a = (2020, 1)
        for k in range(-2, 66):
            cur = t.month_add(*a, k)
            by = {("south", "1 unit"): {a: 0.5 if k == 0 else 0.0}}
            best[cur] = (3, by, {("south", "1 unit"): [0.01, 0.001]})
        lags = t.start_lags(best, "south", "1 unit", a)
        self.assertEqual(lags[0], 0.5)
        self.assertAlmostEqual(sum(lags[k] for k in range(12, 18)), 0.06)
        self.assertAlmostEqual(sum(lags[k] for k in range(18, 66)), 0.048)
        # A missing individual month makes the cohort unusable.
        del best[t.month_add(*a, 3)]
        self.assertIsNone(t.start_lags(best, "south", "1 unit", a))


if __name__ == "__main__":
    unittest.main()
