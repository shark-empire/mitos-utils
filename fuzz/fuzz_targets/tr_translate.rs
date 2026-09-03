//! Fuzzes `tr`'s character-translation and deletion logic --
//! specifically the `set2.get(idx).or_else(|| set2.last())`
//! index/truncation handling, an easy place for an off-by-one to
//! hide.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut parts = data
        .splitn(3, |&b| b == 0)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned());
    let Some(input) = parts.next() else { return };
    let Some(set1_str) = parts.next() else { return };
    let set2_str = parts.next().unwrap_or_default();
    let set1: Vec<char> = set1_str.chars().collect();
    let set2: Vec<char> = set2_str.chars().collect();

    let _ = mitos_utils::applets::tr::translate(&input, &set1, &set2);
    let _ = mitos_utils::applets::tr::delete_chars(&input, &set1);
});
