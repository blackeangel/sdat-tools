use crate::tlist;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}: io error: {1}")]
    Io(PathBuf, #[source] io::Error),
    #[error("{0}: file data is not aligned to block size ({1} bytes)")]
    Alignment(PathBuf, u32),
    #[error("{0}:{1}")]
    TransferList(PathBuf, #[source] tlist::ReadError),
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error(transparent)]
    Read(io::Error),
    #[error(transparent)]
    Write(io::Error),
    #[error(transparent)]
    TransferListRead(#[from] tlist::ReadError),
}

pub fn check_file_alignment(filepath: &Path, block_size: u32) -> Result<u64, Error> {
    let meta = std::fs::metadata(filepath).map_err(|e| Error::Io(filepath.to_owned(), e))?;
    let len = meta.len();
    if !len.is_multiple_of(u64::from(block_size)) {
        return Err(Error::Alignment(filepath.to_owned(), block_size));
    }
    Ok(len)
}

pub fn file_prefix(filepath: &Path) -> Result<&OsStr, Error> {
    filepath.file_prefix().ok_or_else(|| {
        Error::Io(
            filepath.to_owned(),
            io::Error::new(io::ErrorKind::InvalidInput, "invalid file name"),
        )
    })
}
