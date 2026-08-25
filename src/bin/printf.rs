//! `printf` -- format and print arguments using a subset of the
//! POSIX/C `printf` format string: `%s`, `%d`, `%x`, `%%`, and the
//! `\n`/`\t`/`\\` escapes. See docs/compatibility.md for what's out
//! of scope (width/precision specifiers, `%f`).

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("printf", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((format, rest)) = args.split_first() else {
        return Err(AppError::usage("usage: printf FORMAT [ARGUMENT]..."));
    };

    print!("{}", render(format, rest));
    Ok(())
}

fn render(format: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut arg_idx = 0;
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            },
            '%' => match chars.next() {
                Some('%') => out.push('%'),
                Some('s') => {
                    out.push_str(args.get(arg_idx).map(String::as_str).unwrap_or(""));
                    arg_idx += 1;
                }
                Some('d') => {
                    let val: i64 = args.get(arg_idx).and_then(|a| a.parse().ok()).unwrap_or(0);
                    out.push_str(&val.to_string());
                    arg_idx += 1;
                }
                Some('x') => {
                    let val: i64 = args.get(arg_idx).and_then(|a| a.parse().ok()).unwrap_or(0);
                    out.push_str(&format!("{:x}", val));
                    arg_idx += 1;
                }
                Some(other) => {
                    out.push('%');
                    out.push(other);
                }
                None => out.push('%'),
            },
            _ => out.push(c),
        }
    }
    out
}
