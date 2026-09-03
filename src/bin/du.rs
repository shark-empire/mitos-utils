fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("du", mitos_utils::applets::du::USAGE, args, mitos_utils::applets::du::run)
}
