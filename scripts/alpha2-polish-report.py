#!/usr/bin/env python3
"""Summarize reviewed Alpha 2 host receipts without exposing raw observations."""

import argparse
import json
import re
from pathlib import Path

MANIFEST = Path(__file__).with_name("alpha2-polish-matrix.json")
STATUSES = {"verified", "expected refusal", "unavailable", "inconclusive"}


def load_matrix(path=MANIFEST):
    rows = json.loads(path.read_text())
    matrix = {row["id"]: row for row in rows}
    if len(matrix) != len(rows):
        raise ValueError("duplicate matrix row")
    return matrix


def review_receipts(attempt, revision, matrix):
    """Read immutable receipts in manifest order and require cleanup before resuming."""
    if re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ValueError("full revision required")
    receipts = {}
    for row_id in matrix:
        file = attempt / f"{row_id}.json"
        if not file.is_file():
            continue
        receipt = json.loads(file.read_text())
        if receipt["id"] != row_id or receipt["sha"] != revision:
            raise ValueError(f"wrong row or revision in {row_id}")
        if receipt["status"] not in STATUSES:
            raise ValueError(f"invalid status in {row_id}")
        if receipt["cleanup"] is not True:
            raise ValueError(f"unverified cleanup in {row_id}")
        counts = receipt["counts"]
        if any(type(value) is not int or value < 0 for value in counts.values()):
            raise ValueError(f"invalid count in {row_id}")
        receipts[row_id] = receipt
    return receipts


def remaining_rows(matrix, receipts, live_owned_nodes):
    if live_owned_nodes():
        raise ValueError("previous attempt still owns devices")
    return [row_id for row_id in matrix if row_id not in receipts]


def markdown_table(matrix, receipts):
    lines = ["| Step | Family | Realization | Result |",
             "| --- | --- | --- | --- |"]
    for row_id, row in matrix.items():
        status = receipts[row_id]["status"] if row_id in receipts else "unavailable"
        counts = receipts.get(row_id, {}).get("counts", {})
        suffix = ", ".join(f"{label} {value}" for label, value in sorted(counts.items())
                           if re.fullmatch(r"[a-z][a-z0-9 -]*", label))
        result = f"{status} ({suffix})" if suffix else status
        lines.append(f"| {row['step']} | {row['family']} | {row['realization']} | {result} |")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("attempt", type=Path)
    parser.add_argument("revision")
    args = parser.parse_args()
    matrix = load_matrix()
    print(markdown_table(matrix, review_receipts(args.attempt, args.revision, matrix)))


if __name__ == "__main__":
    main()
