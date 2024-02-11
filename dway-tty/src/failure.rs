use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DWayTTYError {
    #[error("failed to get drm resource handles: {0}")]
    ResourceHandlesError(io::Error),
    #[error("failed to get drm planes handles: {0}")]
    PlanesHandlesError(io::Error),
    #[error("failed to get drm property: {0}")]
    GetPropertyError(io::Error),
    #[error("failed to get drm connector: {0}")]
    GetConnectorError(io::Error),
    #[error("failed to get drm encoder: {0}")]
    GetEncoderError(io::Error),
    #[error("failed to get drm crtc: {0}")]
    GetCrtcError(io::Error),
    #[error("failed to set drm property: {0}")]
    SetPropertyError(io::Error),
    #[error("failed to set drm connector state: {0}")]
    SetConnectorStateError(io::Error),
    #[error("failed to set drm cursor state: {0}")]
    SetCursorStateError(io::Error),
    #[error("failed to set drm crtc state: {0}")]
    SetCrtcStateError(io::Error),
    #[error("failed to commit drm state: {0}")]
    AtomicCommitError(io::Error),
    #[error("no such property: {0}")]
    NoSuchProperty(String),
    #[error("drm has no promary plane")]
    NoPrimaryPlane,
    #[error("{0}")]
    UnknownError(#[from] anyhow::Error),
}
