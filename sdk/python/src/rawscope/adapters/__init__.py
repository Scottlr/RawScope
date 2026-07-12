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
    try:
        from pandas import DataFrame as PandasDataFrame
    except ImportError:
        PandasDataFrame = ()
    if isinstance(source, PandasDataFrame):
        from .pandas import PandasAdapter

        return PandasAdapter(source)

    try:
        from polars import DataFrame as PolarsDataFrame
        from polars import LazyFrame as PolarsLazyFrame
    except ImportError:
        PolarsDataFrame = PolarsLazyFrame = ()
    if isinstance(source, (PolarsDataFrame, PolarsLazyFrame)):
        from .polars import PolarsAdapter

        return PolarsAdapter(source)

    try:
        from pyarrow import RecordBatch as PyArrowRecordBatch
        from pyarrow import Table as PyArrowTable
    except ImportError:
        PyArrowRecordBatch = PyArrowTable = ()
    if isinstance(source, (PyArrowTable, PyArrowRecordBatch)):
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
