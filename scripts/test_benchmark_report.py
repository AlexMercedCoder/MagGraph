import unittest

from scripts.benchmark_report import build_report, parse_rows


def baseline():
    return {
        "id": "test",
        "max_regression_ratio": 2.0,
        "tiers": {
            "1000": {
                "open_ms": 10,
                "search_ms": 2,
                "hybrid_ms": 3,
                "backlinks_ms": 1,
                "recall_us": 100,
                "update_us": 50,
            }
        },
    }


class BenchmarkReportTests(unittest.TestCase):
    def test_parse_and_compare_passing_report(self):
        rows = parse_rows("noise\n1000,12,2,4,1,110,55\n")
        report = build_report(rows, baseline())
        self.assertTrue(report["ok"])
        self.assertEqual(report["comparisons"][0]["metrics"]["open_ms"]["ratio"], 1.2)

    def test_missing_or_regressed_tier_fails_report(self):
        self.assertFalse(build_report({}, baseline())["ok"])
        rows = parse_rows("1000,30,2,4,1,110,55\n")
        self.assertFalse(build_report(rows, baseline())["ok"])


if __name__ == "__main__":
    unittest.main()
