fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let quickdev = dir.join("quickdev");

    let args: Vec<String> = std::env::args().skip(1).collect();

    let status = std::process::Command::new(&quickdev)
        .args(&args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to run quickdev: {e}");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}
