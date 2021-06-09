#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error sending a WindowMessage to a Window: {0}")]
    InternalWindowMessageSend(String),

    #[error("kludgine-core error: {0}")]
    Core(#[from] kludgine_core::Error),
    #[error("other error: {0}")]
    Other(#[from] anyhow::Error),
}
