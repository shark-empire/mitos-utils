use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("mkdir: missing operand");
        std::process::exit(1);
    }

    for dir in &args[1..] {
        if let Err(err) = fs::create_dir_all(dir) {
            eprintln!("mkdir: cannot create directory '{}': {}", dir, err);
        }
    }
}
