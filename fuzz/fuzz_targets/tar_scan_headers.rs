//! Fuzzes `tar`'s header-parsing path (`applets::tar::scan_archive_headers`,
//! the same `read_header`/`parse_header`/`skip_data` logic `-t`/`-x`
//! use against a real file) with arbitrary bytes and no filesystem
//! access at all. The property being fuzzed is "never panics" --
//! a checksum mismatch, a malformed size field, or plain garbage
//! should all come back as an `Err`, never a crash.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = mitos_utils::applets::tar::scan_archive_headers(data);
});
