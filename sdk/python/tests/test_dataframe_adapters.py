from __future__ import annotations

import gc
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).parents[1] / "src"))

try:
    import pandas as pd
except ImportError:
    pd = None

try:
    import polars as pl
except ImportError:
    pl = None

try:
    import pyarrow as pa
    import pyarrow.parquet as parquet
except ImportError:
    pa = None
    parquet = None

import rawscope


HAS_PANDAS = pd is not None
HAS_POLARS = pl is not None
HAS_ARROW = pa is not None and parquet is not None


class DataframeAdapterTests(unittest.TestCase):
    @unittest.skipUnless(HAS_PANDAS, "pandas test extra is not installed")
    def test_pandas_dataframe_materializes_supported_parquet(self) -> None:
        dataframe = pd.DataFrame(
            {
                "white_rating": [1500, 1600],
                "black_rating": [1550, 1580],
                "game_id": ["g-1", "g-2"],
            }
        )

        with tempfile.TemporaryDirectory() as temporary:
            session = rawscope.prepare(
                dataframe,
                view=rawscope.ScatterView("white_rating", "black_rating"),
                destination=Path(temporary) / "bundle",
                evidence_key="game_id",
            )
            table = parquet.read_table(session.dataset_path)

            self.assertEqual(table.column_names, ["white_rating", "black_rating", "game_id"])
            self.assertEqual(table.column("white_rating").to_pylist(), [1500, 1600])
            self.assertEqual(table.column("game_id").to_pylist(), ["g-1", "g-2"])

    @unittest.skipUnless(HAS_POLARS, "Polars test extra is not installed")
    def test_polars_dataframe_and_lazyframe_preserve_row_order(self) -> None:
        dataframe = pl.DataFrame(
            {
                "white_rating": [1500, 1600],
                "black_rating": [1550, 1580],
                "game_id": ["g-1", "g-2"],
            }
        )
        view = rawscope.ScatterView("white_rating", "black_rating")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            eager = rawscope.prepare(dataframe, view=view, destination=root / "eager")
            lazy = rawscope.prepare(dataframe.lazy(), view=view, destination=root / "lazy")

            self.assertEqual(parquet.read_table(eager.dataset_path).column("game_id").to_pylist(), ["g-1", "g-2"])
            self.assertEqual(parquet.read_table(lazy.dataset_path).column("game_id").to_pylist(), ["g-1", "g-2"])

    @unittest.skipUnless(HAS_ARROW, "PyArrow test extra is not installed")
    def test_pyarrow_table_and_record_batch_materialize(self) -> None:
        table = pa.table(
            {
                "created_at": [1, 2],
                "winner": ["white", "black"],
            }
        )
        view = rawscope.TimelineView("created_at", "winner")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            from_table = rawscope.prepare(table, view=view, destination=root / "table")
            from_batch = rawscope.prepare(
                table.to_batches(max_chunksize=2)[0],
                view=view,
                destination=root / "batch",
            )

            self.assertEqual(parquet.read_table(from_table.dataset_path).num_rows, 2)
            self.assertEqual(parquet.read_table(from_batch.dataset_path).num_rows, 2)

    @unittest.skipUnless(HAS_ARROW, "PyArrow test extra is not installed")
    def test_evidence_key_rejects_null_or_duplicate_values(self) -> None:
        duplicate = pa.table({"x": [1, 2], "y": [2, 3], "game_id": ["g-1", "g-1"]})
        missing = pa.table({"x": [1, 2], "y": [2, 3], "game_id": ["g-1", None]})
        view = rawscope.ScatterView("x", "y")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(rawscope.DataframeSchemaError, "duplicates"):
                rawscope.prepare(
                    duplicate,
                    view=view,
                    destination=root / "duplicate",
                    evidence_key="game_id",
                )
            with self.assertRaisesRegex(rawscope.DataframeSchemaError, "blank"):
                rawscope.prepare(
                    missing,
                    view=view,
                    destination=root / "missing",
                    evidence_key="game_id",
                )

    @unittest.skipUnless(HAS_ARROW, "PyArrow test extra is not installed")
    def test_process_lifetime_retains_then_cleans_owned_bundle(self) -> None:
        table = pa.table({"x": [1], "y": [2]})
        process_mock = MagicMock()
        process_mock.pid = 21
        process_mock.wait.return_value = 0
        process_mock.poll.return_value = None

        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "rawscope-test-workbench.exe"
            executable.write_bytes(b"")
            with patch("rawscope.launcher.subprocess.Popen", return_value=process_mock):
                launched = rawscope.view(
                    table,
                    view=rawscope.ScatterView("x", "y"),
                    executable=executable,
                )
                bundle_dir = launched.session.bundle_dir
                self.assertTrue(bundle_dir.is_dir())
                self.assertFalse(launched.session.persistent)
                self.assertEqual(launched.wait(), 0)

        self.assertFalse(bundle_dir.exists())

    @unittest.skipUnless(HAS_ARROW, "PyArrow test extra is not installed")
    def test_process_drop_does_not_delete_live_owned_bundle(self) -> None:
        table = pa.table({"x": [1], "y": [2]})
        process_mock = MagicMock()
        process_mock.pid = 22
        process_mock.poll.return_value = None

        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "rawscope-test-workbench.exe"
            executable.write_bytes(b"")
            with patch("rawscope.launcher.subprocess.Popen", return_value=process_mock):
                launched = rawscope.view(
                    table,
                    view=rawscope.ScatterView("x", "y"),
                    executable=executable,
                )
                bundle_dir = launched.session.bundle_dir
                del launched
                gc.collect()

        self.assertTrue(bundle_dir.exists())
        import shutil

        shutil.rmtree(bundle_dir)

    @unittest.skipUnless(HAS_ARROW, "PyArrow test extra is not installed")
    def test_persistent_bundle_is_never_auto_deleted(self) -> None:
        table = pa.table({"x": [1], "y": [2]})
        process_mock = MagicMock()
        process_mock.pid = 23
        process_mock.poll.return_value = 0

        with tempfile.TemporaryDirectory() as temporary:
            bundle_dir = Path(temporary) / "persistent"
            executable = Path(temporary) / "rawscope-test-workbench.exe"
            executable.write_bytes(b"")
            with patch("rawscope.launcher.subprocess.Popen", return_value=process_mock):
                session = rawscope.prepare(
                    table,
                    view=rawscope.ScatterView("x", "y"),
                    destination=bundle_dir,
                )
                launched = rawscope.launch(
                    session,
                    executable=executable,
                )
                launched.wait()

            self.assertTrue(session.bundle_dir.is_dir())
            self.assertTrue(session.manifest_path.is_file())
            self.assertEqual(
                json.loads(session.manifest_path.read_text(encoding="utf-8"))["dataset"]["path"],
                "data.parquet",
            )


if __name__ == "__main__":
    unittest.main()
