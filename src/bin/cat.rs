use std::env;
use std::fs;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Read from standard input if no file argument is provided
    if args.len() < 2 {
        let mut buffer = String::new();
        if io::stdin().read_to_string(&mut buffer).is_ok() {
            print!("{}", buffer);
        }
        return;
    }

    for path in &args[1..] {
        match fs::read_to_string(path) {
            Ok(content) => print!("{}", content),
            Err(err) => eprintln!("cat: {}: {}", path, err),
        }
    }
}
