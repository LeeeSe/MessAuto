pub fn parse_args() -> Option<(String, String)> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 4 && args[1] == "--floating-window" {
        Some((args[2].clone(), args[3].clone()))
    } else {
        None
    }
}

pub fn spawn_floating_window(code: &str, source: &str) -> std::io::Result<std::process::Child> {
    std::process::Command::new(std::env::current_exe()?)
        .arg("--floating-window")
        .arg(code)
        .arg(source)
        .spawn()
}
