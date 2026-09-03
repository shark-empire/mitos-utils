fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("chown", mitos_utils::applets::chown::USAGE, args, mitos_utils::applets::chown::run)
}
