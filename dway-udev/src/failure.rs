use drm::SystemError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DWayTTYError {
    #[error("failed to get drm resource handles: {0}")]
    ResourceHandlesError(SystemError),
    #[error("failed to get drm planes handles: {0}")]
    PlanesHandlesError(SystemError),
    #[error("failed to get drm property: {0}")]
    GetPropertyError(SystemError),
    #[error("failed to get drm connector: {0}")]
    GetConnectorError(SystemError),
    #[error("failed to get drm encoder: {0}")]
    GetEncoderError(SystemError),
    #[error("failed to get drm crtc: {0}")]
    GetCrtcError(SystemError),
    #[error("failed to set drm property: {0}")]
    SetPropertyError(SystemError),
    #[error("failed to set drm connector state: {0}")]
    SetConnectorStateError(SystemError),
    #[error("failed to set drm cursor state: {0}")]
    SetCursorStateError(SystemError),
    #[error("failed to set drm crtc state: {0}")]
    SetCrtcStateError(SystemError),
    #[error("failed to commit drm state: {0}")]
    AtomicCommitError(SystemError),
    #[error("no such property: {0}")]
    NoSuchProperty(String),
    #[error("drm has no promary plane")]
    NoPrimaryPlane,
    #[error("{0}")]
    UnknownError(#[from] anyhow::Error),
}
