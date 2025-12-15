#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_fuzzing_example::validate_email;

fuzz_target!(|data: &[u8]| {
    // Fuzz the email validator - should never panic
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = validate_email(s);
    }
});
