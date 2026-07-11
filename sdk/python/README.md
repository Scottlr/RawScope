# RawScope Python SDK

This local, source-installable package prepares a CSV or Parquet file as a
versioned RawScope session and launches the native workbench. It has no runtime
dependencies on pandas, Polars, or PyArrow; dataframe materialization is a
separate optional task.

```python
import rawscope

process = rawscope.view(
    "games.parquet",
    view=rawscope.ScatterView(
        "white_rating",
        "black_rating",
        profile="lichess-games",
    ),
    destination="rawscope-session",
    evidence_key="game_id",
)
```

The bundle contains `analysis.rawscope.json` and references the existing local
file. It does not upload or copy the source. Set `RAWSCOPE_WORKBENCH` or pass
`executable=` when `rawscope-workbench` is not on `PATH`.

`process.wait()` waits for the native application. The bridge uses an argument
list rather than a shell command string and accepts only local CSV/Parquet paths
in session schema v1.

Dataframe inputs use optional adapters and materialize a flat Parquet bundle:

```python
process = rawscope.view(
    games,
    view=rawscope.ScatterView("white_rating", "black_rating"),
    evidence_key="game_id",
)
```

Install the relevant local extra with `pip install -e "sdk/python[pandas]"`,
`"sdk/python[polars]"`, or `"sdk/python[arrow]"`. Temporary dataframe bundles live until
`process.wait()` or `process.terminate()` and are cleaned only under the SDK's
owned OS-temp prefix. A dataframe is materialized; it is not streamed or
zero-copy.
