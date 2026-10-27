use cli_command_kit::Mode;
use rust_project_scaffold::{CancellationToken, Programs};
use std::io::Write;

#[tokio::main]
async fn main() {
    let token = CancellationToken::new();
    let signal_token = token.clone();
    let signal = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
    });
    let mode = Mode::from_env();
    let result = rust_scaffold_cli::execute(
        std::env::args_os(),
        mode,
        &Programs::default(),
        &token,
        &mut std::io::stdout(),
    )
    .await;
    signal.abort();
    if let Err(error) = result {
        let _ = std::io::stderr().write_all(error.diagnostic(mode).as_bytes());
        std::process::exit(error.exit_status());
    }
}
