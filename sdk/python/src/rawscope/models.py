"""Typed, dependency-free values shared by the RawScope Python bridge."""

from __future__ import annotations

import os
import numbers
from dataclasses import dataclass
from pathlib import Path


class RawScopeError(Exception):
    """Base error for local RawScope SDK operations."""


class InvalidSession(RawScopeError):
    """Raised when a session value cannot be represented by a session schema."""


class UnsupportedDataSource(RawScopeError):
    """Raised when an object has no supported dataframe adapter."""


class MissingOptionalDependency(RawScopeError):
    """Raised when a selected dataframe ecosystem is not installed."""


class DataframeSchemaError(RawScopeError):
    """Raised when a dataframe cannot map to the flat RawScope table contract."""


MAX_SESSION_ROW_LIMIT = 10_000_000_000


def validate_row_limit(value: int | None) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, numbers.Integral):
        raise InvalidSession("dataset.limit must be an integer, not bool")
    normalized = int(value)
    if not 1 <= normalized <= MAX_SESSION_ROW_LIMIT:
        raise InvalidSession(
            f"dataset.limit must be between 1 and {MAX_SESSION_ROW_LIMIT}"
        )
    return normalized


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
    category: str | None = None

    def __post_init__(self) -> None:
        require_text(self.x, "view.x")
        require_text(self.y, "view.y")
        if self.x == self.y:
            raise InvalidSession("view.x and view.y must name different columns")
        if self.profile is not None:
            require_text(self.profile, "view.profile")
        if self.category is not None:
            require_text(self.category, "view.category")
            if self.category in {self.x, self.y}:
                raise InvalidSession("view.category must differ from both axes")


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
class TimeValueView:
    """Generic time/value binding using a timestamp and numeric value column."""

    time: str
    value: str
    profile: str | None = None
    category: str | None = None

    def __post_init__(self) -> None:
        require_text(self.time, "view.time")
        require_text(self.value, "view.value")
        if self.time == self.value:
            raise InvalidSession("view.time and view.value must name different columns")
        if self.profile is not None:
            require_text(self.profile, "view.profile")
        if self.category is not None:
            require_text(self.category, "view.category")
            if self.category in {self.time, self.value}:
                raise InvalidSession("view.category must differ from time and value")


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
