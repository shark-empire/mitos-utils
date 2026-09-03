fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("whoami", mitos_utils::applets::whoami::USAGE, args, mitos_utils::applets::whoami::run)
}
