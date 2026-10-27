#![doc = include_str!("../README.md")]
mod generation;
mod inspection;
mod options;
mod process;
mod publication;

pub use generation::{Generated, Plan, generate, generate_with};
pub use inspection::{Check, Inspection, inspect};
pub use options::{Identity, Options, ProjectKind, clean_package_name};
pub use process::Programs;
pub use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("template: {0}")]
    Template(#[from] minijinja::Error),
    #[error("generation task: {0}")]
    Task(#[from] tokio::task::JoinError),
    #[error("operation cancelled")]
    Cancelled,
    #[error("generated files remain: {0}")]
    Published(String),
    #[error("{program}: {message}")]
    Process { program: String, message: String },
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn check_cancel(token: &CancellationToken) -> Result<()> {
    if token.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
