//! Shared argument-preprocessing: POSIX `--` end-of-options handling,
//! used across applets so each of the ~50 tools doesn't reimplement
//! it (previously none of them did, at all -- see docs/compatibility.md).
//! `--help`/`--version` are handled centrally in
//! `common::errors::run` instead of here, since those apply
//! identically to every applet.

/// Split `args` on the first bare `"--"` token. Everything before it
/// is returned unchanged, for the caller's normal flag-parsing loop.
/// Everything after it is returned separately as forced-positional
/// arguments that must NOT be interpreted as flags even if they
/// start with `-` (e.g. `rm -- -oddly-named-file`). The `--` marker
/// itself is dropped. If there's no `--` token, all of `args` come
/// back in the first vector and the second is empty.
///
/// Typical use in an applet that collects positional file/path
/// arguments:
/// ```ignore
/// let (opts, forced_positional) = common::args::split_dashdash(args);
/// let mut files = Vec::new();
/// for arg in opts {
///     match arg.as_str() {
///         "-n" => ...,
///         _ => files.push(arg),
///     }
/// }
/// files.extend(forced_positional);
/// ```
pub fn split_dashdash(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    match args.iter().position(|a| a == "--") {
        Some(idx) => {
            let mut args = args;
            let rest = args.split_off(idx + 1);
            args.pop(); // drop the "--" marker itself
            (args, rest)
        }
        None => (args, Vec::new()),
    }
}
