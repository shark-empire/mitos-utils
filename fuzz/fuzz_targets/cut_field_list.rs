//! Fuzzes `cut`'s `-f` field-list parser (`"1,3-5"` syntax) --
//! untrusted range parsing is a classic source of integer-overflow
//! and reversed-range panics.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(spec) = std::str::from_utf8(data) {
        let _ = mitos_utils::applets::cut::parse_field_list(spec);
    }
});
