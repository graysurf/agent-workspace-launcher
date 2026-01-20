from __future__ import annotations

import subprocess
import sys

from .conftest import repo_root


def test_pyright() -> None:
    repo = repo_root()

    completed = subprocess.run(
        [sys.executable, "-m", "pyright", "-p", str(repo)],
        cwd=repo,
        text=True,
        capture_output=True,
    )

    combined = "\n".join([completed.stdout.strip(), completed.stderr.strip()]).strip()
    assert completed.returncode == 0, f"pyright failed (exit={completed.returncode})\n{combined}".strip()

