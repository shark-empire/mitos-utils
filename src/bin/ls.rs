use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let target_dir = if args.len() > 1 { &args[1] } else { "." };

    match fs::read_dir(target_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    println!("{}", name);
                }
            }
        }
        Err(err) => eprintln!("ls: cannot access '{}': {}", target_dir, err),
    }
}
