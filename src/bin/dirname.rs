fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("dirname", mitos_utils::applets::dirname::USAGE, args, mitos_utils::applets::dirname::run)
}
