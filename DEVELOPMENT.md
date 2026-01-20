# Development Guide

## Testing

### Setup

- Create the virtual environment: `python3 -m venv .venv`
- Install dev deps: `.venv/bin/python -m pip install -r requirements-dev.txt`

### Run all tests

- `.venv/bin/python -m pytest`

### Smoke tests (no real Docker)

- `.venv/bin/python -m pytest -m script_smoke`
- These tests stub `docker` via `tests/stubs/bin` and validate the `cws` wrapper output.

### E2E tests (real Docker; placeholder only)

- `.venv/bin/python -m pytest -m e2e`
- Currently skipped; intended for real `cws create/rm` coverage later.

### Artifacts

- Smoke/test summaries and coverage are written to `out/tests/`.
