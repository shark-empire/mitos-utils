//! `clear` -- clear the terminal screen via the ANSI escape sequence
//! (no terminfo database lookup -- see docs/compatibility.md).

fn main() {
    print!("\x1b[2J\x1b[H");
}
