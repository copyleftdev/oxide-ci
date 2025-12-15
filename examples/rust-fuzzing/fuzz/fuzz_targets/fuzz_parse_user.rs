#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_fuzzing_example::parse_user_input;

fuzz_target!(|data: &[u8]| {
    // Fuzz the JSON parser - should never panic
    let _ = parse_user_input(data);
});
