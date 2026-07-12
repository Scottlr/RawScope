"""Dataframe validation and Parquet-backed session bundle creation."""

from __future__ import annotations

import os
import shutil
import tempfile
import uuid
from pathlib import Path

from .adapters import DataframeAdapter, select_adapter
from .adapters.common import validate_required_columns
from .manifest import (
    _atomic_write_json,
    _destination_paths,
    _manifest_payload,
)
from .models import (
    DataframeSchemaError,
    InvalidSession,
    PreparedSession,
    ScatterView,
    TimelineView,
    require_text,
    validate_row_limit,
)


def prepare_dataframe(
    source: object,
    *,
    view: ScatterView | TimelineView,
    destination: str | os.PathLike[str] | None,
    display_name: str | None,
    evidence_key: str | None,
    limit: int | None,
) -> PreparedSession:
    adapter = select_adapter(source)
    _validate_dataframe(adapter, view, display_name, evidence_key, limit)
    temporary_bundle = destination is None
    staged_bundle = False
    if temporary_bundle:
        bundle_dir = Path(tempfile.mkdtemp(prefix="rawscope-session-"))
        manifest_path = bundle_dir / "analysis.rawscope.json"
    else:
        bundle_dir, manifest_path = _destination_paths(destination)
        bundle_dir.parent.mkdir(parents=True, exist_ok=True)
        staging_dir = Path(
            tempfile.mkdtemp(prefix=f".{bundle_dir.name}.rawscope-", dir=bundle_dir.parent)
        )
        bundle_dir = staging_dir
        manifest_path = staging_dir / "analysis.rawscope.json"
        staged_bundle = True

    dataset_path = bundle_dir / "data.parquet"
    try:
        adapter.write_parquet(dataset_path)
        payload = _manifest_payload(
            dataset_path="data.parquet",
            data_format="parquet",
            view=view,
            display_name=display_name,
            evidence_key=evidence_key,
            limit=limit,
        )
        _atomic_write_json(manifest_path, payload)
    except Exception:
        if temporary_bundle or staged_bundle:
            shutil.rmtree(bundle_dir, ignore_errors=True)
        raise
    if staged_bundle:
        final_bundle_dir, final_manifest_path = _destination_paths(destination)
        _publish_staged_bundle(bundle_dir, final_bundle_dir)
        bundle_dir = final_bundle_dir
        manifest_path = final_manifest_path
        dataset_path = final_bundle_dir / "data.parquet"
    return PreparedSession(
        bundle_dir=bundle_dir.resolve(),
        manifest_path=manifest_path.resolve(),
        dataset_path=dataset_path.resolve(),
        persistent=not temporary_bundle,
    )


def _publish_staged_bundle(staged_bundle: Path, final_bundle: Path) -> None:
    """Publish a complete generation while retaining the previous one on failure."""

    previous_bundle: Path | None = None
    if final_bundle.exists():
        previous_bundle = final_bundle.with_name(
            f".{final_bundle.name}.previous-{uuid.uuid4().hex}"
        )
        os.replace(final_bundle, previous_bundle)
    try:
        os.replace(staged_bundle, final_bundle)
    except Exception:
        if previous_bundle is not None and not final_bundle.exists():
            os.replace(previous_bundle, final_bundle)
        raise
    if previous_bundle is not None:
        shutil.rmtree(previous_bundle)


def _validate_dataframe(
    adapter: DataframeAdapter,
    view: ScatterView | TimelineView,
    display_name: str | None,
    evidence_key: str | None,
    limit: int | None,
) -> None:
    if display_name is not None:
        require_text(display_name, "dataset.display_name")
    if evidence_key is not None:
        require_text(evidence_key, "dataset.evidence_key")
    limit = validate_row_limit(limit)
    names = adapter.column_names()
    required = (
        (view.x, view.y) if isinstance(view, ScatterView) else (view.time, view.lane)
    )
    validate_required_columns(names, required)
    adapter.row_count()
    if evidence_key is not None:
        adapter.validate_evidence_key(evidence_key)
