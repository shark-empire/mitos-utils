//! Every mitos-utils applet as a callable library function, plus a
//! name -> (usage, function) dispatch table used by `mitos-box`
//! (`src/bin/mitos-box.rs`) to act as all ~50 of them from one
//! binary, busybox/toybox-style. Each `src/bin/<name>.rs` is a thin
//! wrapper that also calls straight into the matching module here --
//! see docs/architecture.md for why the crate is split this way.
//!
//! Every applet's `run` has the exact same signature
//! (`fn(Vec<String>) -> AppResult<()>`), enforced by `AppletFn`
//! below, so they can all live in one dispatch table.

pub mod basename;
pub mod cat;
pub mod chgrp;
pub mod chmod;
pub mod chown;
pub mod clear;
pub mod cp;
pub mod cut;
pub mod df;
pub mod diff;
pub mod dirname;
pub mod dmesg;
pub mod du;
pub mod echo;
pub mod env;
pub mod free;
pub mod grep;
pub mod groups;
pub mod head;
pub mod hostname;
pub mod id;
pub mod kill;
pub mod ln;
pub mod ls;
pub mod mkdir;
pub mod mount;
pub mod mv;
pub mod printenv;
pub mod printf;
pub mod ps;
pub mod pwd;
pub mod readlink;
pub mod realpath;
pub mod rm;
pub mod rmdir;
pub mod sleep;
pub mod sort;
pub mod stat;
pub mod sync;
pub mod tail;
pub mod tee;
pub mod touch;
pub mod tr;
/// `true` is a reserved keyword, so the module (and its source file)
/// is named `true_` -- but its registered applet *name* below is the
/// plain `"true"` a real invocation actually uses.
pub mod true_;
pub mod umount;
pub mod uname;
pub mod uniq;
pub mod uptime;
pub mod wc;
pub mod whoami;

use crate::common::errors::AppResult;

/// The signature every applet's `run` function has.
pub type AppletFn = fn(Vec<String>) -> AppResult<()>;

/// `(name, usage, run)` for every applet, in the same order as
/// `docs/commands.md`. `mitos-box` looks names up here; nothing else
/// in the crate needs to enumerate all 50 by hand.
pub const APPLETS: &[(&str, &str, AppletFn)] = &[
    ("cat", cat::USAGE, cat::run),
    ("ls", ls::USAGE, ls::run),
    ("mkdir", mkdir::USAGE, mkdir::run),
    ("rmdir", rmdir::USAGE, rmdir::run),
    ("touch", touch::USAGE, touch::run),
    ("cp", cp::USAGE, cp::run),
    ("mv", mv::USAGE, mv::run),
    ("rm", rm::USAGE, rm::run),
    ("ln", ln::USAGE, ln::run),
    ("pwd", pwd::USAGE, pwd::run),
    ("basename", basename::USAGE, basename::run),
    ("dirname", dirname::USAGE, dirname::run),
    ("realpath", realpath::USAGE, realpath::run),
    ("readlink", readlink::USAGE, readlink::run),
    ("stat", stat::USAGE, stat::run),
    ("echo", echo::USAGE, echo::run),
    ("printf", printf::USAGE, printf::run),
    ("head", head::USAGE, head::run),
    ("tail", tail::USAGE, tail::run),
    ("grep", grep::USAGE, grep::run),
    ("sort", sort::USAGE, sort::run),
    ("uniq", uniq::USAGE, uniq::run),
    ("wc", wc::USAGE, wc::run),
    ("cut", cut::USAGE, cut::run),
    ("tr", tr::USAGE, tr::run),
    ("tee", tee::USAGE, tee::run),
    ("diff", diff::USAGE, diff::run),
    ("ps", ps::USAGE, ps::run),
    ("kill", kill::USAGE, kill::run),
    ("sleep", sleep::USAGE, sleep::run),
    ("uptime", uptime::USAGE, uptime::run),
    ("free", free::USAGE, free::run),
    ("uname", uname::USAGE, uname::run),
    ("hostname", hostname::USAGE, hostname::run),
    ("env", env::USAGE, env::run),
    ("printenv", printenv::USAGE, printenv::run),
    ("whoami", whoami::USAGE, whoami::run),
    ("id", id::USAGE, id::run),
    ("groups", groups::USAGE, groups::run),
    ("df", df::USAGE, df::run),
    ("du", du::USAGE, du::run),
    ("mount", mount::USAGE, mount::run),
    ("umount", umount::USAGE, umount::run),
    ("sync", sync::USAGE, sync::run),
    ("dmesg", dmesg::USAGE, dmesg::run),
    ("chmod", chmod::USAGE, chmod::run),
    ("chown", chown::USAGE, chown::run),
    ("chgrp", chgrp::USAGE, chgrp::run),
    ("clear", clear::USAGE, clear::run),
    ("true", true_::USAGE, true_::run),
];
