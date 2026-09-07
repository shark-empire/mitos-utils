//! `ls` -- list directory contents. Supports `-a` (show dotfiles),
//! `-l` (long format), `-h` (human-readable sizes with `-l`), and
//! multiple path arguments.

use crate::common::args::split_dashdash;
use crate::common::errors::{AppError, AppResult};
use crate::common::output::{columnate, error_path, human_size, stdout_writer};
#[cfg(unix)]
use crate::common::permissions::{file_type_char, format_mode, mode_of};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const USAGE: &str = "ls [-a] [-l] [-h] [PATH...] -- list directory contents";

pub fn run(args: Vec<String>) -> AppResult<()> {
    let mut show_all = false;
    let mut long = false;
    let mut human = false;
    let mut targets: Vec<String> = Vec::new();

    let (opts, forced) = split_dashdash(args);
    for arg in opts {
        if let Some(flags) = arg
            .strip_prefix('-')
            .filter(|f| !f.is_empty() && !arg.starts_with("--"))
        {
            for c in flags.chars() {
                match c {
                    'a' => show_all = true,
                    'l' => long = true,
                    'h' => human = true,
                    _ => return Err(AppError::usage(format!("invalid option -- '{}'", c))),
                }
            }
        } else {
            targets.push(arg);
        }
    }
    targets.extend(forced);
    if targets.is_empty() {
        targets.push(".".to_string());
    }

    let mut had_error = false;
    let multiple = targets.len() > 1;
    // One buffered handle for the entire invocation -- header lines,
    // long-format rows, and columnated names for every target all
    // funnel through it, so a directory listing with thousands of
    // entries (or `ls` given many path arguments) does a handful of
    // `write(2)` calls total instead of one per line. See
    // `common::output::stdout_writer`'s doc comment for why that
    // matters given `Stdout`'s own internal line-buffering.
    let mut out = stdout_writer();
    for (i, target) in targets.iter().enumerate() {
        if multiple {
            if i > 0 {
                let _ = writeln!(out);
            }
            let _ = writeln!(out, "{}:", target);
        }
        if let Err(err) = list_one(target, show_all, long, human, &mut out) {
            error_path("ls", target, err);
            had_error = true;
        }
    }
    let _ = out.flush();

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn list_one(
    target: &str,
    show_all: bool,
    long: bool,
    human: bool,
    out: &mut impl Write,
) -> std::io::Result<()> {
    let path = Path::new(target);
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        // Single non-directory target: `meta` was already fetched
        // above to check `is_dir()`, so hand it straight to
        // `print_entry` instead of statting the same path again.
        print_entry(target, long, human, Some(&meta), out);
        return Ok(());
    }

    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().into_string().unwrap_or_default();
        if !show_all && name.starts_with('.') {
            continue;
        }
        names.push(name);
    }
    names.sort();

    if long {
        for name in &names {
            let entry_meta = fs::symlink_metadata(path.join(name)).ok();
            print_entry(name, true, human, entry_meta.as_ref(), out);
        }
    } else {
        columnate(&names, 80, out);
    }
    Ok(())
}

fn print_entry(
    display_name: &str,
    long: bool,
    human: bool,
    meta: Option<&fs::Metadata>,
    out: &mut impl Write,
) {
    if !long {
        let _ = writeln!(out, "{}", display_name);
        return;
    }
    let Some(meta) = meta else {
        let _ = writeln!(out, "?????????? ? ? ? ? {}", display_name);
        return;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode_str = format_mode(mode_of(meta), file_type_char(meta));
        let owner = crate::common::users::name_for_uid(meta.uid())
            .unwrap_or_else(|| meta.uid().to_string());
        let group = crate::common::users::name_for_gid(meta.gid())
            .unwrap_or_else(|| meta.gid().to_string());
        let size = if human {
            human_size(meta.len())
        } else {
            meta.len().to_string()
        };
        let _ = writeln!(
            out,
            "{} {:>3} {:<8} {:<8} {:>8} {}",
            mode_str,
            meta.nlink(),
            owner,
            group,
            size,
            display_name
        );
    }
    #[cfg(not(unix))]
    {
        let size = if human {
            human_size(meta.len())
        } else {
            meta.len().to_string()
        };
        let _ = writeln!(out, "{:>10} {}", size, display_name);
    }
}
