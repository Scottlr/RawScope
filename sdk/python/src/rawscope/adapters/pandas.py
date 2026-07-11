"""Lazy pandas DataFrame adapter."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from ..models import DataframeSchemaError, MissingOptionalDependency
from .common import validate_column_names, validate_flat_arrow_schema


class PandasAdapter:
    def __init__(self, dataframe: object) -> None:
        try:
            import pandas as pd
        except ImportError as error:
            raise MissingOptionalDependency(
                "pandas support requires the rawscope-sdk[pandas] extra"
            ) from error
        if not isinstance(dataframe, pd.DataFrame):
            raise DataframeSchemaError("pandas adapter received a non-DataFrame object")
        self.dataframe = dataframe
        self._column_names = tuple(dataframe.columns.tolist())
        validate_column_names(self._column_names)

    def column_names(self) -> tuple[str, ...]:
        return self._column_names

    def row_count(self) -> int:
        count = len(self.dataframe)
        if count == 0:
            raise DataframeSchemaError("dataframe contains no rows")
        return count

    def validate_evidence_key(self, column: str) -> None:
        if column not in self._column_names:
            raise DataframeSchemaError(
                f"evidence key column '{column}' is missing from the dataframe"
            )
        values = self.dataframe[column].tolist()
        first_row_by_value: dict[str, int] = {}
        for row_index, value in enumerate(values):
            if self._is_missing(value) or not str(value).strip():
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
            import pyarrow as pa
            import pyarrow.parquet as parquet
        except ImportError as error:
            raise MissingOptionalDependency(
                "pandas Parquet support requires the rawscope-sdk[pandas] extra"
            ) from error
        try:
            table = pa.Table.from_pandas(self.dataframe, preserve_index=False)
            validate_flat_arrow_schema(table.schema)
            parquet.write_table(table, path)
        except DataframeSchemaError:
            raise
        except Exception as error:
            raise DataframeSchemaError(
                f"pandas dataframe could not be materialized as flat Parquet: {error}"
            ) from error

    @staticmethod
    def _is_missing(value: Any) -> bool:
        if value is None:
            return True
        try:
            import pandas as pd

            result = pd.isna(value)
            return bool(result) if not hasattr(result, "__len__") else False
        except (TypeError, ValueError):
            return False
