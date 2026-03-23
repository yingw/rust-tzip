//! # tzip - TorrentZip Library for Rust
//!
//! Create, update, and validate TorrentZip-formatted ZIP files.
//!
//! ## What is TorrentZip?
//!
//! TorrentZip is a standardized ZIP format designed for reproducible, deterministic
//! archives. It ensures that identical input always produces byte-identical output,
//! making it ideal for:
//!
//! - ROM preservation and distribution
//! - BitTorrent sharing (identical hashes across sources)
//! - Content-addressable storage
//!
//! ## TorrentZip Requirements
//!
//! - All files sorted by lowercase filename
//! - Fixed timestamp: December 24, 1996, 23:32:00
//! - DEFLATE compression at maximum level (9)
//! - Archive comment: `TORRENTZIPPED-{CRC32}` where CRC32 is of the central directory
//!
//! ## Example: Creating a TorrentZip
//!
//! ```rust
//! use tzip::TorrentZipWriter;
//! use std::io::Cursor;
//!
//! let mut buffer = Cursor::new(Vec::new());
//! let mut tz = TorrentZipWriter::new(&mut buffer);
//!
//! tz.add_file("game.bin", &[0x00, 0x01, 0x02, 0x03]).unwrap();
//! tz.add_file("readme.txt", b"This is a readme").unwrap();
//!
//! tz.finish().unwrap();
//!
//! // buffer now contains a valid TorrentZip archive
//! println!("TorrentZip CRC32: {:08X}", tz.torrentzip_crc32().unwrap());
//! ```

mod crc;
mod error;
mod reader;
pub mod spec;
mod writer;

pub use crc::compute_central_directory_crc32;
pub use error::{Error, Result, ValidationError};
pub use reader::{TorrentZipValidator, ValidationResult};
pub use writer::TorrentZipWriter;

/// Convenience function to create a TorrentZip from file pairs.
///
/// Files are automatically sorted by lowercase filename.
///
/// # Example
///
/// ```rust
/// use tzip::create_torrentzip;
/// use std::io::Cursor;
///
/// let files = vec![
///     ("file.txt".to_string(), b"Hello, World!".to_vec()),
///     ("data.bin".to_string(), vec![0x00, 0x01, 0x02]),
/// ];
///
/// let mut buffer = Cursor::new(Vec::new());
/// let crc32 = create_torrentzip(&mut buffer, files).unwrap();
/// println!("Created TorrentZip with CRC32: {:08X}", crc32);
/// ```
pub fn create_torrentzip<W, I>(writer: W, files: I) -> Result<u32>
where
    W: std::io::Write,
    I: IntoIterator<Item = (String, Vec<u8>)>,
{
    let mut tz = TorrentZipWriter::new(writer);

    // Sort files by lowercase name
    let mut sorted_files: Vec<_> = files.into_iter().collect();
    sorted_files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    for (name, data) in sorted_files {
        tz.add_file(&name, &data)?;
    }

    tz.finish()?;
    tz.torrentzip_crc32().ok_or(Error::NotFinished)
}
