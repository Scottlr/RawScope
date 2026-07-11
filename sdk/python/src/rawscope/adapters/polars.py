"""Lazy Polars DataFrame and LazyFrame adapter."""

from __future__ import annotations

from pathlib import Path

from ..models import DataframeSchemaError, MissingOptionalDependency
from .common import validate_column_names

SUPPORTED_POLARS_DTYPES = {
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "Float32",
    "Float64",
    "String",
    "Utf8",
    "Null",
}


class PolarsAdapter:
    def __init__(self, dataframe: object) -> None:
        try:
            import polars as pl
        except ImportError as error:
            raise MissingOptionalDependency(
                "Polars support requires the rawscope-sdk[polars] extra"
            ) from error
        self._pl = pl
        self._lazy = isinstance(dataframe, pl.LazyFrame)
        if not self._lazy and not isinstance(dataframe, pl.DataFrame):
            raise DataframeSchemaError("Polars adapter received an unsupported object")
        self._frame = dataframe
        schema = dataframe.collect_schema() if self._lazy else dataframe.schema
        self._column_names = tuple(schema.names())
        validate_column_names(self._column_names)
        self._dtypes = tuple(schema.values())
        self._collected = None
        self._validate_dtypes()

    def column_names(self) -> tuple[str, ...]:
        return self._column_names

    def row_count(self) -> int:
        frame = self._materialized_frame()
        if frame.height == 0:
            raise DataframeSchemaError("dataframe contains no rows")
        return frame.height

    def validate_evidence_key(self, column: str) -> None:
        if column not in self._column_names:
            raise DataframeSchemaError(
                f"evidence key column '{column}' is missing from the dataframe"
            )
        values = self._materialized_frame().get_column(column).to_list()
        first_row_by_value: dict[str, int] = {}
        for row_index, value in enumerate(values):
            if value is None or not str(value).strip():
                raise DataframeSchemaError(
                    f"evidence key column '{column}' is blank at row {row_index}"
                )
            normalized = str(value).strip()
            if normalized in first_row_by_value:
                raise DataframeSchemaError(
                    f"evidence key column '{column}' duplicates value '{normalized}' "
                    f"at rows {first_row_by_value[normalized]} and {row_index}"
                )
            first_row_by_value[normalized] = row_index

    def write_parquet(self, path: Path) -> None:
        try:
            self._materialized_frame().write_parquet(path)
        except Exception as error:
            raise DataframeSchemaError(
                f"Polars dataframe could not be materialized as flat Parquet: {error}"
            ) from error

    def _materialized_frame(self):
        if self._lazy and self._collected is None:
            self._collected = self._frame.collect()
        return self._collected if self._lazy else self._frame

    def _validate_dtypes(self) -> None:
        for name, dtype in zip(self._column_names, self._dtypes):
            if str(dtype) not in SUPPORTED_POLARS_DTYPES:
                raise DataframeSchemaError(
                    f"column '{name}' has unsupported Polars type '{dtype}'; "
                    "RawScope accepts flat integer, float, string, or null columns"
                )
