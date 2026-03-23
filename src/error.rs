//! Error types for tzip crate.

use std::io;
use thiserror::Error;

/// Result type alias for tzip operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for tzip operations.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error during read/write operations
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Invalid ZIP structure
    #[error("Invalid ZIP structure: {0}")]
    InvalidZip(String),

    /// TorrentZip validation failed
    #[error("TorrentZip validation failed: {0}")]
    ValidationFailed(String),

    /// CRC32 mismatch
    #[error("CRC32 mismatch: expected {expected:08X}, got {actual:08X}")]
    CrcMismatch { expected: u32, actual: u32 },

    /// File too large for standard ZIP (needs ZIP64)
    #[error("File too large: {size} bytes (max 4GB for standard ZIP)")]
    FileTooLarge { size: u64 },

    /// Too many files for standard ZIP (needs ZIP64)
    #[error("Too many files: {count} (max 65535 for standard ZIP)")]
    TooManyFiles { count: usize },

    /// Operation called before finish() was called
    #[error("Archive not finished")]
    NotFinished,

    /// Operation called after finish() was called
    #[error("Archive already finished")]
    AlreadyFinished,

    /// Empty archive (no files added)
    #[error("Cannot create empty TorrentZip archive")]
    EmptyArchive,
}

/// Validation error for TorrentZip compliance checking.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Timestamp is not the fixed TorrentZip value (Dec 24, 1996 23:32:00)
    #[error("Wrong timestamp: expected TorrentZip fixed time (Dec 24, 1996 23:32:00)")]
    WrongTimestamp,

    /// Compression method is not DEFLATE
    #[error("Wrong compression method: expected DEFLATE (8), got {0}")]
    WrongCompressionMethod(u16),

    /// General purpose bit flag is incorrect
    #[error("Wrong general purpose bit flag: expected 0x0002, got 0x{0:04X}")]
    WrongGeneralFlag(u16),

    /// Files are not sorted by lowercase filename
    #[error("Files not sorted by lowercase name: {0:?} comes before {1:?}")]
    FilesNotSorted(String, String),

    /// Archive comment is missing or invalid
    #[error("Invalid or missing TorrentZip comment")]
    InvalidComment,

    /// Comment CRC32 doesn't match central directory
    #[error("Comment CRC32 mismatch: expected {expected:08X}, got {actual:08X}")]
    CommentCrcMismatch { expected: u32, actual: u32 },

    /// Compression level is not maximum (9)
    #[error("Compression level is not maximum (level 9 required)")]
    WrongCompressionLevel,

    /// Extra data fields present (not allowed in TorrentZip)
    #[error("Extra data fields present (not allowed in TorrentZip)")]
    ExtraDataPresent,

    /// File comments present (not allowed in TorrentZip)
    #[error("File comments present (not allowed in TorrentZip)")]
    FileCommentsPresent,
}

impl From<flate2::CompressError> for Error {
    fn from(err: flate2::CompressError) -> Self {
        Error::Compression(err.to_string())
    }
}

impl From<flate2::DecompressError> for Error {
    fn from(err: flate2::DecompressError) -> Self {
        Error::Compression(err.to_string())
    }
}
