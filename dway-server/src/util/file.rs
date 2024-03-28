use crate::prelude::*;
use nix::{
    fcntl::{FcntlArg, SealFlag},
    sys::memfd::{memfd_create, MemFdCreateFlag},
};
use std::{
    ffi::CStr,
    fs::File,
    io::{Seek, Write},
    os::fd::AsRawFd,
};

pub fn create_sealed_file(name: &CStr, data: &[u8]) -> Result<(File, usize)> {
    let fd = memfd_create(
        name,
        MemFdCreateFlag::MFD_CLOEXEC | MemFdCreateFlag::MFD_ALLOW_SEALING,
    )?;

    let mut file = File::from(fd);
    file.write_all(data)?;
    file.flush()?;
    file.seek(std::io::SeekFrom::Start(0))?;

    nix::fcntl::fcntl(file.as_raw_fd(), FcntlArg::F_ADD_SEALS(SealFlag::all()))?;

    Ok((file, data.len()))
}
