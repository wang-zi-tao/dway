use std::ptr::null_mut;

use anyhow::anyhow;
use anyhow::Result;
use bevy::utils::HashSet;
use khronos_egl::{Boolean, Int};

pub type EGLInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_4>;

pub fn call_egl_boolean(egl: &EGLInstance, f: impl FnOnce() -> Boolean) -> Result<()> {
    let r = f();
    if r != khronos_egl::TRUE {
        if let Some(err) = egl.get_error() {
            Err(anyhow!("egl error: {:?}", err))
        } else {
            Err(anyhow!("unknown egl error"))
        }
    } else {
        Ok(())
    }
}

pub fn call_egl_vec<T: Default>(
    egl: &EGLInstance,
    mut f: impl FnMut(Int, *mut T, *mut Int) -> Boolean,
) -> Result<Vec<T>> {
    let mut num = 0;
    call_egl_boolean(egl, || f(0, null_mut(), &mut num))?;
    if num == 0 {
        return Ok(vec![]);
    }
    let mut vec = Vec::new();
    vec.resize_with(num as usize, || Default::default());
    call_egl_boolean(egl, || f(num, vec.as_mut_ptr() as *mut T, &mut num))?;
    Ok(vec)
}

pub fn call_egl_double_vec<T1: Default, T2: Default>(
    egl: &EGLInstance,
    mut f: impl FnMut(Int, *mut T1, *mut T2, *mut Int) -> Boolean,
) -> Result<(Vec<T1>, Vec<T2>), khronos_egl::Error> {
    let on_error = |egl: &EGLInstance| {
        if let Some(err) = egl.get_error() {
            if err == khronos_egl::Error::BadParameter {
                return Ok((vec![], vec![]));
            } else {
                return Err(err);
            }
        } else {
            return Ok((vec![], vec![]));
        }
    };
    let mut num = 0;
    if f(0, null_mut(), null_mut(), &mut num) != khronos_egl::TRUE {
        return on_error(egl);
    }
    if num == 0 {
        return Ok((vec![], vec![]));
    }
    let mut vec1 = Vec::new();
    vec1.resize_with(num as usize, || Default::default());
    let mut vec2 = Vec::new();
    vec2.resize_with(num as usize, || Default::default());
    if f(
        num,
        vec1.as_mut_ptr() as *mut T1,
        vec2.as_mut_ptr() as *mut T2,
        &mut num,
    ) != khronos_egl::TRUE
    {
        return on_error(egl);
    }
    Ok((vec1, vec2))
}

pub fn get_egl_extensions(
    egl: &EGLInstance,
    egl_display: khronos_egl::Display,
) -> Result<HashSet<String>> {
    Ok(egl
        .query_string(Some(egl_display), khronos_egl::EXTENSIONS)?
        .to_string_lossy()
        .split(' ')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string())
        .collect())
}
