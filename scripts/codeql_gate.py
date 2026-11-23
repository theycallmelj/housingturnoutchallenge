#!/usr/bin/env python3
"""Fail on high or critical CodeQL findings in SARIF files.

    python3 scripts/codeql_gate.py results-dir/

The Official Rules (7d) require no unaddressed critical or high CodeQL
findings. GitHub enforces that through code scanning, which private repos
only get with the paid Code Security add-on, so CI also checks the SARIF
itself: a finding fails the gate if its rule's security-severity is 7.0 or
more (GitHub's high/critical bands) or its level is "error". Writes a short
summary for the job summary. Standard library only.
"""

import json
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    files = sorted(Path(sys.argv[1]).glob("*.sarif"))
    if not files:
        print(f"no SARIF files in {sys.argv[1]}", file=sys.stderr)
        return 2
    total, blocking = 0, []
    for f in files:
        sarif = json.loads(f.read_text())
        for run in sarif.get("runs", []):
            rules = {}
            for rule in run.get("tool", {}).get("driver", {}).get("rules", []):
                rules[rule.get("id")] = rule
            for r in run.get("results", []):
                total += 1
                rule = rules.get(r.get("ruleId"), {})
                sev = float(rule.get("properties", {}).get("security-severity", 0) or 0)
                level = r.get("level") or rule.get("defaultConfiguration", {}).get("level", "warning")
                if sev >= 7.0 or level == "error":
                    loc = r.get("locations", [{}])[0].get("physicalLocation", {})
                    where = f"{loc.get('artifactLocation', {}).get('uri', '?')}:{loc.get('region', {}).get('startLine', '?')}"
                    blocking.append(f"{r.get('ruleId')} (severity {sev or level}) at {where}: {r.get('message', {}).get('text', '')[:160]}")
    print(f"CodeQL: {total} finding(s) in {len(files)} SARIF file(s); {len(blocking)} high/critical or error.")
    for b in blocking:
        print(f"  - {b}")
    return 1 if blocking else 0


if __name__ == "__main__":
    sys.exit(main())
