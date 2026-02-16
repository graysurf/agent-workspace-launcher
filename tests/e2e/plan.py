from __future__ import annotations

import contextlib
import shlex
from dataclasses import dataclass
import json
import os
import re
import subprocess
import time
from pathlib import Path
from typing import Iterator

try:  # pragma: no cover
    import fcntl  # type: ignore[attr-defined]
except Exception:  # pragma: no cover
    fcntl = None  # type: ignore[assignment]

import pytest

from tests.conftest import repo_root


@dataclass(frozen=True)
class AwsE2ECase:
    case_id: str
    aws_args: list[str]
    purpose: str
    requires: str | None = None
    env: dict[str, str] | None = None
    prelude: str | None = None


@dataclass(frozen=True)
class AwsE2EPlanCase:
    wrapper: str  # cli|bash|zsh
    case: AwsE2ECase
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


def _build_cli(case: AwsE2ECase) -> AwsE2EPlanCase:
    script = repo_root() / "scripts" / "aws.bash"
    argv = [str(script), *case.aws_args]
    display = f"{_env_prefix(case.env)}{_repo_rel(script)} {_shell_join(case.aws_args)}".rstrip()
    return AwsE2EPlanCase(
        wrapper="cli", case=case, command_argv=argv, command_display=display
    )


def _build_bash(case: AwsE2ECase) -> AwsE2EPlanCase:
    script = repo_root() / "scripts" / "aws.bash"
    joined = _shell_join(case.aws_args)

    lines: list[str] = []
    if case.prelude:
        lines.append(case.prelude)
    lines.append(f"source {shlex.quote(_repo_rel(script))}")
    lines.append(f"aws {joined}".rstrip())
    command = "\n".join(lines).rstrip() + "\n"

    argv = ["bash", "-lc", command]
    display = f"{_env_prefix(case.env)}bash -lc {shlex.quote(command.strip())}"
    return AwsE2EPlanCase(
        wrapper="bash", case=case, command_argv=argv, command_display=display
    )


def _build_zsh(case: AwsE2ECase) -> AwsE2EPlanCase:
    script = repo_root() / "scripts" / "aws.zsh"
    joined = _shell_join(case.aws_args)

    lines: list[str] = []
    if case.prelude:
        lines.append(case.prelude)
    lines.append(f"source {shlex.quote(_repo_rel(script))}")
    lines.append(f"aws {joined}".rstrip())
    command = "\n".join(lines).rstrip() + "\n"

    argv = ["zsh", "-f", "-c", command]
    display = f"{_env_prefix(case.env)}zsh -f -c {shlex.quote(command.strip())}"
    return AwsE2EPlanCase(
        wrapper="zsh", case=case, command_argv=argv, command_display=display
    )


def base_cases() -> list[AwsE2ECase]:
    return [
        # help
        AwsE2ECase(
            case_id="help",
            aws_args=["--help"],
            purpose="Show top-level help and usage.",
        ),
        AwsE2ECase(
            case_id="help_auth",
            aws_args=["auth", "--help"],
            purpose="Show help for auth.",
        ),
        AwsE2ECase(
            case_id="help_create",
            aws_args=["create", "--help"],
            purpose="Show help for create.",
        ),
        AwsE2ECase(
            case_id="help_ls", aws_args=["ls", "--help"], purpose="Show help for ls."
        ),
        AwsE2ECase(
            case_id="help_exec",
            aws_args=["exec", "--help"],
            purpose="Show help for exec.",
        ),
        AwsE2ECase(
            case_id="help_rm", aws_args=["rm", "--help"], purpose="Show help for rm."
        ),
        AwsE2ECase(
            case_id="help_reset",
            aws_args=["reset", "--help"],
            purpose="Show help for reset.",
        ),
        AwsE2ECase(
            case_id="help_tunnel",
            aws_args=["tunnel", "--help"],
            purpose="Show help for tunnel.",
        ),
        # create
        AwsE2ECase(
            case_id="create_public_owner_repo",
            aws_args=["create", "OWNER/REPO"],
            purpose="Create a workspace from a public repo in OWNER/REPO form.",
            requires="Docker daemon running; network access.",
        ),
        AwsE2ECase(
            case_id="create_public_https",
            aws_args=["create", "https://github.com/OWNER/REPO"],
            purpose="Create a workspace from a public repo via https URL.",
            requires="Docker daemon running; network access.",
        ),
        AwsE2ECase(
            case_id="create_public_https_git_suffix",
            aws_args=["create", "https://github.com/OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via https URL (with .git).",
            requires="Docker daemon running; network access.",
        ),
        AwsE2ECase(
            case_id="create_public_ssh_scp_style",
            aws_args=["create", "git@github.com:OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via SSH scp-style URL.",
            requires="Docker daemon running; network access (or SSH configured).",
        ),
        AwsE2ECase(
            case_id="create_public_ssh_url_style",
            aws_args=["create", "ssh://git@github.com/OWNER/REPO.git"],
            purpose="Create a workspace from a public repo via SSH URL form.",
            requires="Docker daemon running; network access (or SSH configured).",
        ),
        AwsE2ECase(
            case_id="create_no_extras",
            aws_args=["create", "--no-extras", "OWNER/REPO"],
            purpose="Create workspace while skipping ~/.private and extra repos.",
        ),
        AwsE2ECase(
            case_id="create_seed_private_repo",
            aws_args=["create", "--private-repo", "OWNER/PRIVATE_REPO", "OWNER/REPO"],
            purpose="Seed ~/.private from a repo during create.",
            requires="Valid GitHub token; access to the private repo.",
        ),
        AwsE2ECase(
            case_id="create_no_work_repos_with_name",
            aws_args=["create", "--no-work-repos", "--name", "ws-e2e"],
            purpose="Create workspace without cloning repos, using an explicit name.",
        ),
        AwsE2ECase(
            case_id="create_multiple_repos",
            aws_args=["create", "OWNER/REPO", "OWNER/REPO"],
            purpose="Create workspace by cloning multiple repos in order.",
            requires="Docker daemon running; network access.",
        ),
        # list
        AwsE2ECase(
            case_id="ls",
            aws_args=["ls"],
            purpose="List workspaces.",
        ),
        # exec
        AwsE2ECase(
            case_id="exec_command",
            aws_args=["exec", "ws-e2e", "git", "-C", "REPO_PATH", "status"],
            purpose="Run a non-interactive command in the workspace container.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="exec_shell",
            aws_args=["exec", "ws-e2e"],
            purpose="Open an interactive shell in the workspace container.",
            requires="Existing workspace container ws-e2e; interactive TTY support.",
        ),
        AwsE2ECase(
            case_id="exec_root",
            aws_args=["exec", "--root", "ws-e2e", "id", "-u"],
            purpose="Exec as root and verify uid=0.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="exec_user",
            aws_args=["exec", "--user", "agent", "ws-e2e", "id", "-u"],
            purpose="Exec as a specific user and verify expected uid.",
            requires="Existing workspace container ws-e2e.",
        ),
        # auth
        AwsE2ECase(
            case_id="auth_github",
            aws_args=["auth", "github", "ws-e2e"],
            purpose="Update GitHub auth inside the workspace.",
            requires="Existing workspace container ws-e2e; valid GitHub token or `gh` login.",
        ),
        AwsE2ECase(
            case_id="auth_github_host",
            aws_args=["auth", "github", "--host", "github.com", "ws-e2e"],
            purpose="Update GitHub auth with an explicit host override.",
            requires="Existing workspace container ws-e2e; valid GitHub token or `gh` login.",
        ),
        AwsE2ECase(
            case_id="auth_agent_profile",
            aws_args=["auth", "codex", "--profile", "AGENT_PROFILE", "ws-e2e"],
            purpose="Apply Codex auth (profile-based) inside the workspace.",
            requires="Existing workspace container ws-e2e; host Codex secrets must be accessible to the launcher container.",
        ),
        AwsE2ECase(
            case_id="auth_gpg_key",
            aws_args=["auth", "gpg", "--key", "GPG_KEY_ID", "ws-e2e"],
            purpose="Import a GPG signing key into the workspace.",
            requires="Existing workspace container ws-e2e; host GPG keyring accessible; keyid exists.",
        ),
        # reset
        AwsE2ECase(
            case_id="reset_repo",
            aws_args=["reset", "repo", "ws-e2e", "REPO_PATH", "--yes"],
            purpose="Reset the primary repo inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="reset_work_repos",
            aws_args=["reset", "work-repos", "ws-e2e", "--yes"],
            purpose="Reset work repos inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="reset_opt_repos",
            aws_args=["reset", "opt-repos", "ws-e2e", "--yes"],
            purpose="Reset optional repos inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="reset_private_repo",
            aws_args=["reset", "private-repo", "ws-e2e", "--yes"],
            purpose="Reset the private repo inside the workspace.",
            requires="Existing workspace container ws-e2e.",
        ),
        # tunnel
        AwsE2ECase(
            case_id="tunnel_foreground",
            aws_args=["tunnel", "ws-e2e"],
            purpose="Start a VS Code tunnel (foreground).",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        AwsE2ECase(
            case_id="tunnel_detach",
            aws_args=["tunnel", "ws-e2e", "--detach"],
            purpose="Start a VS Code tunnel in the background.",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        AwsE2ECase(
            case_id="tunnel_named",
            aws_args=["tunnel", "ws-e2e", "--name", "ws-e2e-tunnel"],
            purpose="Start a tunnel with an explicit name.",
            requires="Existing workspace container ws-e2e; VS Code tunnel prerequisites.",
        ),
        # rm
        AwsE2ECase(
            case_id="rm_workspace_yes",
            aws_args=["rm", "ws-e2e", "--yes"],
            purpose="Remove a specific workspace without confirmation.",
            requires="Existing workspace container ws-e2e.",
        ),
        AwsE2ECase(
            case_id="rm_all_yes",
            aws_args=["rm", "--all", "--yes"],
            purpose="Remove all workspaces without confirmation.",
            requires="At least one existing workspace container.",
        ),
        # env/config
        AwsE2ECase(
            case_id="env_custom_image",
            aws_args=["ls"],
            env={"AWS_IMAGE": "graysurf/agent-workspace-launcher:latest"},
            purpose="Verify custom image selection is respected.",
        ),
        AwsE2ECase(
            case_id="env_auth_env",
            aws_args=["ls"],
            env={"AWS_AUTH": "env"},
            purpose="Force env-based auth mode (no gh keyring lookup).",
        ),
        AwsE2ECase(
            case_id="env_auth_none",
            aws_args=["ls"],
            env={"AWS_AUTH": "none"},
            purpose="Disable auth-related behavior entirely.",
        ),
    ]


def wrapper_extra_cases(wrapper: str) -> list[AwsE2ECase]:
    cases: list[AwsE2ECase] = []

    if wrapper in {"cli", "bash", "zsh"}:
        cases.append(
            AwsE2ECase(
                case_id="env_docker_args_string",
                aws_args=["ls"],
                env={"AWS_DOCKER_ARGS": "-e FOO=bar -e BAZ=qux"},
                purpose="Verify extra docker args are passed through (string form).",
            )
        )

    if wrapper in {"bash", "zsh"}:
        cases.append(
            AwsE2ECase(
                case_id="env_docker_args_array",
                aws_args=["ls"],
                prelude="AWS_DOCKER_ARGS=(-e FOO=bar -e BAZ=qux)",
                purpose="Verify extra docker args are passed through (array form).",
            )
        )

    return cases


def plan_cases(wrapper: str) -> list[AwsE2EPlanCase]:
    if wrapper not in {"cli", "bash", "zsh"}:
        raise ValueError(f"unknown wrapper: {wrapper}")

    cases = [*base_cases(), *wrapper_extra_cases(wrapper)]
    if wrapper == "cli":
        return [_build_cli(c) for c in cases if not c.prelude]
    if wrapper == "bash":
        return [_build_bash(c) for c in cases]
    return [_build_zsh(c) for c in cases]


def case_id(plan_case: AwsE2EPlanCase) -> str:
    return f"{plan_case.wrapper}:{plan_case.case.case_id}"


def run_aws_e2e(_case: AwsE2EPlanCase) -> None:
    config = _load_e2e_config()
    if not config.enabled:
        pytest.skip("AWS_E2E is not enabled (set AWS_E2E=1 to run real Docker e2e).")

    with _e2e_lock():
        plan_case = _materialize_case(_case, config)
        skip_reason = _skip_reason(plan_case, config)
        if skip_reason:
            pytest.skip(skip_reason)

        env = _build_env(plan_case.case.env, config)
        workspace = _workspace_name(plan_case.case.aws_args)
        created_workspace: str | None = None
        created_repo_path: str | None = None

        try:
            if _needs_existing_workspace(plan_case.case.aws_args) and workspace:
                if _needs_repo_workspace(plan_case.case.aws_args):
                    created_workspace, created_repo_path = _create_repo_workspace(
                        plan_case.wrapper,
                        env,
                        config,
                        include_private=_needs_private_repo(plan_case.case.aws_args),
                    )
                    plan_case = _replace_workspace_for_case(
                        plan_case, created_workspace, created_repo_path
                    )
                elif not _workspace_exists(plan_case.wrapper, env, workspace):
                    _create_named_workspace(plan_case.wrapper, env, workspace)
                    created_workspace = workspace

            accept_exit_codes = {0}
            if _is_exec_shell(plan_case.case.aws_args):
                result = _run_case_interactive(
                    plan_case, env, input_text="exit\n", send_after_sec=0.5
                )
            elif plan_case.case.case_id in {"tunnel_foreground", "tunnel_named"}:
                accept_exit_codes = {0, 130}
                result = _run_case_interactive(
                    plan_case, env, input_text="\x03", send_after_sec=3.0
                )
            else:
                result = _run_case(plan_case, env)

            _write_e2e_record(plan_case, result)

            if result.exit_code not in accept_exit_codes:
                raise AssertionError(
                    f"e2e failed: {plan_case.wrapper}:{plan_case.case.case_id} (exit {result.exit_code})"
                )

            if (
                plan_case.case.aws_args[:1] == ["rm"]
                and created_workspace
                and created_workspace == workspace
            ):
                created_workspace = None

            if _is_create_case(plan_case.case.aws_args):
                created_name = _parse_created_workspace(result.stdout)
                if created_name and not config.keep_workspaces:
                    _remove_workspace(plan_case.wrapper, env, created_name)
        finally:
            if created_workspace and not config.keep_workspaces:
                _remove_workspace(plan_case.wrapper, env, created_workspace)


def run_aws_e2e_flow(wrapper: str) -> None:
    config = _load_e2e_config()
    if not config.enabled:
        pytest.skip("AWS_E2E is not enabled (set AWS_E2E=1 to run real Docker e2e).")

    if wrapper not in {"cli", "bash", "zsh"}:
        raise ValueError(f"unknown wrapper: {wrapper}")

    with _e2e_lock():
        if not _docker_available():
            pytest.skip("Docker is not available (required for e2e).")

        env = _build_env(None, config)
        run_id = time.strftime("%Y%m%d-%H%M%S")
        named = f"ws-e2e-{wrapper}-{run_id}"

        def run(case_id: str, aws_args: list[str], purpose: str) -> _E2ERunResult:
            plan_case = _build_case(
                wrapper,
                AwsE2ECase(case_id=case_id, aws_args=aws_args, purpose=purpose),
            )
            result = _run_case(plan_case, env)
            _write_e2e_record(plan_case, result)
            return result

        def assert_ok(result: _E2ERunResult, label: str) -> None:
            if result.exit_code != 0:
                raise AssertionError(
                    f"e2e failed: {wrapper}:{label} (exit {result.exit_code})"
                )

        named_created = False
        repo_container: str | None = None

        try:
            assert_ok(run("01_help", ["--help"], "Show top-level help."), "01_help")
            assert_ok(run("02_ls", ["ls"], "List workspaces."), "02_ls")

            create_named = run(
                "10_create_named",
                ["create", "--no-work-repos", "--name", named],
                "Create a named workspace without cloning repos.",
            )
            assert_ok(create_named, "10_create_named")
            named_created = True

            exec_user = run(
                "11_exec_user",
                ["exec", "--user", "agent", named, "id", "-u"],
                "Exec as agent and verify uid.",
            )
            assert_ok(exec_user, "11_exec_user")
            assert exec_user.stdout.strip().isdigit()
            assert exec_user.stdout.strip() != "0"

            exec_root = run(
                "12_exec_root",
                ["exec", "--root", named, "id", "-u"],
                "Exec as root and verify uid.",
            )
            assert_ok(exec_root, "12_exec_root")
            assert exec_root.stdout.strip() == "0"

            if wrapper == "cli" or config.full:
                if not config.public_repo:
                    pytest.skip(
                        "AWS_E2E_PUBLIC_REPO is required for repo-backed e2e flow."
                    )

                create_repo = run(
                    "20_create_repo",
                    ["create", config.public_repo],
                    "Create a workspace from a public repo.",
                )
                assert_ok(create_repo, "20_create_repo")
                repo_container = _parse_created_workspace(create_repo.stdout)
                repo_path = _parse_created_path(create_repo.stdout)
                if not repo_container or not repo_path:
                    raise AssertionError(
                        "Failed to parse repo workspace output (workspace/path)."
                    )

                assert_ok(
                    run(
                        "21_exec_git_status",
                        ["exec", repo_container, "git", "-C", repo_path, "status"],
                        "Run git status inside the repo workspace.",
                    ),
                    "21_exec_git_status",
                )
                assert_ok(
                    run(
                        "22_reset_repo",
                        ["reset", "repo", repo_container, repo_path, "--yes"],
                        "Reset the primary repo inside the repo workspace.",
                    ),
                    "22_reset_repo",
                )
                assert_ok(
                    run(
                        "23_reset_opt_repos",
                        ["reset", "opt-repos", repo_container, "--yes"],
                        "Reset /opt repos inside the repo workspace.",
                    ),
                    "23_reset_opt_repos",
                )
        finally:
            if repo_container and not config.keep_workspaces:
                run(
                    "90_rm_repo",
                    ["rm", repo_container, "--yes"],
                    "Remove repo workspace.",
                )
            if named_created and not config.keep_workspaces:
                run("91_rm_named", ["rm", named, "--yes"], "Remove named workspace.")


@dataclass(frozen=True)
class _E2EConfig:
    enabled: bool
    public_repo: str | None
    private_repo: str | None
    agent_profile: str | None
    gpg_key_id: str | None
    allow_rm_all: bool
    enable_auth: bool
    enable_codex: bool
    enable_gpg: bool
    enable_ssh: bool
    enable_tunnel: bool
    enable_exec_shell: bool
    keep_workspaces: bool
    use_host_home: bool
    image: str | None
    full: bool


@dataclass(frozen=True)
class _E2ERunResult:
    argv: list[str]
    command_display: str
    exit_code: int
    duration_ms: int
    stdout: str
    stderr: str
    stdout_path: str
    stderr_path: str


def _env_flag(name: str) -> bool:
    value = os.environ.get(name, "")
    return value.lower() in {"1", "true", "yes", "on"}


def _load_e2e_config() -> _E2EConfig:
    return _E2EConfig(
        enabled=_env_flag("AWS_E2E"),
        public_repo=os.environ.get("AWS_E2E_PUBLIC_REPO"),
        private_repo=os.environ.get("AWS_E2E_PRIVATE_REPO"),
        agent_profile=os.environ.get("AWS_E2E_AGENT_PROFILE"),
        gpg_key_id=os.environ.get("AWS_E2E_GPG_KEY_ID"),
        allow_rm_all=_env_flag("AWS_E2E_ALLOW_RM_ALL"),
        enable_auth=_env_flag("AWS_E2E_ENABLE_AUTH"),
        enable_codex=_env_flag("AWS_E2E_ENABLE_CODEX"),
        enable_gpg=_env_flag("AWS_E2E_ENABLE_GPG"),
        enable_ssh=_env_flag("AWS_E2E_ENABLE_SSH"),
        enable_tunnel=_env_flag("AWS_E2E_ENABLE_TUNNEL"),
        enable_exec_shell=_env_flag("AWS_E2E_ENABLE_EXEC_SHELL"),
        keep_workspaces=_env_flag("AWS_E2E_KEEP_WORKSPACES"),
        use_host_home=_env_flag("AWS_E2E_USE_HOST_HOME"),
        image=os.environ.get("AWS_E2E_IMAGE"),
        full=_env_flag("AWS_E2E_FULL"),
    )


def _materialize_case(plan_case: AwsE2EPlanCase, config: _E2EConfig) -> AwsE2EPlanCase:
    case = plan_case.case
    aws_args = _replace_placeholders(case.aws_args, plan_case.wrapper, config)
    env = dict(case.env or {})
    if case.case_id == "env_custom_image" and config.image:
        env["AWS_IMAGE"] = config.image
    updated_case = AwsE2ECase(
        case_id=case.case_id,
        aws_args=aws_args,
        purpose=case.purpose,
        requires=case.requires,
        env=(env or None),
        prelude=case.prelude,
    )
    return _build_case(plan_case.wrapper, updated_case)


def _build_case(wrapper: str, case: AwsE2ECase) -> AwsE2EPlanCase:
    if wrapper == "cli":
        return _build_cli(case)
    if wrapper == "bash":
        return _build_bash(case)
    return _build_zsh(case)


def _replace_placeholders(
    args: list[str], wrapper: str, config: _E2EConfig
) -> list[str]:
    workspace = _workspace_label(wrapper, args)
    tunnel_name = f"{workspace}-tunnel"

    replaced: list[str] = []
    for arg in args:
        sentinel = "__AWS_E2E_TUNNEL__"
        updated = (
            arg.replace("ws-e2e-tunnel", sentinel)
            .replace("ws-e2e", workspace)
            .replace(sentinel, tunnel_name)
        )
        if "OWNER/REPO" in updated and config.public_repo:
            updated = updated.replace("OWNER/REPO", config.public_repo)
        if "OWNER/PRIVATE_REPO" in updated and config.private_repo:
            updated = updated.replace("OWNER/PRIVATE_REPO", config.private_repo)
        if updated == "AGENT_PROFILE" and config.agent_profile:
            updated = config.agent_profile
        if updated == "GPG_KEY_ID" and config.gpg_key_id:
            updated = config.gpg_key_id
        replaced.append(updated)
    return replaced


def _workspace_label(wrapper: str, args: list[str]) -> str:
    base = f"ws-e2e-{wrapper}"
    if _needs_private_repo(args):
        return f"{base}-private"
    if _needs_repo_workspace(args):
        return f"{base}-repo"
    return base


def _skip_reason(plan_case: AwsE2EPlanCase, config: _E2EConfig) -> str | None:
    if not _docker_available():
        return "Docker is not available (required for e2e)."

    args = plan_case.case.aws_args
    if _is_exec_shell(args) and not config.enable_exec_shell:
        return "Interactive exec shell disabled (set AWS_E2E_ENABLE_EXEC_SHELL=1)."

    if _is_auth_case(args) and not config.enable_auth:
        return "Auth cases disabled (set AWS_E2E_ENABLE_AUTH=1)."

    if _is_codex_auth_case(args) and not config.enable_codex:
        return "Codex auth disabled (set AWS_E2E_ENABLE_CODEX=1)."

    if (
        _is_codex_auth_case(args)
        and "AGENT_PROFILE" in args
        and not config.agent_profile
    ):
        return "Codex profile not configured (set AWS_E2E_AGENT_PROFILE)."

    if _is_gpg_auth_case(args) and not config.enable_gpg:
        return "GPG auth disabled (set AWS_E2E_ENABLE_GPG=1)."

    if _is_gpg_auth_case(args) and "GPG_KEY_ID" in args and not config.gpg_key_id:
        return "GPG key id not configured (set AWS_E2E_GPG_KEY_ID)."

    if _is_tunnel_case(args) and not config.enable_tunnel:
        return "Tunnel cases disabled (set AWS_E2E_ENABLE_TUNNEL=1)."

    if _is_rm_all_case(args) and not config.allow_rm_all:
        return "rm --all disabled (set AWS_E2E_ALLOW_RM_ALL=1)."

    if _is_ssh_create_case(args) and not config.enable_ssh:
        return "SSH create cases disabled (set AWS_E2E_ENABLE_SSH=1)."

    if _needs_private_repo(args) and not config.private_repo:
        return "Private repo not configured (set AWS_E2E_PRIVATE_REPO)."

    if _needs_public_repo(args) and not config.public_repo:
        return "Public repo not configured (set AWS_E2E_PUBLIC_REPO)."

    return None


def _docker_available() -> bool:
    try:
        subprocess.run(
            ["docker", "info"],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True
    except Exception:
        return False


def _needs_public_repo(args: list[str]) -> bool:
    return any("OWNER/REPO" in arg for arg in args) or _needs_repo_workspace(args)


def _needs_private_repo(args: list[str]) -> bool:
    if any("OWNER/PRIVATE_REPO" in arg for arg in args) or "--private-repo" in args:
        return True
    return args[:2] == ["reset", "private-repo"]


def _is_create_case(args: list[str]) -> bool:
    return bool(args) and args[0] == "create"


def _is_exec_shell(args: list[str]) -> bool:
    return len(args) >= 2 and args[0] == "exec" and len(args) == 2


def _is_auth_case(args: list[str]) -> bool:
    return bool(args) and args[0] == "auth"


def _is_codex_auth_case(args: list[str]) -> bool:
    return len(args) >= 3 and args[0] == "auth" and args[1] == "codex"


def _is_gpg_auth_case(args: list[str]) -> bool:
    return len(args) >= 3 and args[0] == "auth" and args[1] == "gpg"


def _is_tunnel_case(args: list[str]) -> bool:
    return bool(args) and args[0] == "tunnel"


def _is_rm_all_case(args: list[str]) -> bool:
    return bool(args) and args[0] == "rm" and "--all" in args


def _is_ssh_create_case(args: list[str]) -> bool:
    return _is_create_case(args) and any(
        arg.startswith("git@") or arg.startswith("ssh://git@") for arg in args
    )


def _needs_existing_workspace(args: list[str]) -> bool:
    if not args:
        return False
    return args[0] in {"exec", "auth", "reset", "tunnel", "rm"}


def _needs_repo_workspace(args: list[str]) -> bool:
    if not args:
        return False
    subcmd = args[0]
    if subcmd == "reset":
        return True
    if subcmd == "exec" and "git" in args:
        return True
    return False


def _workspace_name(args: list[str]) -> str | None:
    if not args:
        return None

    subcmd = args[0]
    if subcmd == "create":
        name = _arg_value(args, "--name")
        return name

    if subcmd == "exec":
        return _first_non_option(args[1:], {"--user"})

    if subcmd == "auth":
        return _first_non_option(args[2:], {"--host", "--profile", "--key"})

    if subcmd == "reset":
        return _first_non_option(args[2:], set())

    if subcmd == "tunnel":
        return _first_non_option(args[1:], {"--name"})

    if subcmd == "rm":
        if "--all" in args:
            return None
        return _first_non_option(args[1:], {"--yes"})

    return None


def _arg_value(args: list[str], key: str) -> str | None:
    if key not in args:
        return None
    idx = args.index(key)
    if idx + 1 < len(args):
        return args[idx + 1]
    return None


def _first_non_option(args: list[str], options_with_values: set[str]) -> str | None:
    i = 0
    while i < len(args):
        arg = args[i]
        if arg.startswith("-"):
            if arg in options_with_values:
                i += 2
                continue
            i += 1
            continue
        return arg
    return None


def _build_env(case_env: dict[str, str] | None, config: _E2EConfig) -> dict[str, str]:
    env = os.environ.copy()
    if not config.use_host_home:
        env.pop("AWS_DOCKER_ARGS", None)
    gh_token = env.get("AWS_E2E_GH_TOKEN", "")
    if gh_token and not env.get("GH_TOKEN") and not env.get("GITHUB_TOKEN"):
        env["GH_TOKEN"] = gh_token
    env["AGENT_HOME"] = str(repo_root())
    env["AGENT_WORKSPACE_OPEN_VSCODE_ENABLED"] = "false"
    env["NO_COLOR"] = "1"
    env["CLICOLOR"] = "0"
    env["CLICOLOR_FORCE"] = "0"
    env["FORCE_COLOR"] = "0"
    env["PY_COLORS"] = "0"
    env["GIT_PAGER"] = "cat"
    env["PAGER"] = "cat"
    if not config.enable_gpg:
        env["AGENT_WORKSPACE_GPG"] = "none"
        env["AGENT_WORKSPACE_GPG_KEY"] = ""

    if config.image:
        env["AWS_IMAGE"] = config.image

    if not config.use_host_home:
        out_base = _e2e_out_dir()
        home = out_base / "home"
        xdg_config = out_base / "xdg" / "config"
        xdg_cache = out_base / "xdg" / "cache"
        xdg_data = out_base / "xdg" / "data"
        xdg_state = out_base / "xdg" / "state"
        tmp = out_base / "tmp"

        for p in (home, xdg_config, xdg_cache, xdg_data, xdg_state, tmp):
            p.mkdir(parents=True, exist_ok=True)

        env.update(
            {
                "HOME": str(home),
                "XDG_CONFIG_HOME": str(xdg_config),
                "XDG_CACHE_HOME": str(xdg_cache),
                "XDG_DATA_HOME": str(xdg_data),
                "XDG_STATE_HOME": str(xdg_state),
                "TMPDIR": str(tmp),
            }
        )

    if case_env:
        env.update({str(k): str(v) for k, v in case_env.items()})
    return env


def _run_case(plan_case: AwsE2EPlanCase, env: dict[str, str]) -> _E2ERunResult:
    start = time.monotonic()
    input_text = "y\n" if plan_case.case.aws_args[:1] == ["reset"] else None
    completed = subprocess.run(
        plan_case.command_argv,
        cwd=repo_root(),
        env=env,
        capture_output=True,
        text=True,
        input=input_text,
    )
    duration_ms = int((time.monotonic() - start) * 1000)

    out_dir = _case_out_dir(plan_case)
    out_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = out_dir / "stdout.txt"
    stderr_path = out_dir / "stderr.txt"
    stdout_path.write_text(completed.stdout, "utf-8")
    stderr_path.write_text(completed.stderr, "utf-8")

    return _E2ERunResult(
        argv=plan_case.command_argv,
        command_display=plan_case.command_display,
        exit_code=completed.returncode,
        duration_ms=duration_ms,
        stdout=completed.stdout,
        stderr=completed.stderr,
        stdout_path=str(stdout_path),
        stderr_path=str(stderr_path),
    )


def _case_out_dir(plan_case: AwsE2EPlanCase) -> Path:
    safe_case = re.sub(r"[^a-zA-Z0-9._-]+", "_", plan_case.case.case_id)
    return _e2e_out_dir() / plan_case.wrapper / safe_case


def _e2e_out_dir() -> Path:
    base = repo_root() / "out" / "tests" / "e2e"
    base.mkdir(parents=True, exist_ok=True)
    return base


def _write_e2e_record(plan_case: AwsE2EPlanCase, result: _E2ERunResult) -> None:
    record: dict[str, object] = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "case_id": plan_case.case.case_id,
        "wrapper": plan_case.wrapper,
        "purpose": plan_case.case.purpose,
        "requires": plan_case.case.requires,
        "command_display": plan_case.command_display,
        "argv": result.argv,
        "exit_code": result.exit_code,
        "duration_ms": result.duration_ms,
        "stdout_path": result.stdout_path,
        "stderr_path": result.stderr_path,
    }

    meta_path = _case_out_dir(plan_case) / "meta.json"
    meta_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", "utf-8")
    _append_summary(record)


def _append_summary(record: dict[str, object]) -> None:
    summary_path = _e2e_out_dir() / "summary.jsonl"
    with summary_path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")


def _parse_created_workspace(stdout: str) -> str | None:
    match = re.search(r"^workspace:\s*(\S+)\s*$", stdout, re.MULTILINE)
    if match:
        return match.group(1)
    return None


def _parse_created_path(stdout: str) -> str | None:
    match = re.search(r"^path:\s*(\S+)\s*$", stdout, re.MULTILINE)
    if match:
        return match.group(1)
    return None


def _create_repo_workspace(
    wrapper: str,
    env: dict[str, str],
    config: _E2EConfig,
    *,
    include_private: bool,
) -> tuple[str, str]:
    if not config.public_repo:
        raise AssertionError(
            "AWS_E2E_PUBLIC_REPO is required for repo-backed e2e cases."
        )
    create_case = AwsE2ECase(
        case_id=f"e2e_setup_create_repo_{wrapper}",
        aws_args=(
            ["create", "--private-repo", config.private_repo, config.public_repo]
            if include_private and config.private_repo
            else ["create", config.public_repo]
        ),
        purpose="Create repo workspace for dependent e2e cases.",
    )
    plan_case = _build_case(wrapper, create_case)
    result = _run_case(plan_case, env)
    _write_e2e_record(plan_case, result)
    if result.exit_code != 0:
        raise AssertionError(
            f"Failed to create repo workspace (exit {result.exit_code})."
        )
    workspace = _parse_created_workspace(result.stdout)
    repo_path = _parse_created_path(result.stdout)
    if not workspace or not repo_path:
        raise AssertionError("Failed to parse repo workspace output (workspace/path).")
    return workspace, repo_path


def _create_named_workspace(wrapper: str, env: dict[str, str], workspace: str) -> None:
    create_case = AwsE2ECase(
        case_id=f"e2e_setup_create_{workspace}",
        aws_args=["create", "--no-work-repos", "--name", workspace],
        purpose="Create workspace for dependent e2e cases.",
    )
    plan_case = _build_case(wrapper, create_case)
    result = _run_case(plan_case, env)
    _write_e2e_record(plan_case, result)
    if result.exit_code != 0:
        raise AssertionError(
            f"Failed to create workspace {workspace} (exit {result.exit_code})."
        )


def _remove_workspace(wrapper: str, env: dict[str, str], workspace: str | None) -> None:
    if not workspace:
        return
    rm_case = AwsE2ECase(
        case_id=f"e2e_cleanup_rm_{workspace}",
        aws_args=["rm", workspace, "--yes"],
        purpose="Remove workspace created for e2e tests.",
    )
    plan_case = _build_case(wrapper, rm_case)
    result = _run_case(plan_case, env)
    _write_e2e_record(plan_case, result)


def _workspace_exists(wrapper: str, env: dict[str, str], workspace: str) -> bool:
    ls_case = AwsE2ECase(
        case_id=f"e2e_probe_ls_{workspace}",
        aws_args=["ls"],
        purpose="Check for existing workspaces.",
    )
    plan_case = _build_case(wrapper, ls_case)
    result = _run_case(plan_case, env)
    _write_e2e_record(plan_case, result)
    if result.exit_code != 0:
        return False
    expected = {
        workspace,
        f"agent-ws-{workspace}",
        f"codex-ws-{workspace}",
    }
    for line in result.stdout.splitlines():
        name = line.split("\t", 1)[0].strip()
        if name in expected:
            return True
    return False


def _replace_workspace_for_case(
    plan_case: AwsE2EPlanCase,
    workspace: str,
    repo_path: str | None,
) -> AwsE2EPlanCase:
    args = list(plan_case.case.aws_args)
    current = _workspace_name(args)
    if current:
        args = [workspace if arg == current else arg for arg in args]

    args = _replace_repo_path(args, repo_path)
    args = _ensure_reset_args(args, repo_path)
    updated_case = AwsE2ECase(
        case_id=plan_case.case.case_id,
        aws_args=args,
        purpose=plan_case.case.purpose,
        requires=plan_case.case.requires,
        env=plan_case.case.env,
        prelude=plan_case.case.prelude,
    )
    return _build_case(plan_case.wrapper, updated_case)


def _replace_repo_path(args: list[str], repo_path: str | None) -> list[str]:
    if not repo_path:
        return args
    return [repo_path if arg == "REPO_PATH" else arg for arg in args]


def _ensure_reset_args(args: list[str], repo_path: str | None) -> list[str]:
    if not args or args[0] != "reset":
        return args
    if len(args) >= 2 and args[1] == "repo" and repo_path:
        if len(args) < 4:
            args = [*args[:3], repo_path, *args[3:]]
    if "--yes" not in args:
        args = [*args, "--yes"]
    return args


@contextlib.contextmanager
def _e2e_lock() -> Iterator[None]:
    if fcntl is None:
        yield
        return

    lock_path = _e2e_out_dir() / ".lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("w", encoding="utf-8") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def _run_case_interactive(
    plan_case: AwsE2EPlanCase,
    env: dict[str, str],
    *,
    input_text: str,
    send_after_sec: float = 0.5,
) -> _E2ERunResult:
    import os
    import pty
    import select

    start = time.monotonic()

    master_fd, slave_fd = pty.openpty()
    output: bytes = b""
    try:
        proc = subprocess.Popen(
            plan_case.command_argv,
            cwd=repo_root(),
            env=env,
            stdin=slave_fd,
            stdout=slave_fd,
            stderr=slave_fd,
            text=False,
        )
        os.close(slave_fd)

        sent = False
        deadline = start + 15.0
        while True:
            if not sent and time.monotonic() - start > send_after_sec:
                os.write(master_fd, input_text.encode("utf-8"))
                sent = True

            rlist, _, _ = select.select([master_fd], [], [], 0.2)
            if rlist:
                try:
                    chunk = os.read(master_fd, 4096)
                except OSError:
                    chunk = b""
                if chunk:
                    output += chunk

            if proc.poll() is not None:
                break
            if time.monotonic() > deadline:
                proc.terminate()
                break

        try:
            exit_code = proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            exit_code = proc.wait(timeout=5)
    finally:
        try:
            os.close(master_fd)
        except OSError:
            pass

    duration_ms = int((time.monotonic() - start) * 1000)
    stdout = output.decode("utf-8", errors="replace")
    stderr = ""

    out_dir = _case_out_dir(plan_case)
    out_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = out_dir / "stdout.txt"
    stderr_path = out_dir / "stderr.txt"
    stdout_path.write_text(stdout, "utf-8")
    stderr_path.write_text(stderr, "utf-8")

    return _E2ERunResult(
        argv=plan_case.command_argv,
        command_display=plan_case.command_display,
        exit_code=exit_code,
        duration_ms=duration_ms,
        stdout=stdout,
        stderr=stderr,
        stdout_path=str(stdout_path),
        stderr_path=str(stderr_path),
    )
