"""Tests for the Python-to-Rust BIL bridge."""

import importlib
from pathlib import Path
from types import SimpleNamespace

from axle.cli.main import bil_repo_root, resolve_bil_command, run_bil_subcommand

main_module = importlib.import_module("axle.cli.main")


def test_resolve_bil_command_prefers_env_binary(monkeypatch) -> None:
    monkeypatch.setenv("BIL_CLI_BIN", "/tmp/custom-bil")

    command, cwd = resolve_bil_command()

    assert command == ["/tmp/custom-bil"]
    assert cwd is None


def test_resolve_bil_command_falls_back_to_cargo_run(monkeypatch) -> None:
    monkeypatch.delenv("BIL_CLI_BIN", raising=False)

    command, cwd = resolve_bil_command()

    assert command == ["cargo", "run", "-p", "bil-cli", "--"]
    assert cwd == bil_repo_root()
    assert cwd == Path(__file__).resolve().parents[1]


def test_run_bil_subcommand_passes_arguments_through(monkeypatch) -> None:
    calls: dict[str, object] = {}

    def fake_run(cmd: list[str], cwd: Path | None = None, check: bool = False) -> SimpleNamespace:
        calls["cmd"] = cmd
        calls["cwd"] = cwd
        calls["check"] = check
        return SimpleNamespace(returncode=0)

    monkeypatch.setenv("BIL_CLI_BIN", "/tmp/custom-bil")
    monkeypatch.setattr(main_module.subprocess, "run", fake_run)

    exit_code = run_bil_subcommand(["status", "--verbose"])

    assert exit_code == 0
    assert calls["cmd"] == ["/tmp/custom-bil", "status", "--verbose"]
    assert calls["cwd"] is None
    assert calls["check"] is False


def test_run_bil_subcommand_returns_subprocess_exit_code(monkeypatch) -> None:
    calls: dict[str, object] = {}

    def fake_run(cmd: list[str], cwd: Path | None = None, check: bool = False) -> SimpleNamespace:
        calls["cmd"] = cmd
        calls["cwd"] = cwd
        calls["check"] = check
        return SimpleNamespace(returncode=17)

    monkeypatch.delenv("BIL_CLI_BIN", raising=False)
    monkeypatch.setattr(main_module.subprocess, "run", fake_run)

    exit_code = run_bil_subcommand(["axle", "check", "proof.lean"])

    assert exit_code == 17
    assert calls["cmd"] == [
        "cargo",
        "run",
        "-p",
        "bil-cli",
        "--",
        "axle",
        "check",
        "proof.lean",
    ]
    assert calls["cwd"] == bil_repo_root()
    assert calls["check"] is False
