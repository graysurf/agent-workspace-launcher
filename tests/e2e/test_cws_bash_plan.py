from __future__ import annotations

import pytest

from .plan import run_cws_e2e_flow


@pytest.mark.e2e
def test_cws_bash_e2e_flow() -> None:
    run_cws_e2e_flow("bash")
