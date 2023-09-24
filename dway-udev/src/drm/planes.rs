use anyhow::Result;
use bevy::prelude::Rect;
use drm::control::{crtc, framebuffer, plane, property::Value, Device, PlaneType};

use super::{DrmDevice, surface::DrmTransform};
use crate::failure::DWayTTYError::*;

#[derive(Clone, Debug)]
pub struct PlaneConfig {
    pub src: Rect,
    pub dest: Rect,
    pub transform: DrmTransform,
    pub framebuffer: framebuffer::Handle,
}

#[derive(Clone, Debug)]
pub struct PlaneInfo {
    pub handle: plane::Handle,
    pub type_: PlaneType,
    pub zpos: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct Planes {
    pub primary: PlaneInfo,
    pub cursor: Option<PlaneInfo>,
    pub overlay: Vec<PlaneInfo>,
}

impl Planes {
    pub fn new(crtc: &crtc::Handle, drm: &DrmDevice) -> Result<Self> {
        let mut primary = None;
        let mut cursor = None;
        let mut overlay = Vec::new();
        let has_universal_planes = drm.inner.lock().unwrap().has_universal_planes;
        let plane_handles = drm.plane_handles().map_err(PlanesHandlesError)?;
        let resources = drm.resource_handles().map_err(ResourceHandlesError)?;

        for plane_handle in plane_handles {
            let plane = drm.get_plane(plane_handle)?;
            let filter = plane.possible_crtcs();
            if resources.filter_crtcs(filter).contains(crtc) {
                let zpos = drm
                    .try_with_prop(plane_handle, "zpos", |prop, value| {
                        Ok(match prop.value_type().convert_value(value) {
                            Value::UnsignedRange(u) => Some(u as i32),
                            Value::SignedRange(i) => Some(i as i32),
                            Value::Boolean(b) => Some(b.into()),
                            _ => None,
                        })
                    })?
                    .flatten();
                let plane_type = drm.with_prop(plane_handle, "type", |_, value| {
                    Ok(match value {
                        x if x == (PlaneType::Primary as u64) => PlaneType::Primary,
                        x if x == (PlaneType::Cursor as u64) => PlaneType::Cursor,
                        _ => PlaneType::Overlay,
                    })
                })?;
                let plane_info = PlaneInfo {
                    handle: plane_handle,
                    type_: plane_type,
                    zpos,
                };
                match plane_type {
                    PlaneType::Overlay => {
                        if has_universal_planes {
                            overlay.push(plane_info)
                        }
                    }
                    PlaneType::Primary => primary = Some(plane_info),
                    PlaneType::Cursor => {
                        if has_universal_planes {
                            cursor = Some(plane_info)
                        }
                    }
                };
            }
        }

        Ok(Self {
            primary: primary.ok_or(NoPrimaryPlane)?,
            cursor,
            overlay,
        })
    }
}
