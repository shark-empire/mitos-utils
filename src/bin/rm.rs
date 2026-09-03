fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("rm", mitos_utils::applets::rm::USAGE, args, mitos_utils::applets::rm::run)
}
