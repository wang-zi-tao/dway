use drm::control::RawResourceHandle;

pub unsafe fn transmute_vec_from_u32<T: From<RawResourceHandle>>(raw: Vec<u32>) -> Vec<T> {
    let mut from = std::mem::ManuallyDrop::new(raw);
    Vec::from_raw_parts(from.as_mut_ptr() as *mut T, from.len(), from.capacity())
}
