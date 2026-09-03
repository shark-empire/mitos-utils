fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("sort", mitos_utils::applets::sort::USAGE, args, mitos_utils::applets::sort::run)
}
