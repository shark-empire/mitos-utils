fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("clear", mitos_utils::applets::clear::USAGE, args, mitos_utils::applets::clear::run)
}
