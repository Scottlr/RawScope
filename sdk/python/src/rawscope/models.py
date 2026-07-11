"""Typed, dependency-free values shared by the RawScope Python bridge."""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path


class RawScopeError(Exception):
    """Base error for local RawScope SDK operations."""


class InvalidSession(RawScopeError):
    """Raised when a session value cannot be represented by schema v1."""


def require_text(value: str, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise InvalidSession(f"{field} must be a non-blank string")
    return value


@dataclass(frozen=True, slots=True)
class ScatterView:
    """Scatter binding for two numeric source columns."""

    x: str
    y: str
    profile: str | None = None

    def __post_init__(self) -> None:
        require_text(self.x, "view.x")
        require_text(self.y, "view.y")
        if self.x == self.y:
            raise InvalidSession("view.x and view.y must name different columns")
        if self.profile is not None:
            require_text(self.profile, "view.profile")


@dataclass(frozen=True, slots=True)
class TimelineView:
    """Timeline binding for a time column and lane column."""

    time: str
    lane: str
    profile: str | None = None

    def __post_init__(self) -> None:
        require_text(self.time, "view.time")
        require_text(self.lane, "view.lane")
        if self.time == self.lane:
            raise InvalidSession("view.time and view.lane must name different columns")
        if self.profile is not None:
            require_text(self.profile, "view.profile")


@dataclass(frozen=True, slots=True)
class DatasetSource:
    """Resolved local path and v1 format selected from its extension."""

    path: Path
    format: str

    @classmethod
    def from_path(cls, source: str | os.PathLike[str]) -> DatasetSource:
        raw_path = os.fspath(source)
        if "://" in raw_path:
            raise InvalidSession("dataset paths must be local files, not URIs")
        path = Path(raw_path).expanduser()
        if not path.is_file():
            raise InvalidSession(f"dataset path is not a regular file: {path}")
        extension = path.suffix.lower().lstrip(".")
        if extension not in {"csv", "parquet"}:
            raise InvalidSession(
                f"dataset must use a .csv or .parquet extension: {path}"
            )
        return cls(path.resolve(), extension)


@dataclass(frozen=True, slots=True)
class PreparedSession:
    """Manifest and data paths ready to pass to the native workbench."""

    bundle_dir: Path
    manifest_path: Path
    dataset_path: Path
    persistent: bool = True
