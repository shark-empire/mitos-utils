use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut show_hidden = false;
    let mut long_format = false;
    let mut recursive = false;
    let mut sort_by_size = false;
    let mut reverse_sort = false;
    let mut human_readable = false;
    let mut paths: Vec<PathBuf> = Vec::new();

    // Parse flags
    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 && arg != "-" {
            for flag in arg[1..].chars() {
                match flag {
                    'a' => show_hidden = true,
                    'l' => long_format = true,
                    'R' => recursive = true,
                    'S' => sort_by_size = true,
                    'r' => reverse_sort = true,
                    'h' => human_readable = true,
                    _ => {
                        eprintln!("ls: invalid option -- '{}'", flag);
                        process::exit(2);
                    }
                }
            }
        } else {
            paths.push(PathBuf::from(arg));
        }
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }

    let mut exit_code = 0;
    let multiple = paths.len() > 1;

    for path in paths {
        if !path.exists() {
            eprintln!(
                "ls: cannot access '{}': No such file or directory",
                path.display()
            );
            exit_code = 2; // GNU ls standard exit code for missing files
            continue;
        }

        if path.is_file() {
            if let Err(e) = print_file(&path, long_format, human_readable) {
                eprintln!("ls: {}: {}", path.display(), e);
                exit_code = 1;
            }
        } else {
            if multiple {
                println!("{}:", path.display());
            }
            if let Err(e) = list_dir(
                &path,
                show_hidden,
                long_format,
                recursive,
                sort_by_size,
                reverse_sort,
                human_readable,
            ) {
                eprintln!("ls: {}: {}", path.display(), e);
                exit_code = 1;
            }
            if multiple {
                println!();
            }
        }
    }

    process::exit(exit_code);
}

fn list_dir(
    path: &Path,
    show_hidden: bool,
    long_format: bool,
    recursive: bool,
    sort_by_size: bool,
    reverse_sort: bool,
    human_readable: bool,
) -> io::Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .filter(|e| show_hidden || !e.file_name().to_string_lossy().starts_with('.'))
        .collect();

    // Handle -S (Sort by size, largest first)
    if sort_by_size {
        entries.sort_by(|a, b| {
            let size_a = a.metadata().map(|m| m.len()).unwrap_or(0);
            let size_b = b.metadata().map(|m| m.len()).unwrap_or(0);
            size_b.cmp(&size_a)
        });
    } else {
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    }

    // Handle -r (Reverse sort)
    if reverse_sort {
        entries.reverse();
    }

    // Print current directory contents
    for entry in &entries {
        let p = entry.path();
        if let Err(e) = print_file(&p, long_format, human_readable) {
            eprintln!("ls: {}: {}", p.display(), e);
        }
    }

    // Handle -R (Recursive)
    if recursive {
        let mut subdirs: Vec<PathBuf> = entries
            .iter()
            .map(|e| e.path())
            .filter(|p| p.is_dir() && !p.ends_with(".") && !p.ends_with(".."))
            .collect();

        // Maintain the sorted order for subdirectories
        if reverse_sort && !sort_by_size {
            subdirs.reverse();
        }

        for subdir in subdirs {
            println!("\n{}:", subdir.display());
            if let Err(e) = list_dir(
                &subdir,
                show_hidden,
                long_format,
                recursive,
                sort_by_size,
                reverse_sort,
                human_readable,
            ) {
                eprintln!("ls: {}: {}", subdir.display(), e);
            }
        }
    }

    Ok(())
}

fn print_file(path: &Path, long_format: bool, human_readable: bool) -> io::Result<()> {
    if long_format {
        let metadata = fs::symlink_metadata(path)?;
        let size = metadata.len();
        let size_str = if human_readable {
            human_size(size)
        } else {
            size.to_string()
        };

        #[cfg(unix)]
        let perms = format_permissions(metadata.permissions().mode());
        #[cfg(not(unix))]
        let perms = "----------".to_string();

        let name = path.file_name().unwrap_or_default().to_string_lossy();
        // Simplified long format: permissions, links, user, group, size, name
        println!("{} 1 user group {:>8} {}", perms, size_str, name);
    } else {
        println!("{}", path.file_name().unwrap_or_default().to_string_lossy());
    }
    Ok(())
}

#[cfg(unix)]
fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(10);
    let file_type = mode & 0o170000;
    s.push(if file_type == 0o040000 {
        'd'
    } else if file_type == 0o120000 {
        'l'
    } else {
        '-'
    });

    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];

    for (bit, c) in bits {
        s.push(if mode & bit != 0 { c } else { '-' });
    }
    s
}

fn human_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut s = size as f64;
    let mut i = 0;
    while s >= 1024.0 && i < UNITS.len() - 1 {
        s /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}{}", size, UNITS[i])
    } else {
        format!("{:.1}{}", s, UNITS[i])
    }
}
