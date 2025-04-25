use bevy::ecs::error::BevyError;

#[derive(thiserror::Error, Debug)]
pub(crate) enum YoleckAssetLoaderError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("{0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("{0}")]
    Bevy(BevyError),
}

// For some reason #[from] doesn't work...
impl From<BevyError> for YoleckAssetLoaderError {
    fn from(value: BevyError) -> Self {
        Self::Bevy(value)
    }
}
