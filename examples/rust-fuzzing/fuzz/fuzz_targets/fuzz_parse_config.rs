#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_fuzzing_example::parse_config;

fuzz_target!(|data: &[u8]| {
    // Fuzz the config parser - should never panic
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_config(s);
    }
});
