use easygpu_lyon::lyon_tessellation::TessellationError;

/// All errors that `kludgine-core` can return.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// An error opening an image.
    #[error("error reading image: {0}")]
    Image(#[from] image::ImageError),
    /// An error parsing Json.
    #[error("error parsing json: {0}")]
    Json(#[from] json::Error),
    /// An error while rendering shapes.
    #[error("error tessellating shape")]
    Tessellation(TessellationError),
    /// An error while parsing sprite data.
    #[error("error parsing sprite data: {0}")]
    SpriteParse(String),
    /// The sprite's current tag has no frames.
    #[error("no frames could be found for the current tag")]
    InvalidSpriteTag,
}
