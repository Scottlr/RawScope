"""Atomic writer for the RawScope session schema v1."""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

from .models import (
    DatasetSource,
    InvalidSession,
    PreparedSession,
    RawScopeError,
    ScatterView,
    TimeValueView,
    TimelineView,
    require_text,
    validate_row_limit,
)

ARTIFACT_KIND = "rawscope.session"
SCHEMA_VERSION = 1
SCHEMA_VERSION_V2 = 2
MANIFEST_NAME = "analysis.rawscope.json"


def prepare(
    source: object,
    *,
    view: ScatterView | TimelineView | TimeValueView,
    destination: str | os.PathLike[str],
    display_name: str | None = None,
    evidence_key: str | None = None,
    limit: int | None = None,
    schema_version: int | None = None,
) -> PreparedSession:
    """Prepare either a file-backed or dataframe-backed local session."""

    if not _is_file_source(source):
        from .bundle import prepare_dataframe

        return prepare_dataframe(
            source,
            view=view,
            destination=destination,
            display_name=display_name,
            evidence_key=evidence_key,
            limit=limit,
            schema_version=schema_version,
        )

    dataset = source if isinstance(source, DatasetSource) else DatasetSource.from_path(source)
    if not isinstance(view, (ScatterView, TimelineView, TimeValueView)):
        raise InvalidSession("view must be ScatterView, TimelineView, or TimeValueView")
    if display_name is not None:
        require_text(display_name, "dataset.display_name")
    if evidence_key is not None:
        require_text(evidence_key, "dataset.evidence_key")
    limit = validate_row_limit(limit)

    bundle_dir, manifest_path = _destination_paths(destination)
    bundle_dir.mkdir(parents=True, exist_ok=True)
    dataset_path = _manifest_dataset_path(dataset.path, bundle_dir)
    payload = _manifest_payload(
        dataset_path=dataset_path,
        data_format=dataset.format,
        view=view,
        display_name=display_name,
        evidence_key=evidence_key,
        limit=limit,
        schema_version=schema_version,
    )
    _atomic_write_json(manifest_path, payload)
    return PreparedSession(
        bundle_dir=bundle_dir.resolve(),
        manifest_path=manifest_path.resolve(),
        dataset_path=dataset.path,
        persistent=True,
    )


def view(
    source: object,
    *,
    view: ScatterView | TimelineView | TimeValueView,
    destination: str | os.PathLike[str] | None = None,
    executable: str | os.PathLike[str] | None = None,
    display_name: str | None = None,
    evidence_key: str | None = None,
    limit: int | None = None,
    schema_version: int | None = None,
):
    """Prepare a file or dataframe-backed session and launch the native workbench."""

    from .launcher import launch

    if _is_file_source(source):
        if destination is None:
            raise InvalidSession("destination is required when source is an existing file")
        session = prepare(
            source,
            view=view,
            destination=destination,
            display_name=display_name,
            evidence_key=evidence_key,
            limit=limit,
            schema_version=schema_version,
        )
    else:
        from .bundle import prepare_dataframe

        session = prepare_dataframe(
            source,
            view=view,
            destination=destination,
            display_name=display_name,
            evidence_key=evidence_key,
            limit=limit,
            schema_version=schema_version,
        )
    return launch(session, executable=executable)


def _is_file_source(source: object) -> bool:
    return isinstance(source, (str, os.PathLike, DatasetSource))


def _destination_paths(
    destination: str | os.PathLike[str],
) -> tuple[Path, Path]:
    path = Path(destination).expanduser()
    if path.name.lower().endswith(".rawscope.json"):
        return path.parent or Path("."), path
    return path, path / MANIFEST_NAME


def _manifest_dataset_path(dataset_path: Path, bundle_dir: Path) -> str:
    resolved_bundle = bundle_dir.resolve()
    try:
        common_path = Path(os.path.commonpath([dataset_path, resolved_bundle]))
    except ValueError:
        common_path = Path()
    if common_path == resolved_bundle:
        return Path(os.path.relpath(dataset_path, resolved_bundle)).as_posix()
    return str(dataset_path)


def _manifest_payload(
    *,
    dataset_path: str,
    data_format: str,
    view: ScatterView | TimelineView | TimeValueView,
    display_name: str | None,
    evidence_key: str | None,
    limit: int | None,
    schema_version: int | None,
) -> dict[str, object]:
    if schema_version not in (None, SCHEMA_VERSION, SCHEMA_VERSION_V2):
        raise InvalidSession("schema_version must be 1 or 2")
    requires_v2 = isinstance(view, TimeValueView) or (
        isinstance(view, ScatterView) and view.category is not None
    )
    if schema_version == SCHEMA_VERSION and requires_v2:
        raise InvalidSession("the selected view requires session schema v2")
    use_v2 = schema_version == SCHEMA_VERSION_V2 or requires_v2

    dataset: dict[str, object] = {
        "path": dataset_path,
        "format": data_format,
    }
    if display_name is not None:
        dataset["display_name"] = display_name
    if limit is not None:
        dataset["limit"] = limit
    if evidence_key is not None:
        dataset["evidence_key"] = evidence_key

    if isinstance(view, ScatterView):
        view_payload: dict[str, object] = {
            "kind": "numeric_pair" if use_v2 else "scatter",
            "x": view.x,
            "y": view.y,
        }
        if view.category is not None:
            view_payload["category"] = view.category
    elif isinstance(view, TimeValueView):
        view_payload = {
            "kind": "time_value",
            "time": view.time,
            "value": view.value,
        }
        if view.category is not None:
            view_payload["category"] = view.category
    else:
        view_payload = {
            "kind": "timeline_lane" if use_v2 else "timeline",
            "time": view.time,
            "lane": view.lane,
        }
    if view.profile is not None:
        view_payload["profile"] = view.profile

    return {
        "artifact_kind": ARTIFACT_KIND,
        "schema_version": SCHEMA_VERSION_V2 if use_v2 else SCHEMA_VERSION,
        "source" if use_v2 else "dataset": dataset,
        "view": view_payload,
    }


def _atomic_write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            temporary_path = Path(temporary.name)
            json.dump(payload, temporary, ensure_ascii=True, indent=2)
            temporary.write("\n")
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
    except OSError as error:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
        raise RawScopeError(f"failed to write session manifest '{path}': {error}") from error
