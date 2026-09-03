fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("hostname", mitos_utils::applets::hostname::USAGE, args, mitos_utils::applets::hostname::run)
}
