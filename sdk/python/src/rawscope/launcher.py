"""Safe local process launching for prepared RawScope sessions."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from .models import PreparedSession, RawScopeError

WORKBENCH_ENVIRONMENT_VARIABLE = "RAWSCOPE_WORKBENCH"
WORKBENCH_COMMAND = "rawscope-workbench"


class RawScopeExecutableNotFound(RawScopeError):
    """Raised when the native workbench cannot be resolved."""


def resolve_workbench_executable(
    explicit: str | os.PathLike[str] | None = None,
) -> str:
    """Resolve explicit path, environment override, then PATH command."""

    if explicit is not None:
        resolved = _find_executable(explicit)
        if resolved is None:
            raise RawScopeExecutableNotFound(
                f"RawScope workbench executable '{os.fspath(explicit)}' was not found"
            )
        return resolved

    environment_value = os.environ.get(WORKBENCH_ENVIRONMENT_VARIABLE)
    if environment_value:
        resolved = _find_executable(environment_value)
        if resolved is not None:
            return resolved

    resolved = shutil.which(WORKBENCH_COMMAND)
    if resolved is not None:
        return resolved
    raise RawScopeExecutableNotFound(
        "RawScope workbench was not found; pass executable=..., set "
        f"{WORKBENCH_ENVIRONMENT_VARIABLE}, or put {WORKBENCH_COMMAND} on PATH"
    )


def launch(
    session: PreparedSession | str | os.PathLike[str],
    *,
    executable: str | os.PathLike[str] | None = None,
) -> RawScopeProcess:
    """Launch the native workbench with a distinct `--session` argument."""

    prepared = _coerce_session(session)
    if not prepared.manifest_path.is_file():
        raise RawScopeError(f"session manifest is not a regular file: {prepared.manifest_path}")
    command = [
        resolve_workbench_executable(executable),
        "--session",
        str(prepared.manifest_path),
    ]
    process = subprocess.Popen(command, shell=False)
    return RawScopeProcess(process, prepared)


class RawScopeProcess:
    """Small lifecycle wrapper around the local native workbench process."""

    def __init__(self, process: subprocess.Popen[bytes], session: PreparedSession) -> None:
        self._process = process
        self.session = session

    @property
    def pid(self) -> int:
        return self._process.pid

    def poll(self) -> int | None:
        return self._process.poll()

    def wait(self, timeout: float | None = None) -> int:
        return_code = self._process.wait(timeout=timeout)
        self._cleanup_temporary_bundle()
        return return_code

    def terminate(self) -> int:
        if self._process.poll() is None:
            self._process.terminate()
        try:
            return self.wait(timeout=5.0)
        except subprocess.TimeoutExpired:
            self._process.kill()
            return self.wait(timeout=5.0)

    def __del__(self) -> None:
        try:
            if self._process.poll() is not None:
                self._cleanup_temporary_bundle()
        except Exception:
            pass

    def _cleanup_temporary_bundle(self) -> None:
        if self.session.persistent:
            return
        # T005 creates temporary bundles under this explicit prefix. T004 never
        # deletes a caller-provided destination.
        if not self.session.bundle_dir.name.startswith("rawscope-session-"):
            return
        if self.session.bundle_dir.is_dir():
            shutil.rmtree(self.session.bundle_dir)


def _coerce_session(
    session: PreparedSession | str | os.PathLike[str],
) -> PreparedSession:
    if isinstance(session, PreparedSession):
        return session
    manifest_path = Path(session).expanduser().resolve()
    return PreparedSession(
        bundle_dir=manifest_path.parent,
        manifest_path=manifest_path,
        dataset_path=manifest_path,
        persistent=True,
    )


def _find_executable(candidate: str | os.PathLike[str]) -> str | None:
    value = os.fspath(candidate)
    path = Path(value).expanduser()
    if path.is_file():
        return str(path.resolve())
    return shutil.which(value)
