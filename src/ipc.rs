pub fn parse_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "--floating-window" {
        Some(args[2].clone())
    } else {
        None
    }
}

pub fn spawn_floating_window(code: &str) -> std::io::Result<std::process::Child> {
    std::process::Command::new(std::env::current_exe()?)
        .arg("--floating-window")
        .arg(code)
        .spawn()
}
