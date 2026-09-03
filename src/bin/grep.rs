fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("grep", mitos_utils::applets::grep::USAGE, args, mitos_utils::applets::grep::run)
}
