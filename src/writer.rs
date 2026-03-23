//! TorrentZip writer implementation.

use std::io::Write;

use flate2::write::DeflateEncoder;
use flate2::Compression;

use crate::crc::compute_central_directory_crc32;
use crate::error::{Error, Result};
use crate::spec::{make_torrentzip_comment, CentralDirHeader, EndOfCentralDir, LocalFileHeader};

/// Internal representation of a file entry being written.
struct FileEntry {
    name: String,
    name_lower: String,
    data: Vec<u8>,
    crc32: u32,
    compressed: Vec<u8>,
}

/// TorrentZip-compliant ZIP file writer.
///
/// This writer produces deterministic, byte-identical ZIP archives
/// following the TorrentZip specification.
pub struct TorrentZipWriter<W: Write> {
    writer: W,
    entries: Vec<FileEntry>,
    torrentzip_crc32: Option<u32>,
    finished: bool,
}

impl TorrentZipWriter<Vec<u8>> {
    /// Create a new TorrentZip writer that writes to a Vec<u8> buffer.
    pub fn new_vec() -> Self {
        Self::new(Vec::new())
    }

    /// Get the finished ZIP data.
    ///
    /// Panics if not finished.
    pub fn into_inner(self) -> Vec<u8> {
        assert!(self.finished, "TorrentZipWriter not finished");
        self.writer
    }
}

impl<W: Write> TorrentZipWriter<W> {
    /// Create a new TorrentZip writer.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            entries: Vec::new(),
            torrentzip_crc32: None,
            finished: false,
        }
    }

    /// Add a file to the archive.
    pub fn add_file(&mut self, name: &str, data: &[u8]) -> Result<()> {
        if self.finished {
            return Err(Error::AlreadyFinished);
        }

        // Compute CRC32 of uncompressed data
        let crc32 = if data.is_empty() {
            0
        } else {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(data);
            hasher.finalize()
        };

        // Compress with DEFLATE level 9
        let compressed = if data.is_empty() {
            vec![]
        } else {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
            encoder.write_all(data)?;
            encoder.finish()?
        };

        let entry = FileEntry {
            name: name.to_string(),
            name_lower: name.to_lowercase(),
            data: data.to_vec(),
            crc32,
            compressed,
        };

        self.entries.push(entry);
        Ok(())
    }

    /// Finish writing the ZIP archive.
    pub fn finish(&mut self) -> Result<()> {
        if self.finished {
            return Err(Error::AlreadyFinished);
        }

        // Sort entries by lowercase filename
        self.entries.sort_by(|a, b| a.name_lower.cmp(&b.name_lower));

        // Build the entire ZIP in memory first
        let mut buffer: Vec<u8> = Vec::new();

        // Track local header offsets for central directory
        let mut local_offsets: Vec<(String, u32)> = Vec::new();
        let mut pos: u32 = 0;

        // Write local file headers + data
        for entry in &self.entries {
            local_offsets.push((entry.name.clone(), pos));

            // Write local file header
            let header = LocalFileHeader::new_torrentzip(entry.name.len() as u16);
            let mut header_bytes = header.to_bytes().to_vec();

            // Update with actual values
            let uncompressed_size = entry.data.len() as u32;
            let compressed_size = entry.compressed.len() as u32;

            // Update CRC32, compressed size, uncompressed size
            header_bytes[14..18].copy_from_slice(&entry.crc32.to_le_bytes());
            header_bytes[18..22].copy_from_slice(&compressed_size.to_le_bytes());
            header_bytes[22..26].copy_from_slice(&uncompressed_size.to_le_bytes());

            buffer.extend_from_slice(&header_bytes);
            buffer.extend_from_slice(entry.name.as_bytes());
            buffer.extend_from_slice(&entry.compressed);

            pos += LocalFileHeader::SIZE as u32
                + entry.name.len() as u32
                + compressed_size;
        }

        // Record central directory offset
        let cd_offset = buffer.len() as u32;
        let cd_start = buffer.len();

        // Write central directory headers
        for (name, local_offset) in &local_offsets {
            let entry = self
                .entries
                .iter()
                .find(|e| &e.name == name)
                .unwrap();

            let uncompressed_size = entry.data.len() as u32;
            let compressed_size = entry.compressed.len() as u32;

            let header = CentralDirHeader::new_torrentzip(name.len() as u16, *local_offset)
                .with_sizes(entry.crc32, compressed_size, uncompressed_size);

            buffer.extend_from_slice(&header.to_bytes());
            buffer.extend_from_slice(name.as_bytes());
        }

        let cd_size = (buffer.len() - cd_start) as u32;

        // Compute CRC32 of central directory
        let tz_crc32 = compute_central_directory_crc32(&buffer[cd_start..]);

        // Write EOCD
        let num_entries = self.entries.len() as u16;
        let eocd = EndOfCentralDir::new_torrentzip(num_entries, cd_size, cd_offset);
        buffer.extend_from_slice(&eocd.to_bytes());

        // Write TorrentZip comment
        let comment = make_torrentzip_comment(tz_crc32);
        buffer.extend_from_slice(&comment);

        // Write final buffer to output
        self.writer.write_all(&buffer)?;

        self.torrentzip_crc32 = Some(tz_crc32);
        self.finished = true;
        Ok(())
    }

    /// Get the TorrentZip CRC32 (available after finish()).
    pub fn torrentzip_crc32(&self) -> Option<u32> {
        self.torrentzip_crc32
    }

    /// Check if the writer has been finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }
}
