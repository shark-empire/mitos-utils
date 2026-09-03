fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("head", mitos_utils::applets::head::USAGE, args, mitos_utils::applets::head::run)
}
