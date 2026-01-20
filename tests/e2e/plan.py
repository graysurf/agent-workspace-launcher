from __future__ import annotations

import shlex
from dataclasses import dataclass
from pathlib import Path

from tests.conftest import repo_root


@dataclass(frozen=True)
class CwsE2ECase:
    case_id: str
    cws_args: list[str]
    purpose: str
    requires: str | None = None
    env: dict[str, str] | None = None
    prelude: str | None = None


@dataclass(frozen=True)
class CwsE2EPlanCase:
    wrapper: str  # cli|bash|zsh
    case: CwsE2ECase
    command_argv: list[str]
    command_display: str


def _shell_join(parts: list[str]) -> str:
    return " ".join(shlex.quote(p) for p in parts)


def _env_prefix(env: dict[str, str] | None) -> str:
    if not env:
        return ""
    return " ".join(f"{k}={shlex.quote(v)}" for k, v in sorted(env.items())) + " "


def _repo_rel(path: Path) -> str:
    return path.relative_to(repo_root()).as_posix()


def _build_cli(case: CwsE2ECase) -> CwsE2EPlanCase:
    script = repo_root() / "scripts" / "cws"
    argv = [str(script), *case.cws_args]
    display = f"{_env_prefix(case.env)}{_repo_rel(script)} {_shell_join(case.cws_args)}".rstrip()
    return CwsE2EPlanCase(wrapper="cli", case=case, command_argv=argv, command_display=display)


def _build_bash(case: CwsE2ECase) -> CwsE2EPlanCase:
    script = repo_root() / "scripts" / "cws.bash"
    joined = _shell_join(case.cws_args)

    lines: list[str] = []
    if case.prelude:
        lines.append(case.prelude)
    lines.append(f"source {shlex.quote(_repo_rel(script))}")
    lines.append(f"cws {joined}".rstrip())
    command = "\n".join(lines).rstrip() + "\n"

    argv = ["bash", "-lc", command]
    display = f"{_env_prefix(case.env)}bash -lc {shlex.quote(command.strip())}"
    return CwsE2EPlanCase(wrapper="bash", case=case, command_argv=argv, command_display=display)


def _build_zsh(case: CwsE2ECase) -> CwsE2EPlanCase:
    script = repo_root() / "scripts" / "cws.zsh"
    joined = _shell_join(case.cws_args)

    lines: list[str] = []
    if case.prelude:
        lines.append(case.prelude)
    lines.append(f"source {shlex.quote(_repo_rel(script))}")
    lines.append(f"cws {joined}".rstrip())
    command = "\n".join(lines).rstrip() + "\n"

    argv = ["zsh", "-f", "-c", command]
    display = f"{_env_prefix(case.env)}zsh -f -c {shlex.quote(command.strip())}"
    return CwsE2EPlanCase(wrapper="zsh", case=case, command_argv=argv, command_display=display)


def base_cases() -> list[CwsE2ECase]:
    return [
        # help
        CwsE2ECase(
            case_id="help",
            cws_args=["--help"],
            purpose="Show top-level help and usage.",
        ),
        CwsE2ECase(case_id="help_auth", cws_args=["auth", "--help"], purpose="Show help for auth."),
        CwsE2ECase(case_id="help_create", cws_args=["create", "--help"], purpose="Show help for create."),
        CwsE2ECase(case_id="help_ls", cws_args=["ls", "--help"], purpose="Show help for ls."),
        CwsE2ECase(case_id="help_exec", cws_args=["exec", "--help"], purpose="Show help for exec."),
        CwsE2ECase(case_id="help_rm", cws_args=["rm", "--help"], purpose="Show help for rm."),
        CwsE2ECase(case_id="help_reset", cws_args=["reset", "--help"], purpose="Show help for reset."),
        CwsE2ECase(case_id="help_tunnel", cws_args=["tunnel", "--help"], purpose="Show help for tunnel."),
        # create
        CwsE2ECase(
            case_id="create_public_owner_repo",
            cws_args=["create", "OWNER/REPO"],
            purpose="Create a workspace from a public repo in OWNER/REPO form.",
            requires="Docker daemon running; network access.",
        ),
        CwsE2ECase(
            case_id="create_public_https",
            cws_args=["create", "https://github.com/OWNER/REPO"],
            purpose="Create a workspace from a public repo via https URL.",
            requires="Docker daemon running; network access.",
        ),
        CwsE2ECase(
            case_id="create_public_https_git_suffix",
            cws_args=["create", "https://github.com/OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via https URL (with .git).",
            requires="Docker daemon running; network access.",
        ),
        CwsE2ECase(
            case_id="create_public_ssh_scp_style",
            cws_args=["create", "git@github.com:OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via SSH scp-style URL.",
            requires="Docker daemon running; network access (or SSH configured).",
        ),
        CwsE2ECase(
            case_id="create_public_ssh_url_style",
            cws_args=["create", "ssh://git@github.com/OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via SSH URL form.",
            requires="Docker daemon running; network access (or SSH configured).",
        ),
        CwsE2ECase(
            case_id="create_no_extras",
            cws_args=["create", "--no-extras", "OWNER/REPO"],
            purpose="Create workspace while skipping ~/.private and extra repos.",
        ),
        CwsE2ECase(
            case_id="create_seed_private_repo",
            cws_args=["create", "--private-repo", "OWNER/PRIVATE_REPO", "OWNER/REPO"],
            purpose="Seed ~/.private from a repo during create.",
            requires="Valid GitHub token; access to the private repo.",
        ),
        CwsE2ECase(
            case_id="create_no_work_repos_with_name",
            cws_args=["create", "--no-work-repos", "--name", "ws-e2e"],
            purpose="Create workspace without cloning repos, using an explicit name.",
        ),
        CwsE2ECase(
            case_id="create_with_name",
            cws_args=["create", "--name", "ws-e2e", "OWNER/REPO"],
            purpose="Create workspace with explicit name override.",
        ),
        # list
        CwsE2ECase(
            case_id="ls",
            cws_args=["ls"],
            purpose="List workspaces.",
        ),
        # exec
        CwsE2ECase(
            case_id="exec_command",
            cws_args=["exec", "ws-e2e", "git", "status"],
            purpose="Run a non-interactive command in the workspace container.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="exec_shell",
            cws_args=["exec", "ws-e2e"],
            purpose="Open an interactive shell in the workspace container.",
            requires="Existing workspace container ws-e2e; interactive TTY support.",
        ),
        CwsE2ECase(
            case_id="exec_root",
            cws_args=["exec", "--root", "ws-e2e", "id", "-u"],
            purpose="Exec as root and verify uid=0.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="exec_user",
            cws_args=["exec", "--user", "codex", "ws-e2e", "id", "-u"],
            purpose="Exec as a specific user and verify expected uid.",
            requires="Existing workspace container ws-e2e.",
        ),
        # auth
        CwsE2ECase(
            case_id="auth_github",
            cws_args=["auth", "github", "ws-e2e"],
            purpose="Update GitHub auth inside the workspace.",
            requires="Existing workspace container ws-e2e; valid GitHub token or `gh` login.",
        ),
        CwsE2ECase(
            case_id="auth_github_host",
            cws_args=["auth", "github", "--host", "github.com", "ws-e2e"],
            purpose="Update GitHub auth with an explicit host override.",
            requires="Existing workspace container ws-e2e; valid GitHub token or `gh` login.",
        ),
        CwsE2ECase(
            case_id="auth_codex_profile",
            cws_args=["auth", "codex", "--profile", "work", "ws-e2e"],
            purpose="Apply Codex auth (profile-based) inside the workspace.",
            requires="Existing workspace container ws-e2e; host Codex secrets must be accessible to the launcher container.",
        ),
        CwsE2ECase(
            case_id="auth_gpg_key",
            cws_args=["auth", "gpg", "--key", "DEADBEEF", "ws-e2e"],
            purpose="Import a GPG signing key into the workspace.",
            requires="Existing workspace container ws-e2e; host GPG keyring accessible; keyid DEADBEEF exists.",
        ),
        # reset
        CwsE2ECase(
            case_id="reset_repo",
            cws_args=["reset", "repo", "ws-e2e"],
            purpose="Reset the primary repo inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="reset_work_repos",
            cws_args=["reset", "work-repos", "ws-e2e"],
            purpose="Reset work repos inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="reset_opt_repos",
            cws_args=["reset", "opt-repos", "ws-e2e"],
            purpose="Reset optional repos inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="reset_private_repo",
            cws_args=["reset", "private-repo", "ws-e2e"],
            purpose="Reset the private repo inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        # tunnel
        CwsE2ECase(
            case_id="tunnel_foreground",
            cws_args=["tunnel", "ws-e2e"],
            purpose="Start a VS Code tunnel (foreground).",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        CwsE2ECase(
            case_id="tunnel_detach",
            cws_args=["tunnel", "ws-e2e", "--detach"],
            purpose="Start a VS Code tunnel in the background.",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        CwsE2ECase(
            case_id="tunnel_named",
            cws_args=["tunnel", "ws-e2e", "--name", "ws-e2e-tunnel"],
            purpose="Start a tunnel with an explicit name.",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        # rm
        CwsE2ECase(
            case_id="rm_workspace_yes",
            cws_args=["rm", "ws-e2e", "--yes"],
            purpose="Remove a specific workspace without confirmation.",
            requires="Existing workspace container ws-e2e.",
        ),
        CwsE2ECase(
            case_id="rm_all_yes",
            cws_args=["rm", "--all", "--yes"],
            purpose="Remove all workspaces without confirmation.",
            requires="At least one existing workspace container.",
        ),
        # env/config
        CwsE2ECase(
            case_id="env_custom_image",
            cws_args=["ls"],
            env={"CWS_IMAGE": "graysurf/codex-workspace-launcher:latest"},
            purpose="Verify custom image selection is respected.",
        ),
        CwsE2ECase(
            case_id="env_auth_env",
            cws_args=["ls"],
            env={"CWS_AUTH": "env"},
            purpose="Force env-based auth mode (no gh keyring lookup).",
        ),
        CwsE2ECase(
            case_id="env_auth_none",
            cws_args=["ls"],
            env={"CWS_AUTH": "none"},
            purpose="Disable auth-related behavior entirely.",
        ),
    ]


def wrapper_extra_cases(wrapper: str) -> list[CwsE2ECase]:
    if wrapper == "bash":
        return [
            CwsE2ECase(
                case_id="env_docker_args_string",
                cws_args=["ls"],
                env={"CWS_DOCKER_ARGS": "-e FOO=bar -e BAZ=qux"},
                purpose="Verify extra docker args are passed through (string form).",
            ),
            CwsE2ECase(
                case_id="env_docker_args_array",
                cws_args=["ls"],
                prelude="CWS_DOCKER_ARGS=(-e FOO=bar -e BAZ=qux)",
                purpose="Verify extra docker args are passed through (array form).",
            ),
        ]

    if wrapper == "zsh":
        return [
            CwsE2ECase(
                case_id="env_docker_args_string",
                cws_args=["ls"],
                env={"CWS_DOCKER_ARGS": "-e FOO=bar -e BAZ=qux"},
                purpose="Verify extra docker args are passed through (string form).",
            ),
            CwsE2ECase(
                case_id="env_docker_args_array",
                cws_args=["ls"],
                prelude="CWS_DOCKER_ARGS=(-e FOO=bar -e BAZ=qux)",
                purpose="Verify extra docker args are passed through (array form).",
            ),
        ]

    return []


def plan_cases(wrapper: str) -> list[CwsE2EPlanCase]:
    if wrapper not in {"cli", "bash", "zsh"}:
        raise ValueError(f"unknown wrapper: {wrapper}")

    cases = [*base_cases(), *wrapper_extra_cases(wrapper)]
    if wrapper == "cli":
        return [_build_cli(c) for c in cases if not c.prelude]
    if wrapper == "bash":
        return [_build_bash(c) for c in cases]
    return [_build_zsh(c) for c in cases]


def case_id(plan_case: CwsE2EPlanCase) -> str:
    return f"{plan_case.wrapper}:{plan_case.case.case_id}"


def run_cws_e2e(_case: CwsE2EPlanCase) -> None:
    # TODO: Implement subprocess invocation + assertions.
    # - For cli: run case.command_argv with env=case.case.env and cwd=repo_root().
    # - For bash/zsh: run case.command_argv (shell -c) with env=case.case.env and cwd=repo_root().
    # - Parse create output (workspace/path) and thread it through dependent cases.
    # - Enforce cleanup (rm) even on failure.
    return None
