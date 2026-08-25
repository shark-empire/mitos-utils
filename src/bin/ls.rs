//! `ls` -- list directory contents. Supports `-a` (show dotfiles),
//! `-l` (long format), `-h` (human-readable sizes with `-l`), and
//! multiple path arguments.

use mitos_utils::common::errors::{run, AppError, AppResult};
use mitos_utils::common::output::{columnate, error_path, human_size};
#[cfg(unix)]
use mitos_utils::common::permissions::{file_type_char, format_mode, mode_of};
use std::fs;
use std::path::Path;

fn main() -> std::process::ExitCode {
    run("ls", real_main)
}

fn real_main() -> AppResult<()> {
    let mut show_all = false;
    let mut long = false;
    let mut human = false;
    let mut targets: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        if let Some(flags) = arg.strip_prefix('-').filter(|f| !f.is_empty() && !arg.starts_with("--")) {
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
    if targets.is_empty() {
        targets.push(".".to_string());
    }

    let mut had_error = false;
    let multiple = targets.len() > 1;
    for (i, target) in targets.iter().enumerate() {
        if multiple {
            if i > 0 {
                println!();
            }
            println!("{}:", target);
        }
        if let Err(err) = list_one(target, show_all, long, human) {
            error_path("ls", target, err);
            had_error = true;
        }
    }

    if had_error {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}

fn list_one(target: &str, show_all: bool, long: bool, human: bool) -> std::io::Result<()> {
    let path = Path::new(target);
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        print_entry(target, path, long, human);
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
            print_entry(name, &path.join(name), true, human);
        }
    } else {
        columnate(&names, 80);
    }
    Ok(())
}

fn print_entry(display_name: &str, full_path: &Path, long: bool, human: bool) {
    if !long {
        println!("{}", display_name);
        return;
    }
    let Ok(meta) = fs::symlink_metadata(full_path) else {
        println!("?????????? ? ? ? ? {}", display_name);
        return;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode_str = format_mode(mode_of(&meta), file_type_char(&meta));
        let owner = mitos_utils::common::users::name_for_uid(meta.uid())
            .unwrap_or_else(|| meta.uid().to_string());
        let group = mitos_utils::common::users::name_for_gid(meta.gid())
            .unwrap_or_else(|| meta.gid().to_string());
        let size = if human { human_size(meta.len()) } else { meta.len().to_string() };
        println!(
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
        let size = if human { human_size(meta.len()) } else { meta.len().to_string() };
        println!("{:>10} {}", size, display_name);
    }
}
