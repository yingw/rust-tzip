//! Test TorrentZip output correctness

use std::fs;
use std::path::PathBuf;
use tzip::TorrentZipWriter;

#[test]
fn test_torrentzip_structure() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let file_data = fs::read(test_dir.join("at-6-1_a.bin")).expect("Failed to read test data");
    
    let mut buffer = Vec::new();
    let mut tz = TorrentZipWriter::new(&mut buffer);
    tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
    tz.finish().expect("Failed to finish");
    
    let crc = tz.torrentzip_crc32().expect("Should have CRC32 after finish");
    
    // Verify ZIP structure
    assert!(buffer.len() > 0, "Buffer should not be empty");
    
    // Check local file header signature
    assert_eq!(&buffer[0..4], &[0x50, 0x4b, 0x03, 0x04], "Should have local file header signature");
    
    // Check version needed (20 = 2.0 for DEFLATE)
    let version = u16::from_le_bytes([buffer[4], buffer[5]]);
    assert_eq!(version, 20, "Version needed should be 2.0");
    
    // Check general purpose flag (bit 1 set for max compression)
    let flag = u16::from_le_bytes([buffer[6], buffer[7]]);
    assert_eq!(flag, 0x0002, "General purpose flag should have bit 1 set");
    
    // Check compression method (8 = DEFLATE)
    let method = u16::from_le_bytes([buffer[8], buffer[9]]);
    assert_eq!(method, 8, "Compression method should be DEFLATE");
    
    // Check DOS timestamp (0xBC00 = time, 0x2198 = date)
    let time = u16::from_le_bytes([buffer[10], buffer[11]]);
    let date = u16::from_le_bytes([buffer[12], buffer[13]]);
    assert_eq!(time, 0xBC00, "DOS time should be 23:32:00");
    assert_eq!(date, 0x2198, "DOS date should be Dec 24, 1996");
    
    // Check that EOCD signature exists
    let eocd_sig = [0x50, 0x4b, 0x05, 0x06];
    let found = buffer.windows(4).rposition(|w| w == eocd_sig);
    assert!(found.is_some(), "Should have EOCD signature");
    
    // Check that TorrentZip comment exists
    let comment = b"TORRENTZIPPED-";
    let has_comment = buffer.windows(comment.len()).any(|w| w == comment);
    assert!(has_comment, "Should have TorrentZip comment");
    
    eprintln!("Generated {} bytes", buffer.len());
    eprintln!("TorrentZip CRC32: {:08X}", crc);
}

#[test]
fn test_deterministic_output() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let file_data = fs::read(test_dir.join("at-6-1_a.bin")).expect("Failed to read test data");
    
    // Generate twice with same input
    let mut buffer1 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer1);
        tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }
    
    let mut buffer2 = Vec::new();
    {
        let mut tz = TorrentZipWriter::new(&mut buffer2);
        tz.add_file("at-6-1_a.bin", &file_data).expect("Failed to add file");
        tz.finish().expect("Failed to finish");
    }
    
    // Should be byte-identical
    assert_eq!(buffer1, buffer2, "Same input should produce identical output");
}
