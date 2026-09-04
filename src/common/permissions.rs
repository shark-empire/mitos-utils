//! File mode formatting and parsing, shared by `ls -l`, `stat`, and
//! `chmod`.

#[cfg(unix)]
use std::fs::Metadata;

/// Render a Unix permission mode as the familiar 10-character
/// string (e.g. `-rw-r--r--`, `drwxr-xr-x`) used by `ls -l` and
/// `stat`. `file_type_char` is the column-1 marker (`d`, `l`, `-`,
/// ...); see `file_type_char` below for computing it from Metadata.
pub fn format_mode(mode: u32, file_type_char: char) -> String {
    let mut s = String::with_capacity(10);
    s.push(file_type_char);
    const BITS: [(u32, char); 9] = [
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
    for (mask, ch) in BITS {
        s.push(if mode & mask != 0 { ch } else { '-' });
    }
    if mode & 0o4000 != 0 {
        s.replace_range(3..4, if mode & 0o100 != 0 { "s" } else { "S" });
    }
    if mode & 0o2000 != 0 {
        s.replace_range(6..7, if mode & 0o010 != 0 { "s" } else { "S" });
    }
    if mode & 0o1000 != 0 {
        s.replace_range(9..10, if mode & 0o001 != 0 { "t" } else { "T" });
    }
    s
}

/// The column-1 file-type marker `ls -l`/`stat` use, from
/// `std::fs::Metadata`.
#[cfg(unix)]
pub fn file_type_char(meta: &Metadata) -> char {
    use std::os::unix::fs::FileTypeExt;
    let ft = meta.file_type();
    if ft.is_dir() {
        'd'
    } else if ft.is_symlink() {
        'l'
    } else if ft.is_block_device() {
        'b'
    } else if ft.is_char_device() {
        'c'
    } else if ft.is_fifo() {
        'p'
    } else if ft.is_socket() {
        's'
    } else {
        '-'
    }
}

#[cfg(unix)]
pub fn mode_of(meta: &Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o7777
}

/// Parse a `chmod`-style mode argument against `current`: either a
/// bare octal literal (`755`, `0644`) or a comma-separated list of
/// symbolic clauses (`u+x`, `go-w`, `a=r,u+w`). Covers the common
/// subset of POSIX chmod syntax (`u`/`g`/`o`/`a`, `+`/`-`/`=`,
/// `r`/`w`/`x`) -- see docs/compatibility.md for what's out of
/// scope (`X`, `s`, `t` symbolic bits, multiple `who` groups with
/// distinct ops in one clause beyond comma-separation).
pub fn parse_mode(spec: &str, current: u32) -> Result<u32, String> {
    if !spec.is_empty() && spec.chars().all(|c| c.is_ascii_digit()) {
        return u32::from_str_radix(spec, 8)
            .map(|m| m & 0o7777)
            .map_err(|_| format!("invalid mode: '{}'", spec));
    }

    let mut mode = current;
    for clause in spec.split(',') {
        mode = apply_symbolic_clause(clause, mode)?;
    }
    Ok(mode)
}

fn apply_symbolic_clause(clause: &str, mode: u32) -> Result<u32, String> {
    let op_pos = clause
        .find(['+', '-', '='])
        .ok_or_else(|| format!("invalid mode clause: '{}'", clause))?;
    let (who, rest) = clause.split_at(op_pos);
    let op = rest.as_bytes()[0] as char;
    let perms = &rest[1..];

    let who_mask: u32 = if who.is_empty() {
        0o777
    } else {
        who.chars().fold(0u32, |acc, c| {
            acc | match c {
                'u' => 0o700,
                'g' => 0o070,
                'o' => 0o007,
                'a' => 0o777,
                _ => 0,
            }
        })
    };

    let perm_bits: u32 = perms.chars().fold(0u32, |acc, c| {
        acc | match c {
            'r' => 0o444,
            'w' => 0o222,
            'x' => 0o111,
            _ => 0,
        }
    });
    let applied = perm_bits & who_mask;

    Ok(match op {
        '+' => mode | applied,
        '-' => mode & !applied,
        '=' => (mode & !who_mask) | applied,
        _ => mode,
    })
}
