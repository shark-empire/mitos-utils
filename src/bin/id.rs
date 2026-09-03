fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("id", mitos_utils::applets::id::USAGE, args, mitos_utils::applets::id::run)
}
