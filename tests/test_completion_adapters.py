from __future__ import annotations

import os
import shlex
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest

from tests.conftest import default_smoke_env, repo_root

HOST_WORKSPACES = ["host-ws-a", "host-ws-b"]
CONTAINER_WORKSPACES = ["container-ws-a", "container-ws-b"]
LEGACY_WORKSPACES = ["legacy-ws-a", "legacy-ws-b"]

WORKSPACE_TAILS = {
    "auth": ["auth", "github", ""],
    "rm": ["rm", ""],
    "exec": ["exec", ""],
    "reset": ["reset", "repo", ""],
    "tunnel": ["tunnel", ""],
}


@dataclass(frozen=True)
class CompletionTarget:
    script: str
    function: str
    command_name: str
    shell: str
    case_id: str


BASH_TARGETS = [
    CompletionTarget(
        script="scripts/awl.bash",
        function="_awl_complete",
        command_name="awl",
        shell="bash",
        case_id="bash-awl",
    ),
    CompletionTarget(
        script="completions/agent-workspace-launcher.bash",
        function="_agent_workspace_launcher_complete",
        command_name="agent-workspace-launcher",
        shell="bash",
        case_id="bash-completion",
    ),
]

ZSH_TARGETS = [
    CompletionTarget(
        script="scripts/awl.zsh",
        function="_awl_completion",
        command_name="awl",
        shell="zsh",
        case_id="zsh-awl",
    ),
    CompletionTarget(
        script="completions/_agent-workspace-launcher",
        function="_agent-workspace-launcher",
        command_name="agent-workspace-launcher",
        shell="zsh",
        case_id="zsh-completion",
    ),
]

DOCKER_TARGETS = [
    CompletionTarget(
        script="scripts/awl_docker.bash",
        function="_awl_docker_complete",
        command_name="awl_docker",
        shell="bash",
        case_id="bash-awl-docker",
    ),
    CompletionTarget(
        script="scripts/awl_docker.zsh",
        function="_awl_docker_completion",
        command_name="awl_docker",
        shell="zsh",
        case_id="zsh-awl-docker",
    ),
]


def _shell_array_literal(words: list[str]) -> str:
    return " ".join(shlex.quote(word) for word in words)


def _write_completion_stub(bin_dir: Path) -> None:
    stub_path = bin_dir / "agent-workspace-launcher"
    stub_path.write_text(
        """#!/usr/bin/env bash
set -euo pipefail

log_path="${AWL_COMPLETION_LOG:-}"

if [[ "${1:-}" == "__complete" ]]; then
  if [[ -n "${log_path}" ]]; then
    printf '%s\\n' "$*" >>"${log_path}"
  fi

  shift
  shell_name=""
  cword="0"
  words=()
  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --shell)
        shell_name="${2:-}"
        shift 2
        ;;
      --cword)
        cword="${2:-0}"
        shift 2
        ;;
      --word)
        words+=("${2-}")
        shift 2
        ;;
      *)
        shift
        ;;
    esac
  done

  runtime="container"
  for ((i = 0; i < ${#words[@]}; i++)); do
    token="${words[$i]}"
    if [[ "${token}" == "--runtime" ]] && (( i + 1 < ${#words[@]} )); then
      runtime="${words[$((i + 1))]}"
    elif [[ "${token}" == --runtime=* ]]; then
      runtime="${token#--runtime=}"
    fi
  done

  subcmd=""
  for ((i = 1; i < ${#words[@]}; i++)); do
    token="${words[$i]}"
    case "${token}" in
      --runtime)
        ((i += 1))
        ;;
      --runtime=*|--help|--version|-h|-V)
        ;;
      auth|rm|exec|reset|tunnel|create|ls)
        subcmd="${token}"
        break
        ;;
    esac
  done

  idx=$((cword))
  prev=""
  if (( idx > 0 && idx <= ${#words[@]} )); then
    prev="${words[$((idx - 1))]}"
  fi

  should_suggest=0
  case "${subcmd}" in
    auth)
      case "${prev}" in
        github|codex|gpg) should_suggest=1 ;;
      esac
      ;;
    rm|exec|tunnel)
      if [[ "${prev}" == "${subcmd}" ]]; then
        should_suggest=1
      fi
      ;;
    reset)
      case "${prev}" in
        repo|work-repos|opt-repos|private-repo) should_suggest=1 ;;
      esac
      ;;
  esac

  if (( should_suggest == 1 )); then
    if [[ "${runtime}" == "host" ]]; then
      printf '%s\\n' "host-ws-a" "host-ws-b"
    else
      printf '%s\\n' "container-ws-a" "container-ws-b"
    fi
  fi
  exit 0
fi

if [[ "${1:-}" == "ls" ]]; then
  printf '%s\\n' "legacy-ws-a" "legacy-ws-b"
  exit 0
fi

printf '%s\\n' "agent-workspace-launcher"
for arg in "$@"; do
  printf '%s\\n' "${arg}"
done
""",
        encoding="utf-8",
    )
    stub_path.chmod(0o755)


def _completion_env(tmp_path: Path) -> tuple[dict[str, str], Path]:
    repo = repo_root()
    env = default_smoke_env(repo)
    bin_dir = tmp_path / "bin"
    log_path = tmp_path / "completion.calls.log"
    bin_dir.mkdir(parents=True, exist_ok=True)
    _write_completion_stub(bin_dir)
    env["PATH"] = os.pathsep.join([str(bin_dir), env.get("PATH", "")])
    env["AWL_COMPLETION_LOG"] = str(log_path)
    return env, log_path


def _run_bash_completion(
    target: CompletionTarget,
    words: list[str],
    env: dict[str, str],
) -> list[str]:
    repo = repo_root()
    array_literal = _shell_array_literal(words)
    cword = len(words) - 1
    command = "\n".join(
        [
            "set -euo pipefail",
            f"source {shlex.quote(target.script)}",
            f"COMP_WORDS=({array_literal})",
            f"COMP_CWORD={cword}",
            f"{target.function}",
            "printf '%s\\n' \"${COMPREPLY[@]}\"",
        ]
    )

    completed = subprocess.run(
        ["bash", "--noprofile", "--norc", "-c", command],
        cwd=repo,
        env=env,
        text=True,
        capture_output=True,
    )
    combined = "\n".join([completed.stdout.strip(), completed.stderr.strip()]).strip()
    assert completed.returncode == 0, (
        f"{target.case_id} completion failed with exit {completed.returncode}\n{combined}"
    )
    return [line for line in completed.stdout.splitlines() if line]


def _run_zsh_completion(
    target: CompletionTarget,
    words: list[str],
    env: dict[str, str],
) -> list[str]:
    repo = repo_root()
    array_literal = _shell_array_literal(words)
    current = len(words)
    command = "\n".join(
        [
            "set -euo pipefail",
            "typeset -ga __awl_completion_capture",
            "compadd() {",
            "  local seen=0",
            "  local arg",
            "  __awl_completion_capture=()",
            '  for arg in "$@"; do',
            "    if (( seen )); then",
            '      __awl_completion_capture+=("${arg}")',
            '    elif [[ "${arg}" == "--" ]]; then',
            "      seen=1",
            "    fi",
            "  done",
            "}",
            f"source {shlex.quote(target.script)}",
            f"words=({array_literal})",
            f"CURRENT={current}",
            f"{target.function}",
            "printf '%s\\n' \"${__awl_completion_capture[@]}\"",
        ]
    )

    completed = subprocess.run(
        ["zsh", "-f", "-c", command],
        cwd=repo,
        env=env,
        text=True,
        capture_output=True,
    )
    combined = "\n".join([completed.stdout.strip(), completed.stderr.strip()]).strip()
    assert completed.returncode == 0, (
        f"{target.case_id} completion failed with exit {completed.returncode}\n{combined}"
    )
    return [line for line in completed.stdout.splitlines() if line]


def _run_completion(
    target: CompletionTarget,
    words: list[str],
    env: dict[str, str],
) -> list[str]:
    if target.shell == "bash":
        return _run_bash_completion(target, words, env)
    if target.shell == "zsh":
        return _run_zsh_completion(target, words, env)
    raise ValueError(f"unsupported shell: {target.shell}")


@pytest.mark.parametrize("target", BASH_TARGETS + ZSH_TARGETS, ids=lambda t: t.case_id)
@pytest.mark.parametrize("runtime", ["host", "container"])
@pytest.mark.parametrize("subcommand", ["auth", "rm", "exec", "reset", "tunnel"])
def test_runtime_aware_workspace_completions(
    tmp_path: Path, target: CompletionTarget, runtime: str, subcommand: str
) -> None:
    env, _ = _completion_env(tmp_path)
    words = [target.command_name, "--runtime", runtime, *WORKSPACE_TAILS[subcommand]]

    actual = _run_completion(target, words, env)
    expected = HOST_WORKSPACES if runtime == "host" else CONTAINER_WORKSPACES
    assert actual == expected


@pytest.mark.parametrize("target", DOCKER_TARGETS, ids=lambda t: t.case_id)
@pytest.mark.parametrize("subcommand", ["auth", "rm", "exec", "reset", "tunnel"])
def test_docker_wrappers_default_to_container_workspace_completions(
    tmp_path: Path, target: CompletionTarget, subcommand: str
) -> None:
    env, _ = _completion_env(tmp_path)
    words = [target.command_name, *WORKSPACE_TAILS[subcommand]]

    actual = _run_completion(target, words, env)
    assert actual == CONTAINER_WORKSPACES


@pytest.mark.parametrize("target", BASH_TARGETS + ZSH_TARGETS, ids=lambda t: t.case_id)
def test_legacy_mode_uses_static_completion_path(
    tmp_path: Path, target: CompletionTarget
) -> None:
    env, log_path = _completion_env(tmp_path)
    env["AGENT_WORKSPACE_COMPLETION_MODE"] = "legacy"
    words = [target.command_name, "exec", ""]

    actual = _run_completion(target, words, env)
    assert actual == LEGACY_WORKSPACES
    assert not log_path.exists() or log_path.read_text(encoding="utf-8").strip() == ""
