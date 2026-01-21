from __future__ import annotations

import subprocess

import pytest

from tests.conftest import default_smoke_env, repo_root
from tests.e2e.plan import CwsE2EPlanCase, plan_cases


def _parse_stubbed_docker_argv(stdout: str) -> list[str]:
    lines = stdout.splitlines()
    if "docker" not in lines:
        raise AssertionError(f"stubbed docker output missing 'docker' sentinel:\n{stdout}")
    idx = lines.index("docker")
    return lines[idx + 1 :]


def _run_plan_case_stub(plan_case: CwsE2EPlanCase) -> list[str]:
    repo = repo_root()
    env = default_smoke_env(repo)

    # Ensure host environment doesn't accidentally affect wrapper behavior.
    for key in [
        "CWS_BASH_PATH",
        "CWS_DOCKER_ARGS",
        "CODEX_WORKSPACE_GPG",
        "CODEX_WORKSPACE_GPG_KEY",
        "GH_TOKEN",
        "GITHUB_TOKEN",
    ]:
        env.pop(key, None)

    env["CODEX_WORKSPACE_GPG"] = "none"
    env["CODEX_WORKSPACE_GPG_KEY"] = ""

    if plan_case.case.env:
        env.update({str(k): str(v) for k, v in plan_case.case.env.items()})

    argv = plan_case.command_argv
    if plan_case.wrapper == "bash" and argv[:2] == ["bash", "-lc"] and len(argv) >= 3:
        # `bash -l` may source host profile files and mutate PATH, bypassing our stubbed docker.
        argv = ["bash", "--noprofile", "--norc", "-c", argv[2]]

    completed = subprocess.run(
        argv,
        cwd=repo,
        env=env,
        text=True,
        capture_output=True,
    )
    combined = "\n".join([completed.stdout.strip(), completed.stderr.strip()]).strip()
    assert completed.returncode == 0, f"{plan_case.wrapper}:{plan_case.case.case_id} failed\n{combined}".strip()
    return _parse_stubbed_docker_argv(completed.stdout)


def _cli_expected_argv(case_id: str, cli_by_id: dict[str, CwsE2EPlanCase]) -> list[str]:
    if case_id in cli_by_id:
        return _run_plan_case_stub(cli_by_id[case_id])
    if case_id == "env_docker_args_array":
        return _run_plan_case_stub(cli_by_id["env_docker_args_string"])
    raise KeyError(case_id)


@pytest.fixture(scope="session")
def cli_by_id() -> dict[str, CwsE2EPlanCase]:
    return {c.case.case_id: c for c in plan_cases("cli")}


@pytest.mark.script_smoke
@pytest.mark.parametrize("plan_case", plan_cases("bash"), ids=lambda c: c.case.case_id)
def test_bash_wrapper_equivalence_against_cli(plan_case: CwsE2EPlanCase, cli_by_id: dict[str, CwsE2EPlanCase]) -> None:
    expected = _cli_expected_argv(plan_case.case.case_id, cli_by_id)
    actual = _run_plan_case_stub(plan_case)
    assert actual == expected


@pytest.mark.script_smoke
@pytest.mark.parametrize("plan_case", plan_cases("zsh"), ids=lambda c: c.case.case_id)
def test_zsh_wrapper_equivalence_against_cli(plan_case: CwsE2EPlanCase, cli_by_id: dict[str, CwsE2EPlanCase]) -> None:
    expected = _cli_expected_argv(plan_case.case.case_id, cli_by_id)
    actual = _run_plan_case_stub(plan_case)
    assert actual == expected
