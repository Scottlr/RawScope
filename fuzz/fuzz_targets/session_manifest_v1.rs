#![no_main]

use libfuzzer_sys::fuzz_target;
use rawscope_session_contracts::parse_session_manifest;

const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|input: &[u8]| {
    let bounded = &input[..input.len().min(MAX_INPUT_BYTES)];
    let json = String::from_utf8_lossy(bounded);
    let _ = parse_session_manifest(&json);
});
