use anyhow::Result;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("daemon") | Some("--daemon") => asr_switch::run_daemon(),
        _ => asr_switch::run(),
    }
}
