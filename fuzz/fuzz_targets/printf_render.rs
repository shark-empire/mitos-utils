//! Fuzzes `printf`'s format-string renderer. The property being
//! checked is simply "never panics" -- hand-written `%`/`\`-escape
//! state machines like this one are exactly the kind of code where a
//! malformed or adversarial format string can trigger a slicing
//! panic or similar that hand-written tests miss.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // NUL-separated chunks: first is the format string, the rest are
    // %s/%d/%x arguments. Using raw bytes + a manual split (rather
    // than adding the `arbitrary` crate as a second fuzz-only
    // dependency) keeps fuzz/Cargo.toml down to the one dependency
    // this workspace member exists for.
    let mut parts = data
        .split(|&b| b == 0)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned());
    let Some(format) = parts.next() else { return };
    let args: Vec<String> = parts.collect();
    let _ = mitos_utils::applets::printf::render(&format, &args);
});
