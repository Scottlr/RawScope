"""Lazy dataframe adapter selection for the local Parquet bridge."""

from __future__ import annotations

from pathlib import Path
from typing import Protocol

from ..models import (
    DataframeSchemaError,
    MissingOptionalDependency,
    UnsupportedDataSource,
)


class DataframeAdapter(Protocol):
    def column_names(self) -> tuple[str, ...]: ...

    def row_count(self) -> int: ...

    def validate_evidence_key(self, column: str) -> None: ...

    def write_parquet(self, path: Path) -> None: ...


def select_adapter(source: object) -> DataframeAdapter:
    module_name = type(source).__module__
    if module_name.startswith("pandas"):
        from .pandas import PandasAdapter

        return PandasAdapter(source)
    if module_name.startswith("polars"):
        from .polars import PolarsAdapter

        return PolarsAdapter(source)
    if module_name.startswith("pyarrow"):
        from .pyarrow import PyArrowAdapter

        return PyArrowAdapter(source)
    raise UnsupportedDataSource(
        "unsupported dataframe source; accepted sources are local file paths, "
        "pandas DataFrame, Polars DataFrame/LazyFrame, and PyArrow Table/RecordBatch"
    )


__all__ = [
    "DataframeAdapter",
    "DataframeSchemaError",
    "MissingOptionalDependency",
    "UnsupportedDataSource",
    "select_adapter",
]
