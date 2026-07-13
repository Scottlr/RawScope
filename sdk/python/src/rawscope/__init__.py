"""Small local bridge for preparing and opening RawScope sessions."""

from .launcher import (
    RawScopeExecutableNotFound,
    RawScopeProcess,
    launch,
    resolve_workbench_executable,
)
from .manifest import prepare, view
from .models import (
    DataframeSchemaError,
    DatasetSource,
    InvalidSession,
    MissingOptionalDependency,
    PreparedSession,
    RawScopeError,
    ScatterView,
    TimeValueView,
    TimelineView,
    UnsupportedDataSource,
)

__all__ = [
    "DatasetSource",
    "DataframeSchemaError",
    "InvalidSession",
    "MissingOptionalDependency",
    "PreparedSession",
    "RawScopeError",
    "RawScopeExecutableNotFound",
    "RawScopeProcess",
    "ScatterView",
    "TimeValueView",
    "TimelineView",
    "UnsupportedDataSource",
    "launch",
    "prepare",
    "resolve_workbench_executable",
    "view",
]
