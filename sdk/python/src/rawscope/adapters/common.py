"""Flat Arrow schema checks shared by dataframe adapters."""

from __future__ import annotations

from typing import Any

from ..models import DataframeSchemaError


def validate_column_names(names: tuple[str, ...]) -> None:
    if any(not isinstance(name, str) or not name.strip() for name in names):
        raise DataframeSchemaError("all dataframe column names must be non-blank strings")
    if len(names) != len(set(names)):
        raise DataframeSchemaError("dataframe column names must be unique")


def validate_flat_arrow_schema(schema: Any) -> None:
    try:
        import pyarrow.types as types
    except ImportError as error:
        raise DataframeSchemaError(
            "PyArrow is required to validate the generated Parquet schema"
        ) from error

    for field in schema:
        data_type = field.type
        is_supported = (
            types.is_null(data_type)
            or types.is_integer(data_type)
            or types.is_floating(data_type)
            or types.is_string(data_type)
            or types.is_large_string(data_type)
        )
        if not is_supported:
            raise DataframeSchemaError(
                f"column '{field.name}' has unsupported Arrow type '{data_type}'; "
                "RawScope accepts flat integer, float, string, or null columns"
            )


def validate_required_columns(
    names: tuple[str, ...],
    required: tuple[str, ...],
) -> None:
    missing = [name for name in required if name not in names]
    if missing:
        raise DataframeSchemaError(
            f"dataframe is missing required columns: {', '.join(missing)}; "
            f"available columns: {', '.join(names)}"
        )
