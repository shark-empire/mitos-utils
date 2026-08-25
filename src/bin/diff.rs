//! `diff` -- compare two files line by line and report differences
//! (a minimal unified-style output, not a full LCS diff -- see
//! docs/compatibility.md).

use mitos_utils::common::errors::{run, AppError, AppResult};

fn main() -> std::process::ExitCode {
    run("diff", real_main)
}

fn real_main() -> AppResult<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(AppError::usage("usage: diff FILE1 FILE2"));
    }

    let a = std::fs::read_to_string(&args[0]).map_err(|e| AppError::new(format!("{}: {}", args[0], e)))?;
    let b = std::fs::read_to_string(&args[1]).map_err(|e| AppError::new(format!("{}: {}", args[1], e)))?;

    let lines_a: Vec<&str> = a.lines().collect();
    let lines_b: Vec<&str> = b.lines().collect();
    let max = lines_a.len().max(lines_b.len());
    let mut differs = false;

    for i in 0..max {
        match (lines_a.get(i), lines_b.get(i)) {
            (Some(x), Some(y)) if x == y => {}
            (Some(x), Some(y)) => {
                differs = true;
                println!("{}c{}", i + 1, i + 1);
                println!("< {}", x);
                println!("---");
                println!("> {}", y);
            }
            (Some(x), None) => {
                differs = true;
                println!("{}d{}", i + 1, lines_b.len());
                println!("< {}", x);
            }
            (None, Some(y)) => {
                differs = true;
                println!("{}a{}", lines_a.len(), i + 1);
                println!("> {}", y);
            }
            (None, None) => unreachable!(),
        }
    }

    if differs {
        Err(AppError::silent(1))
    } else {
        Ok(())
    }
}
