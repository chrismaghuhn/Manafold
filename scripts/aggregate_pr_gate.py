#!/usr/bin/env python3
"""Evaluate the stable pull-request aggregate check fail-closed."""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

REQUIRED_CHECKS = (
    "fast",
    "integration",
    "Analyze (actions)",
    "Analyze (python)",
    "Analyze (rust)",
    "CodeQL",
)
WAIT_EXIT = 10


def aggregate_statuses(
    statuses: Mapping[str, str],
    required: Sequence[str] = REQUIRED_CHECKS,
) -> bool:
    """Return true only when every required check has conclusion success."""

    return all(statuses.get(name) == "success" for name in required)


def check_run_statuses(payload: Mapping[str, Any], expected_head: str) -> dict[str, str]:
    """Project check runs for the expected head into name -> status."""

    runs = payload.get("check_runs")
    if not isinstance(runs, list):
        return {}

    statuses: dict[str, str] = {}
    for run in runs:
        if not isinstance(run, dict) or run.get("head_sha") != expected_head:
            continue
        name = run.get("name")
        if not isinstance(name, str):
            continue
        if name in statuses:
            statuses[name] = "duplicate"
            continue
        if run.get("status") == "completed":
            conclusion = run.get("conclusion")
            statuses[name] = conclusion if isinstance(conclusion, str) else "missing"
        else:
            status = run.get("status")
            statuses[name] = status if isinstance(status, str) else "missing"
    return statuses


def evaluate(
    payload: Mapping[str, Any],
    expected_head: str,
    required: Sequence[str] = REQUIRED_CHECKS,
) -> tuple[str, tuple[str, ...]]:
    """Return PASS, FAIL, or WAIT plus the relevant check names."""

    statuses = check_run_statuses(payload, expected_head)
    if aggregate_statuses(statuses, required):
        return "PASS", ()

    non_success = tuple(
        name
        for name in required
        if name in statuses
        and statuses[name] not in {"queued", "in_progress", "requested", "pending"}
        and statuses[name] != "success"
    )
    if non_success:
        return "FAIL", non_success

    waiting = tuple(name for name in required if statuses.get(name) != "success")
    return "WAIT", waiting


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--check-runs", type=Path, required=True)
    parser.add_argument("--wait", action="store_true")
    args = parser.parse_args()

    try:
        payload = json.loads(args.check_runs.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"FAIL: cannot read check-run payload: {error}", file=sys.stderr)
        return 1
    if not isinstance(payload, dict):
        print("FAIL: check-run payload is not an object", file=sys.stderr)
        return 1

    status, names = evaluate(payload, args.head_sha)
    if status == "PASS":
        print("PASS: manafold-pr-gate")
        return 0
    if status == "WAIT" and args.wait:
        print(f"WAIT: mandatory checks pending or missing: {', '.join(names)}")
        return WAIT_EXIT
    print(f"FAIL: mandatory checks are not successful: {', '.join(names)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
