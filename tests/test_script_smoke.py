from __future__ import annotations

import re
import shlex
import subprocess
import time
from pathlib import Path
from typing import Any, cast

import pytest

from .conftest import SCRIPT_SMOKE_RUN_RESULTS, ScriptRunResult, default_smoke_env, load_script_specs, out_dir_smoke, repo_root


def parse_shebang(script_path: Path) -> list[str]:
    first = script_path.read_text("utf-8", errors="ignore").splitlines()[:1]
    if not first:
        return []
    line = first[0].strip()
    if not line.startswith("#!"):
        return []

    tokens = shlex.split(line[2:].strip())
    if not tokens:
        return []

    if Path(tokens[0]).name == "env":
        tokens = tokens[1:]
        if tokens[:1] == ["-S"]:
            tokens = tokens[1:]

    return tokens


def write_logs(script: str, case: str, stdout: str, stderr: str) -> tuple[str, str]:
    logs_root = out_dir_smoke() / "logs"
    suffix = f".{case}" if case else ""
    stdout_path = logs_root / f"{script}{suffix}.stdout.txt"
    stderr_path = logs_root / f"{script}{suffix}.stderr.txt"
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(stdout, "utf-8")
    stderr_path.write_text(stderr, "utf-8")
    return (str(stdout_path), str(stderr_path))


def compile_optional_regex(pattern: str | None) -> re.Pattern[str] | None:
    if not pattern:
        return None
    return re.compile(pattern, re.MULTILINE)


def run_smoke_script(
    script: str,
    case: str,
    spec: dict[str, Any],
    repo: Path,
    *,
    cwd: Path | None = None,
) -> ScriptRunResult:
    script_path = repo / script
    if not script_path.exists():
        raise FileNotFoundError(script)

    args_raw = spec.get("args", [])
    if not isinstance(args_raw, list):
        raise TypeError(f"spec.args must be a list of strings: {script} ({case})")
    args_items = cast(list[object], args_raw)
    args: list[str] = []
    for item in args_items:
        if not isinstance(item, str):
            raise TypeError(f"spec.args must be a list of strings: {script} ({case})")
        args.append(item)

    command_raw = spec.get("command")
    command: list[str] | None = None
    if command_raw is not None:
        if not isinstance(command_raw, list) or not command_raw:
            raise TypeError(f"spec.command must be a non-empty list of strings: {script} ({case})")
        command_items = cast(list[object], command_raw)
        command = []
        for item in command_items:
            if not isinstance(item, str):
                raise TypeError(f"spec.command must be a non-empty list of strings: {script} ({case})")
            command.append(item)
        if args:
            raise TypeError(f"spec.args must be empty when spec.command is set: {script} ({case})")

    timeout_sec = spec.get("timeout_sec", 10)
    if not isinstance(timeout_sec, (int, float)):
        raise TypeError(f"spec.timeout_sec must be a number: {script} ({case})")

    env = default_smoke_env(repo)
    extra_env_raw = spec.get("env", {})
    if extra_env_raw:
        if not isinstance(extra_env_raw, dict):
            raise TypeError(f"spec.env must be a JSON object: {script} ({case})")
        extra_env = cast(dict[object, object], extra_env_raw)
        for key_obj, value_obj in extra_env.items():
            key = str(key_obj)
            if value_obj is None:
                env.pop(key, None)
            else:
                env[key] = str(value_obj)

    argv: list[str]
    if command is not None:
        argv = list(command)
    else:
        shebang = parse_shebang(script_path)
        if not shebang:
            raise ValueError(f"missing shebang: {script}")
        argv = shebang + [str(script_path)] + list(args)

    expect_raw = spec.get("expect", {})
    if expect_raw and not isinstance(expect_raw, dict):
        raise TypeError(f"spec.expect must be a JSON object: {script} ({case})")
    if not isinstance(expect_raw, dict):
        raise TypeError(f"spec.expect must be a JSON object: {script} ({case})")
    expect = cast(dict[object, object], expect_raw)

    exit_codes_raw = expect.get("exit_codes", [0])
    if not isinstance(exit_codes_raw, list):
        raise TypeError(f"expect.exit_codes must be a list of ints: {script} ({case})")
    exit_code_items = cast(list[object], exit_codes_raw)
    exit_codes: list[int] = []
    for item in exit_code_items:
        if not isinstance(item, int):
            raise TypeError(f"expect.exit_codes must be a list of ints: {script} ({case})")
        exit_codes.append(item)

    stdout_pat_raw = expect.get("stdout_regex")
    if stdout_pat_raw is None:
        stdout_pat: str | None = None
    elif isinstance(stdout_pat_raw, str):
        stdout_pat = stdout_pat_raw
    else:
        raise TypeError(f"expect.stdout_regex must be a string: {script} ({case})")

    stderr_pat_raw = expect.get("stderr_regex")
    if stderr_pat_raw is None:
        stderr_pat: str | None = None
    elif isinstance(stderr_pat_raw, str):
        stderr_pat = stderr_pat_raw
    else:
        raise TypeError(f"expect.stderr_regex must be a string: {script} ({case})")

    stdout_re = compile_optional_regex(stdout_pat)
    stderr_re = compile_optional_regex(stderr_pat)

    start = time.monotonic()
    try:
        completed = subprocess.run(
            argv,
            cwd=str(cwd or repo),
            env=env,
            text=True,
            capture_output=True,
            timeout=float(timeout_sec),
        )
        duration_ms = int((time.monotonic() - start) * 1000)
        stdout = completed.stdout
        stderr = completed.stderr
        stdout_path, stderr_path = write_logs(script, case, stdout, stderr)

        ok = completed.returncode in exit_codes
        note_parts: list[str] = []
        if completed.returncode not in exit_codes:
            note_parts.append(f"exit={completed.returncode} expected={exit_codes}")
        if stdout_re and not stdout_re.search(stdout):
            ok = False
            note_parts.append("stdout_regex_mismatch")
        if stderr_re and not stderr_re.search(stderr):
            ok = False
            note_parts.append("stderr_regex_mismatch")

        status = "pass" if ok else "fail"
        note = "; ".join(note_parts) if note_parts else None
        return ScriptRunResult(
            script=script,
            argv=argv,
            exit_code=completed.returncode,
            duration_ms=duration_ms,
            stdout_path=stdout_path,
            stderr_path=stderr_path,
            status=status,
            note=note,
            case=case,
        )
    except subprocess.TimeoutExpired:
        duration_ms = int((time.monotonic() - start) * 1000)
        stdout_path, stderr_path = write_logs(script, case, "", "")
        return ScriptRunResult(
            script=script,
            argv=argv,
            exit_code=124,
            duration_ms=duration_ms,
            stdout_path=stdout_path,
            stderr_path=stderr_path,
            status="fail",
            note=f"timeout after {timeout_sec}s",
            case=case,
        )


def discover_smoke_cases() -> list[tuple[str, str, dict[str, Any]]]:
    repo = repo_root()
    specs = load_script_specs(repo / "tests" / "script_specs")
    discovered: list[tuple[str, str, dict[str, Any]]] = []

    for script, spec in sorted(specs.items()):
        smoke_raw = spec.get("smoke")
        if not smoke_raw:
            continue

        smoke_cases: list[object]
        if isinstance(smoke_raw, list):
            smoke_cases = cast(list[object], smoke_raw)
        elif isinstance(smoke_raw, dict):
            smoke_dict = cast(dict[object, object], smoke_raw)
            cases_raw = smoke_dict.get("cases")
            if not isinstance(cases_raw, list):
                raise TypeError(f"spec.smoke must be a list (or {{cases:[...]}}): {script}")
            smoke_cases = cast(list[object], cases_raw)
        else:
            raise TypeError(f"spec.smoke must be a list (or {{cases:[...]}}): {script}")

        for idx, case_obj in enumerate(smoke_cases, start=1):
            if not isinstance(case_obj, dict):
                raise TypeError(f"smoke case must be a JSON object: {script} (case {idx})")
            case_dict = cast(dict[str, Any], case_obj)

            name_raw = case_dict.get("name", f"case-{idx}")
            if not isinstance(name_raw, str) or not name_raw.strip():
                raise TypeError(f"smoke case name must be a non-empty string: {script} (case {idx})")
            discovered.append((script, name_raw.strip(), case_dict))

    return sorted(discovered, key=lambda x: (x[0], x[1]))


@pytest.mark.script_smoke
@pytest.mark.parametrize(("script", "case", "spec"), discover_smoke_cases())
def test_script_smoke_spec(script: str, case: str, spec: dict[str, Any]):
    repo = repo_root()

    result = run_smoke_script(script, case, spec, repo)
    SCRIPT_SMOKE_RUN_RESULTS.append(result)

