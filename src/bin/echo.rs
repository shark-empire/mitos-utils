//! `echo` -- print arguments. Supports `-n` (no trailing newline)
//! and `-e` (interpret `\n`, `\t`, `\\` backslash escapes).

use mitos_utils::common::errors::{run, AppResult};

fn main() -> std::process::ExitCode {
    run("echo", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut no_newline = false;
    let mut interpret_escapes = false;
    let mut start = 0;

    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "-n" => no_newline = true,
            "-e" => interpret_escapes = true,
            "-ne" | "-en" => {
                no_newline = true;
                interpret_escapes = true;
            }
            _ => break,
        }
        start = i + 1;
    }

    let text = args[start..].join(" ");
    let text = if interpret_escapes { expand_escapes(&text) } else { text };

    if no_newline {
        print!("{}", text);
    } else {
        println!("{}", text);
    }
    Ok(())
}

fn expand_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
