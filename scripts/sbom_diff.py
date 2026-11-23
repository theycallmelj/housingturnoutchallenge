#!/usr/bin/env python3
"""Fail if two CycloneDX SBOMs list different packages.

    python3 scripts/sbom_diff.py committed.cdx.json regenerated.cdx.json

Compares the set of package URLs (name + version), ignoring timestamps,
serial numbers and tool metadata, which change on every generation. CI uses
this to check that a committed SBOM still matches the lockfile
(AGENTS.md section 1: regenerate the SBOM whenever dependencies change).
Standard library only.
"""

import json
import sys


def packages(path: str) -> set:
    with open(path) as f:
        bom = json.load(f)
    out = set()
    for c in bom.get("components", []):
        purl = c.get("purl") or f"{c.get('group', '')}/{c.get('name')}@{c.get('version')}"
        out.add(purl.split("?")[0])
    return out


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    committed, fresh = packages(sys.argv[1]), packages(sys.argv[2])
    missing, extra = sorted(fresh - committed), sorted(committed - fresh)
    if not missing and not extra:
        print(f"SBOM current: {len(committed)} packages")
        return 0
    print("SBOM is out of date; regenerate it (see AGENTS.md section 1).")
    for p in missing:
        print(f"  not in committed SBOM: {p}")
    for p in extra:
        print(f"  no longer a dependency: {p}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
