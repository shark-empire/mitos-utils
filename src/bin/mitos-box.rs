//! `mitos-box` -- a single binary hosting every mitos-utils applet
//! (busybox/toybox-style), so a mitosOS install can carry one binary
//! plus ~50 symlinks/hardlinks instead of ~50 separate copies of the
//! same statically-linked `std` runtime. See docs/architecture.md
//! for the size-footprint rationale.
//!
//! Two ways to invoke it:
//! - As a symlink/hardlink named after an applet (the normal
//!   install shape): `ln -s mitos-box cat; ./cat file.txt` looks at
//!   `argv[0]`'s basename to find `cat` and dispatches straight to
//!   it, args and all.
//! - Directly, with the applet name as the first argument:
//!   `mitos-box cat file.txt`. Useful for testing without setting up
//!   symlinks, or when only `mitos-box` itself is on `PATH`.
//!
//! Either way it routes through the same `common::errors::run`
//! every standalone `src/bin/<name>.rs` uses, so `--help`/`--version`
//! and error reporting behave identically both ways.

use mitos_utils::applets::APPLETS;
use mitos_utils::common::errors::run;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    if args.is_empty() {
        eprintln!("mitos-box: could not determine argv[0]");
        return ExitCode::from(1);
    }
    let argv0 = args.remove(0);

    let basename = std::path::Path::new(&argv0)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mitos-box");

    // Invoked via a symlink/hardlink named after an applet -- the
    // normal case on a real install.
    if let Some((name, usage, applet)) = find(basename) {
        return run(name, usage, args, *applet);
    }

    // Invoked as `mitos-box` (or any other name that isn't a known
    // applet) itself: treat the first remaining argument as the
    // applet name, e.g. `mitos-box ls -la`.
    if let Some(requested) = args.first().cloned() {
        if let Some((name, usage, applet)) = find(&requested) {
            args.remove(0);
            return run(name, usage, args, *applet);
        }
        eprintln!("mitos-box: unknown applet '{}'", requested);
        list_applets();
        return ExitCode::from(2);
    }

    list_applets();
    ExitCode::from(0)
}

fn find(
    name: &str,
) -> Option<&'static (&'static str, &'static str, mitos_utils::applets::AppletFn)> {
    APPLETS.iter().find(|(n, _, _)| *n == name)
}

fn list_applets() {
    eprintln!("mitos-box: available applets:");
    let mut names: Vec<&str> = APPLETS.iter().map(|(n, _, _)| *n).collect();
    names.sort();
    for chunk in names.chunks(8) {
        eprintln!("  {}", chunk.join(" "));
    }
}
