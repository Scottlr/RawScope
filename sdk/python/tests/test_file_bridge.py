from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).parents[1] / "src"))

import rawscope
from rawscope.bundle import prepare_dataframe
from rawscope.launcher import resolve_workbench_executable


class FileBridgeTests(unittest.TestCase):
    def test_row_limit_rejects_bool_zero_and_oversized_values(self) -> None:
        from rawscope.models import InvalidSession, MAX_SESSION_ROW_LIMIT

        with self.assertRaises(InvalidSession):
            rawscope.prepare("missing.csv", view=rawscope.ScatterView("x", "y"), destination=Path("bundle"), limit=True)
        with self.assertRaises(InvalidSession):
            rawscope.prepare("missing.csv", view=rawscope.ScatterView("x", "y"), destination=Path("bundle"), limit=0)
        with self.assertRaises(InvalidSession):
            rawscope.prepare("missing.csv", view=rawscope.ScatterView("x", "y"), destination=Path("bundle"), limit=MAX_SESSION_ROW_LIMIT + 1)
    def test_scatter_manifest_matches_rust_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dataset = root / "games.csv"
            dataset.write_text("white_rating,black_rating\n1500,1600\n", encoding="utf-8")

            session = rawscope.prepare(
                dataset,
                view=rawscope.ScatterView(
                    "white_rating", "black_rating", profile="lichess-games"
                ),
                destination=root / "session",
                display_name="Matchmaking games",
                evidence_key="game_id",
                limit=100,
            )
            payload = json.loads(session.manifest_path.read_text(encoding="utf-8"))

            self.assertEqual(
                payload,
                {
                    "artifact_kind": "rawscope.session",
                    "schema_version": 1,
                    "dataset": {
                        "path": str(dataset.resolve()),
                        "format": "csv",
                        "display_name": "Matchmaking games",
                        "limit": 100,
                        "evidence_key": "game_id",
                    },
                    "view": {
                        "kind": "scatter",
                        "x": "white_rating",
                        "y": "black_rating",
                        "profile": "lichess-games",
                    },
                },
            )

    def test_timeline_manifest_omits_absent_optional_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dataset = root / "events.csv"
            dataset.write_text("created_at,winner\n1,white\n", encoding="utf-8")

            session = rawscope.prepare(
                dataset,
                view=rawscope.TimelineView("created_at", "winner"),
                destination=root / "session",
            )
            payload = json.loads(session.manifest_path.read_text(encoding="utf-8"))

            self.assertEqual(payload["dataset"], {"path": str(dataset.resolve()), "format": "csv"})
            self.assertEqual(
                payload["view"],
                {"kind": "timeline", "time": "created_at", "lane": "winner"},
            )
            self.assertNotIn("profile", payload["view"])

    def test_prepare_references_existing_file_without_copying(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "source"
            bundle_root = root / "bundle"
            source_root.mkdir()
            dataset = source_root / "games.parquet"
            dataset.write_bytes(b"not read by the bridge")

            session = rawscope.prepare(
                dataset,
                view=rawscope.ScatterView("x", "y"),
                destination=bundle_root,
            )
            payload = json.loads(session.manifest_path.read_text(encoding="utf-8"))

            self.assertEqual(payload["dataset"]["path"], str(dataset.resolve()))
            self.assertFalse((bundle_root / dataset.name).exists())
            self.assertEqual(session.dataset_path, dataset.resolve())

    def test_launcher_uses_explicit_env_then_path_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            explicit = Path(temporary) / "explicit.exe"
            environment = Path(temporary) / "environment.exe"
            explicit.write_bytes(b"")
            environment.write_bytes(b"")

            with patch.dict(os.environ, {"RAWSCOPE_WORKBENCH": str(environment)}):
                self.assertEqual(resolve_workbench_executable(explicit), str(explicit.resolve()))
                self.assertEqual(
                    resolve_workbench_executable(), str(environment.resolve())
                )

            with patch.dict(os.environ, {}, clear=True), patch(
                "rawscope.launcher.shutil.which", return_value="on-path-workbench"
            ):
                self.assertEqual(resolve_workbench_executable(), "on-path-workbench")

    def test_launcher_passes_session_as_distinct_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dataset = root / "data.csv"
            dataset.write_text("x,y\n1,2\n", encoding="utf-8")
            session = rawscope.prepare(
                dataset,
                view=rawscope.ScatterView("x", "y"),
                destination=root / "bundle",
            )
            executable = root / "rawscope-workbench.exe"
            executable.write_bytes(b"")
            process = MagicMock()
            process.pid = 42
            with patch("rawscope.launcher.subprocess.Popen", return_value=process) as popen:
                launched = rawscope.launch(session, executable=executable)

            popen.assert_called_once_with(
                [
                    str(executable.resolve()),
                    "--session",
                    str(session.manifest_path),
                ],
                shell=False,
            )
            self.assertEqual(launched.pid, 42)

    def test_launcher_cleans_temporary_session_when_process_creation_fails(self) -> None:
        source = type("Frame", (), {})()
        with patch("rawscope.bundle.select_adapter") as select_adapter:
            adapter = select_adapter.return_value
            adapter.column_names.return_value = ("x", "y")
            adapter.row_count.return_value = 1
            adapter.write_parquet.side_effect = lambda path: path.write_bytes(b"parquet")
            with patch("rawscope.launcher.subprocess.Popen", side_effect=OSError("spawn failed")):
                session = prepare_dataframe(
                    source,
                    view=rawscope.ScatterView("x", "y"),
                    destination=None,
                    display_name=None,
                    evidence_key=None,
                    limit=None,
                )
                bundle_dir = session.bundle_dir
                executable = bundle_dir.parent / "rawscope-workbench.exe"
                executable.write_bytes(b"")
                with self.assertRaises(OSError), patch(
                    "rawscope.launcher.subprocess.Popen", side_effect=OSError("spawn failed")
                ):
                    rawscope.launch(session, executable=executable)
                self.assertFalse(bundle_dir.exists())

    def test_view_composes_prepare_and_launch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            dataset = root / "data.csv"
            dataset.write_text("x,y\n1,2\n", encoding="utf-8")
            executable = root / "rawscope-workbench.exe"
            executable.write_bytes(b"")
            process = MagicMock()
            process.pid = 7
            with patch("rawscope.launcher.subprocess.Popen", return_value=process):
                launched = rawscope.view(
                    dataset,
                    view=rawscope.ScatterView("x", "y"),
                    destination=root / "bundle",
                    executable=executable,
                )

            self.assertEqual(launched.pid, 7)
            self.assertTrue(launched.session.manifest_path.is_file())


if __name__ == "__main__":
    unittest.main()
