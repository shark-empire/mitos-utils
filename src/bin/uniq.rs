fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("uniq", mitos_utils::applets::uniq::USAGE, args, mitos_utils::applets::uniq::run)
}
