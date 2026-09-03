fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    mitos_utils::common::errors::run("umount", mitos_utils::applets::umount::USAGE, args, mitos_utils::applets::umount::run)
}
