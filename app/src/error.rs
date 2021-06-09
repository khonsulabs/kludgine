/// All errors that kludgine-app can return.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error occurred while communicating internally between windows.
    #[error("error sending a WindowMessage to a Window: {0}")]
    InternalWindowMessageSend(String),

    /// An error from `kludgine-core` occurred.
    #[error("kludgine-core error: {0}")]
    Core(#[from] kludgine_core::Error),

    /// An error from user code arose.
    #[error("other error: {0}")]
    Other(#[from] anyhow::Error),
}
