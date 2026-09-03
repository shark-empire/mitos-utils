fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("printf", mitos_utils::applets::printf::USAGE, args, mitos_utils::applets::printf::run)
}
