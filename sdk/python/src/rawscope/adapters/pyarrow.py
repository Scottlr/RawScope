"""Lazy PyArrow Table and RecordBatch adapter."""

from __future__ import annotations

from pathlib import Path

from ..models import DataframeSchemaError, MissingOptionalDependency
from .common import validate_column_names, validate_flat_arrow_schema


class PyArrowAdapter:
    def __init__(self, source: object) -> None:
        try:
            import pyarrow as pa
        except ImportError as error:
            raise MissingOptionalDependency(
                "PyArrow support requires the rawscope-sdk[arrow] extra"
            ) from error
        if isinstance(source, pa.RecordBatch):
            self.table = pa.Table.from_batches([source])
        elif isinstance(source, pa.Table):
            self.table = source
        else:
            raise DataframeSchemaError("PyArrow adapter received an unsupported object")
        self._column_names = tuple(self.table.schema.names)
        validate_column_names(self._column_names)
        validate_flat_arrow_schema(self.table.schema)

    def column_names(self) -> tuple[str, ...]:
        return self._column_names

    def row_count(self) -> int:
        if self.table.num_rows == 0:
            raise DataframeSchemaError("dataframe contains no rows")
        return self.table.num_rows

    def validate_evidence_key(self, column: str) -> None:
        if column not in self._column_names:
            raise DataframeSchemaError(
                f"evidence key column '{column}' is missing from the dataframe"
            )
        values = self.table.column(column).to_pylist()
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
            import pyarrow.parquet as parquet

            parquet.write_table(self.table, path)
        except Exception as error:
            raise DataframeSchemaError(
                f"PyArrow table could not be materialized as Parquet: {error}"
            ) from error
