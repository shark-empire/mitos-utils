fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("sync", mitos_utils::applets::sync::USAGE, args, mitos_utils::applets::sync::run)
}
