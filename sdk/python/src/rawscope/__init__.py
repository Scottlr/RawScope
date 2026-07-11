"""Small local bridge for preparing and opening RawScope sessions."""

from .launcher import (
    RawScopeExecutableNotFound,
    RawScopeProcess,
    launch,
    resolve_workbench_executable,
)
from .manifest import prepare, view
from .models import (
    DatasetSource,
    InvalidSession,
    PreparedSession,
    RawScopeError,
    ScatterView,
    TimelineView,
)

__all__ = [
    "DatasetSource",
    "InvalidSession",
    "PreparedSession",
    "RawScopeError",
    "RawScopeExecutableNotFound",
    "RawScopeProcess",
    "ScatterView",
    "TimelineView",
    "launch",
    "prepare",
    "resolve_workbench_executable",
    "view",
]
