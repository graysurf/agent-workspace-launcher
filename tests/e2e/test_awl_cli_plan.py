from __future__ import annotations

import pytest

from .plan import run_awl_e2e_flow


@pytest.mark.e2e
def test_awl_cli_e2e_flow() -> None:
    run_awl_e2e_flow("cli")
