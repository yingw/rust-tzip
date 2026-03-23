//! ZIP format constants and structures for TorrentZip compliance.
//!
//! Based on PKWARE APPNOTE 6.3.0 and TorrentZip specification.

/// Fixed DOS date for TorrentZip: December 24, 1996
///
/// DOS date format: bits 0-4 = day, 5-8 = month, 9-15 = year offset from 1980
/// Dec 24, 1996 = day=24, month=12, year=16
/// Binary: (16 << 9) | (12 << 5) | 24 = 0x2198 (8600 decimal)
pub const DOS_DATE: u16 = 0x2198;

/// Fixed DOS time for TorrentZip: 23:32:00 (11:32:00 PM)
///
/// DOS time format: bits 0-4 = seconds/2, 5-10 = minutes, 11-15 = hours
/// 23:32:00 = hours=23, minutes=32, seconds=0
/// Binary: 10111 100000 00000 = 0xBC00
pub const DOS_TIME: u16 = 0xBC00;

/// General purpose bit flag for TorrentZip.
///
/// Bit 1 (0x0002): Maximum compression flag - set for DEFLATE level 9
pub const GENERAL_PURPOSE_FLAG: u16 = 0x0002;

/// ZIP version needed to extract (2.0 = supports DEFLATE)
pub const VERSION_NEEDED: u16 = 20;

/// ZIP version made by (2.0 = standard)
pub const VERSION_MADE_BY: u16 = 20;

/// Compression method: DEFLATE
pub const COMPRESSION_METHOD_DEFLATE: u16 = 8;

/// Compression method: Stored (no compression)
pub const COMPRESSION_METHOD_STORED: u16 = 0;

/// Local file header signature
pub const LOCAL_FILE_HEADER_SIG: u32 = 0x04034b50;

/// Central directory file header signature
pub const CENTRAL_DIR_HEADER_SIG: u32 = 0x02014b50;

/// End of central directory record signature
pub const END_OF_CENTRAL_DIR_SIG: u32 = 0x06054b50;

/// TorrentZip archive comment prefix
pub const TORRENTZIP_COMMENT_PREFIX: &[u8] = b"TORRENTZIPPED-";

/// Length of TorrentZip comment (TORRENTZIPPED-XXXXXXXX = 22 bytes)
pub const TORRENTZIP_COMMENT_LEN: u16 = 22;

/// Local file header structure (before variable-length fields)
///
/// Offset | Size | Description
/// -------|------|------------
/// 0      | 4    | Signature (0x04034b50)
/// 4      | 2    | Version needed to extract
/// 6      | 2    | General purpose bit flag
/// 8      | 2    | Compression method
/// 10     | 2    | Last mod file time
/// 12     | 2    | Last mod file date
/// 14     | 4    | CRC-32
/// 18     | 4    | Compressed size
/// 22     | 4    | Uncompressed size
/// 26     | 2    | Filename length
/// 28     | 2    | Extra field length
/// 30     | n    | Filename
/// 30+n   | m    | Extra field
#[derive(Debug, Clone, Copy)]
pub struct LocalFileHeader {
    pub signature: u32,
    pub version_needed: u16,
    pub general_flag: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub filename_len: u16,
    pub extra_len: u16,
}

impl LocalFileHeader {
    /// Size of fixed header (before filename and extra data)
    pub const SIZE: usize = 30;

    /// Create a new TorrentZip-compliant local file header
    pub fn new_torrentzip(filename_len: u16) -> Self {
        Self {
            signature: LOCAL_FILE_HEADER_SIG,
            version_needed: VERSION_NEEDED,
            general_flag: GENERAL_PURPOSE_FLAG,
            compression_method: COMPRESSION_METHOD_DEFLATE,
            last_mod_time: DOS_TIME,
            last_mod_date: DOS_DATE,
            crc32: 0,         // Set after compression
            compressed_size: 0,    // Set after compression
            uncompressed_size: 0,  // Set after compression
            filename_len,
            extra_len: 0,
        }
    }

    /// Serialize header to bytes (little-endian)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[0..4].copy_from_slice(&self.signature.to_le_bytes());
        buf[4..6].copy_from_slice(&self.version_needed.to_le_bytes());
        buf[6..8].copy_from_slice(&self.general_flag.to_le_bytes());
        buf[8..10].copy_from_slice(&self.compression_method.to_le_bytes());
        buf[10..12].copy_from_slice(&self.last_mod_time.to_le_bytes());
        buf[12..14].copy_from_slice(&self.last_mod_date.to_le_bytes());
        buf[14..18].copy_from_slice(&self.crc32.to_le_bytes());
        buf[18..22].copy_from_slice(&self.compressed_size.to_le_bytes());
        buf[22..26].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        buf[26..28].copy_from_slice(&self.filename_len.to_le_bytes());
        buf[28..30].copy_from_slice(&self.extra_len.to_le_bytes());

        buf
    }
}

/// Central directory file header structure
///
/// Offset | Size | Description
/// -------|------|------------
/// 0      | 4    | Signature (0x02014b50)
/// 4      | 2    | Version made by
/// 6      | 2    | Version needed to extract
/// 8      | 2    | General purpose bit flag
/// 10     | 2    | Compression method
/// 12     | 2    | Last mod file time
/// 14     | 2    | Last mod file date
/// 16     | 4    | CRC-32
/// 20     | 4    | Compressed size
/// 24     | 4    | Uncompressed size
/// 28     | 2    | Filename length
/// 30     | 2    | Extra field length
/// 32     | 2    | File comment length
/// 34     | 2    | Disk number start
/// 36     | 2    | Internal file attributes
/// 38     | 4    | External file attributes
/// 42     | 4    | Relative offset of local header
/// 46     | n    | Filename
/// 46+n   | m    | Extra field
/// 46+n+m | k    | File comment
#[derive(Debug, Clone, Copy)]
pub struct CentralDirHeader {
    pub signature: u32,
    pub version_made_by: u16,
    pub version_needed: u16,
    pub general_flag: u16,
    pub compression_method: u16,
    pub last_mod_time: u16,
    pub last_mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub filename_len: u16,
    pub extra_len: u16,
    pub comment_len: u16,
    pub disk_start: u16,
    pub internal_attr: u16,
    pub external_attr: u32,
    pub local_header_offset: u32,
}

impl CentralDirHeader {
    /// Size of fixed header (before variable fields)
    pub const SIZE: usize = 46;

    /// Create a new TorrentZip-compliant central directory header
    pub fn new_torrentzip(filename_len: u16, local_header_offset: u32) -> Self {
        Self {
            signature: CENTRAL_DIR_HEADER_SIG,
            version_made_by: VERSION_MADE_BY,
            version_needed: VERSION_NEEDED,
            general_flag: GENERAL_PURPOSE_FLAG,
            compression_method: COMPRESSION_METHOD_DEFLATE,
            last_mod_time: DOS_TIME,
            last_mod_date: DOS_DATE,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            filename_len,
            extra_len: 0,
            comment_len: 0,
            disk_start: 0,
            internal_attr: 0,
            external_attr: 0,
            local_header_offset,
        }
    }

    /// Set CRC32, compressed size, and uncompressed size.
    pub fn with_sizes(mut self, crc32: u32, compressed_size: u32, uncompressed_size: u32) -> Self {
        self.crc32 = crc32;
        self.compressed_size = compressed_size;
        self.uncompressed_size = uncompressed_size;
        self
    }

    /// Serialize header to bytes (little-endian)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[0..4].copy_from_slice(&self.signature.to_le_bytes());
        buf[4..6].copy_from_slice(&self.version_made_by.to_le_bytes());
        buf[6..8].copy_from_slice(&self.version_needed.to_le_bytes());
        buf[8..10].copy_from_slice(&self.general_flag.to_le_bytes());
        buf[10..12].copy_from_slice(&self.compression_method.to_le_bytes());
        buf[12..14].copy_from_slice(&self.last_mod_time.to_le_bytes());
        buf[14..16].copy_from_slice(&self.last_mod_date.to_le_bytes());
        buf[16..20].copy_from_slice(&self.crc32.to_le_bytes());
        buf[20..24].copy_from_slice(&self.compressed_size.to_le_bytes());
        buf[24..28].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        buf[28..30].copy_from_slice(&self.filename_len.to_le_bytes());
        buf[30..32].copy_from_slice(&self.extra_len.to_le_bytes());
        buf[32..34].copy_from_slice(&self.comment_len.to_le_bytes());
        buf[34..36].copy_from_slice(&self.disk_start.to_le_bytes());
        buf[36..38].copy_from_slice(&self.internal_attr.to_le_bytes());
        buf[38..42].copy_from_slice(&self.external_attr.to_le_bytes());
        buf[42..46].copy_from_slice(&self.local_header_offset.to_le_bytes());

        buf
    }
}

/// End of central directory record
///
/// Offset | Size | Description
/// -------|------|------------
/// 0      | 4    | Signature (0x06054b50)
/// 4      | 2    | Number of this disk
/// 6      | 2    | Disk where central directory starts
/// 8      | 2    | Number of central directory records on this disk
/// 10     | 2    | Total number of central directory records
/// 12     | 4    | Size of central directory (bytes)
/// 16     | 4    | Offset of start of central directory
/// 20     | 2    | Comment length
/// 22     | n    | Comment
#[derive(Debug, Clone, Copy)]
pub struct EndOfCentralDir {
    pub signature: u32,
    pub disk_num: u16,
    pub disk_cd_start: u16,
    pub disk_entries: u16,
    pub total_entries: u16,
    pub cd_size: u32,
    pub cd_offset: u32,
    pub comment_len: u16,
}

impl EndOfCentralDir {
    /// Size of fixed record (before comment)
    pub const SIZE: usize = 22;

    /// Create a new EOCD for TorrentZip
    pub fn new_torrentzip(entries: u16, cd_size: u32, cd_offset: u32) -> Self {
        Self {
            signature: END_OF_CENTRAL_DIR_SIG,
            disk_num: 0,
            disk_cd_start: 0,
            disk_entries: entries,
            total_entries: entries,
            cd_size,
            cd_offset,
            comment_len: TORRENTZIP_COMMENT_LEN,
        }
    }

    /// Serialize record to bytes (little-endian, without comment)
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[0..4].copy_from_slice(&self.signature.to_le_bytes());
        buf[4..6].copy_from_slice(&self.disk_num.to_le_bytes());
        buf[6..8].copy_from_slice(&self.disk_cd_start.to_le_bytes());
        buf[8..10].copy_from_slice(&self.disk_entries.to_le_bytes());
        buf[10..12].copy_from_slice(&self.total_entries.to_le_bytes());
        buf[12..16].copy_from_slice(&self.cd_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.cd_offset.to_le_bytes());
        buf[20..22].copy_from_slice(&self.comment_len.to_le_bytes());

        buf
    }
}

/// Create the TorrentZip archive comment from a CRC32 value.
///
/// Format: `TORRENTZIPPED-{CRC32:08X}` (uppercase hex)
pub fn make_torrentzip_comment(crc32: u32) -> Vec<u8> {
    format!("TORRENTZIPPED-{:08X}", crc32).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dos_date() {
        // Verify: Dec 24, 1996
        // Year offset from 1980 = 16
        // Month = 12
        // Day = 24
        // Binary: (16 << 9) | (12 << 5) | 24 = 0x2198
        assert_eq!(DOS_DATE, 0x2198);
    }

    #[test]
    fn test_dos_time() {
        // Verify: 23:32:00
        // Hours = 23
        // Minutes = 32
        // Seconds / 2 = 0
        // Binary: (23 << 11) | (32 << 5) | 0 = 0xBC00
        assert_eq!(DOS_TIME, 0xBC00);
    }

    #[test]
    fn test_torrentzip_comment() {
        let comment = make_torrentzip_comment(0xF175FDED);
        assert_eq!(&comment, b"TORRENTZIPPED-F175FDED");
        assert_eq!(comment.len(), 22);
    }
}
