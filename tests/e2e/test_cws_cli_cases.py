from __future__ import annotations

import os

import pytest

from .plan import CwsE2EPlanCase, plan_cases, run_cws_e2e


def _selected_case_ids() -> set[str]:
    raw = os.environ.get("CWS_E2E_CASE", "").strip()
    if not raw:
        return set()
    tokens = [t.strip() for t in raw.split(",") if t.strip()]
    selected: set[str] = set()
    for token in tokens:
        if token.startswith("cli:"):
            selected.add(token[len("cli:") :])
        else:
            selected.add(token)
    return selected


def _selected_plan_cases() -> list[CwsE2EPlanCase]:
    full = os.environ.get("CWS_E2E_FULL", "").lower() in {"1", "true", "yes", "on"}
    allow_rm_all = os.environ.get("CWS_E2E_ALLOW_RM_ALL", "").lower() in {
        "1",
        "true",
        "yes",
        "on",
    }
    selected = _selected_case_ids()
    if not full and not selected:
        return []

    cases = plan_cases("cli")
    if not selected and not allow_rm_all:
        cases = [c for c in cases if c.case.case_id != "rm_all_yes"]
    if not selected:
        return cases
    filtered = [c for c in cases if c.case.case_id in selected]
    if not filtered:
        raise ValueError(
            f"CWS_E2E_CASE did not match any cli plan cases: {sorted(selected)}"
        )
    return filtered


@pytest.mark.e2e
@pytest.mark.parametrize(
    "plan_case", _selected_plan_cases(), ids=lambda c: c.case.case_id
)
def test_cws_cli_e2e_case(plan_case: CwsE2EPlanCase) -> None:
    run_cws_e2e(plan_case)
