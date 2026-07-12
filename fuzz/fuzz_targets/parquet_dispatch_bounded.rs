#![no_main]

use std::{fs, panic::AssertUnwindSafe, path::PathBuf};

use libfuzzer_sys::fuzz_target;
use rawscope_data::{load_dataset_schema, load_parquet_scatter_dataset};

const MAX_INPUT_BYTES: usize = 64 * 1024;

fn fuzz_path(extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "rawscope-fuzz-parquet-{}.{}",
        std::process::id(),
        extension
    ))
}

fuzz_target!(|input: &[u8]| {
    let bounded = &input[..input.len().min(MAX_INPUT_BYTES)];
    let path = fuzz_path("parquet");
    let _ = fs::write(&path, bounded);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = load_dataset_schema(&path, Some(1_024));
        let _ = load_parquet_scatter_dataset(&path, "x", "y", Some(1_024));
    }));
    let _ = fs::remove_file(path);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
});
