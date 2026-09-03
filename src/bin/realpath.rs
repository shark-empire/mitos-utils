fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("realpath", mitos_utils::applets::realpath::USAGE, args, mitos_utils::applets::realpath::run)
}
