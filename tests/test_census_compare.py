import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[1]
spec = importlib.util.spec_from_file_location("census_compare", ROOT / "tools" / "census_compare.py")
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)


def payload(rows, sha="sha"):
    return {"sha": sha, "corpora": {"corpus": {"rows": rows}}}


class CensusCompareTests(unittest.TestCase):
    def test_diff_names_moved_jar_and_call_deltas(self):
        base = payload([{"jar": "a.jar", "mode": "closure", "verdict": "ran-hollow", "calls": [["x#f()", 3]]}])
        merged = payload([{"jar": "a.jar", "mode": "closure", "verdict": "ran-clean", "calls": []}])
        report = tool.compare_payloads(base, merged, {"base": [base], "merged": [merged]}, "server")
        self.assertEqual(report["moved"][0]["jar"], "a.jar")
        self.assertEqual(report["moved"][0]["vanished"], [{"call": "x#f()", "count": 3}])
        self.assertEqual(report["moved"][0]["appeared"], [])

    def test_repetition_difference_is_flaky_and_excluded(self):
        stable = {"jar": "stable.jar", "verdict": "ran-clean", "calls": []}
        first = payload([{"jar": "flaky.jar", "verdict": "ran-clean", "calls": []}, stable])
        second = payload([{"jar": "flaky.jar", "verdict": "ran-hollow", "calls": [["x", 1]]}, stable])
        merged = payload([{"jar": "flaky.jar", "verdict": "ran-clean", "calls": []}, stable])
        report = tool.compare_payloads(first, merged, {"base": [first, second], "merged": [merged, merged]}, "server")
        self.assertEqual(report["moved"], [])
        self.assertEqual(report["flaky"][0]["jar"], "flaky.jar")

    def test_markdown_has_totals_and_attribution_column(self):
        base = payload([{"jar": "a.jar", "verdict": "ran-hollow", "unknown": 2, "calls": [["x", 2]]}])
        merged = payload([{"jar": "a.jar", "verdict": "ran-clean", "unknown": 0, "calls": []}])
        report = tool.compare_payloads(base, merged, {"base": [base], "merged": [merged]}, "server")
        text = tool.markdown(report)
        self.assertIn("Unknown totals", text)
        self.assertIn("a.jar", text)
        self.assertIn("EXACT/SILENT attribution", text)
        self.assertIn("| corpus | default | server | 2 | 0 |", text)


if __name__ == "__main__":
    unittest.main()
