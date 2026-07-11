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
    TimelineView,
    require_text,
)

ARTIFACT_KIND = "rawscope.session"
SCHEMA_VERSION = 1
MANIFEST_NAME = "analysis.rawscope.json"


def prepare(
    source: str | os.PathLike[str],
    *,
    view: ScatterView | TimelineView,
    destination: str | os.PathLike[str],
    display_name: str | None = None,
    evidence_key: str | None = None,
    limit: int | None = None,
) -> PreparedSession:
    """Write a local session manifest referencing an existing CSV or Parquet file."""

    dataset = DatasetSource.from_path(source)
    if not isinstance(view, (ScatterView, TimelineView)):
        raise InvalidSession("view must be ScatterView or TimelineView")
    if display_name is not None:
        require_text(display_name, "dataset.display_name")
    if evidence_key is not None:
        require_text(evidence_key, "dataset.evidence_key")
    if limit is not None and (not isinstance(limit, int) or limit <= 0):
        raise InvalidSession("dataset.limit must be a positive integer")

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
    )
    _atomic_write_json(manifest_path, payload)
    return PreparedSession(
        bundle_dir=bundle_dir.resolve(),
        manifest_path=manifest_path.resolve(),
        dataset_path=dataset.path,
        persistent=True,
    )


def view(
    source: str | os.PathLike[str],
    *,
    view: ScatterView | TimelineView,
    destination: str | os.PathLike[str],
    executable: str | os.PathLike[str] | None = None,
    display_name: str | None = None,
    evidence_key: str | None = None,
    limit: int | None = None,
):
    """Prepare a file-backed session and launch the native workbench."""

    from .launcher import launch

    session = prepare(
        source,
        view=view,
        destination=destination,
        display_name=display_name,
        evidence_key=evidence_key,
        limit=limit,
    )
    return launch(session, executable=executable)


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
    view: ScatterView | TimelineView,
    display_name: str | None,
    evidence_key: str | None,
    limit: int | None,
) -> dict[str, object]:
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
            "kind": "scatter",
            "x": view.x,
            "y": view.y,
        }
    else:
        view_payload = {
            "kind": "timeline",
            "time": view.time,
            "lane": view.lane,
        }
    if view.profile is not None:
        view_payload["profile"] = view.profile

    return {
        "artifact_kind": ARTIFACT_KIND,
        "schema_version": SCHEMA_VERSION,
        "dataset": dataset,
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
