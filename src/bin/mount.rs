fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("mount", mitos_utils::applets::mount::USAGE, args, mitos_utils::applets::mount::run)
}
