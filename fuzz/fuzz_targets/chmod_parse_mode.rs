//! Fuzzes `chmod`'s octal/symbolic mode parser
//! (`common::permissions::parse_mode`), including its interaction
//! with an arbitrary starting "current mode" -- symbolic specs like
//! `u+x` compute their result relative to that value, so exercising
//! the combination (not just the spec string alone) is the point.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let current = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if let Ok(spec) = std::str::from_utf8(&data[4..]) {
        let _ = mitos_utils::common::permissions::parse_mode(spec, current);
    }
});
