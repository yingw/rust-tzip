//! Tests: conditional bit 11 (UTF-8 flag) for TorrentZip files.
//!
//! - ASCII-only 内文件名 → flag=0x0002（与标准 tzip 工具字节一致）
//! - 含非 ASCII 字节的内文件名 → flag=0x0802（避免 CP437 解码乱码）

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tzip::TorrentZipWriter;

/// 读取 ZIP local file header 的 general purpose flag
fn flag_local(data: &[u8]) -> u16 {
    let off = data.windows(4).position(|w| w == b"PK\x03\x04").expect("no local header");
    u16::from_le_bytes([data[off + 6], data[off + 7]])
}

/// 读取 ZIP central directory header 的 general purpose flag
fn flag_cd(data: &[u8]) -> u16 {
    let off = data.windows(4).position(|w| w == b"PK\x01\x02").expect("no central dir header");
    u16::from_le_bytes([data[off + 8], data[off + 9]])
}

// ================================================================
// 单文件 flag 检测
// ================================================================

#[test]
fn test_ascii_no_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("ActRaiser (USA).sfc", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0002, "ASCII: no bit 11");
    assert_eq!(flag_cd(&data), 0x0002, "ASCII cd: no bit 11");
}

#[test]
fn test_japanese_has_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("ドラゴンクエストVI 幻の大地 (Japan).sfc", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Japanese: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Japanese cd: has bit 11");
}

#[test]
fn test_chinese_has_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("蝙蝠侠 - 电视游戏 (世界).gb", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Chinese: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Chinese cd: has bit 11");
}

#[test]
fn test_korean_has_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("별의 커비 (한국).gb", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Korean: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Korean cd: has bit 11");
}

#[test]
fn test_cyrillic_has_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("Покемон (Россия).gb", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Cyrillic: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Cyrillic cd: has bit 11");
}

#[test]
fn test_accented_latin_has_bit11() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("Pokémon - Édition Rouge (France).gb", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Accented: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Accented cd: has bit 11");
}

#[test]
fn test_pure_non_ascii_has_bit11() {
    // 文件名全部是非 ASCII 字符（无任何 ASCII 字母）
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("ファイナルファンタジー.gba", b"mock").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    assert_eq!(flag_local(&data), 0x0802, "Pure non-ASCII: has bit 11");
    assert_eq!(flag_cd(&data), 0x0802, "Pure non-ASCII cd: has bit 11");
}

// ================================================================
// 多文件混合：一个 ASCII + 一个非 ASCII，各自 flag 应正确
// ================================================================

#[test]
fn test_mixed_ascii_and_non_ascii() {
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file("README.txt", b"hello").unwrap();
    tz.add_file("パズルボーイII (日版).gb", b"rom data").unwrap();
    tz.finish().unwrap();
    let data = tz.into_inner();

    // 扫描所有 local headers
    let mut pos = 0;
    let mut flags = vec![];
    while let Some(off) = data[pos..].windows(4).position(|w| w == b"PK\x03\x04") {
        let abs = pos + off;
        let flag = u16::from_le_bytes([data[abs + 6], data[abs + 7]]);
        let fn_len = u16::from_le_bytes([data[abs + 26], data[abs + 27]]) as usize;
        let name = String::from_utf8_lossy(&data[abs + 30..abs + 30 + fn_len]);
        flags.push((name.to_string(), flag));
        pos = abs + 30 + fn_len;
    }

    assert_eq!(flags.len(), 2, "should have 2 entries");
    // 排序按 tzip 规则：小写名排序 → "パズル..." < "readme.txt" 因为片假名 < 'r'
    // 按字节序，日文 UTF-8 首字节 \xe3 > 'r'，所以 ASCII 在前
    assert_eq!(flags[0].0, "README.txt");
    assert_eq!(flags[0].1, 0x0002, "ASCII entry: no bit 11");
    assert_eq!(flags[1].0, "パズルボーイII (日版).gb");
    assert_eq!(flags[1].1, 0x0802, "non-ASCII entry: has bit 11");
}

// ================================================================
// outer SHA1 一致性验证：对现有 ASCII TZip 解压后再打包，hash 不变
// ================================================================

#[test]
fn test_roundtrip_ascii_preserves_hash() {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let orig_path = test_dir.join("22vp931_orig.zip");
    let orig_data = fs::read(&orig_path).expect("missing test fixture");

    // 1) 计算原始 outer SHA1
    let orig_hash = {
        let mut h = sha1_smol::Sha1::new();
        h.update(&orig_data);
        h.digest().to_string()
    };

    // 2) 解压（用 tzip 的 reader 或简单找 local data）
    // 找到第一个文件的压缩数据
    let local_off = orig_data.windows(4).position(|w| w == b"PK\x03\x04").unwrap();
    let comp_size = u32::from_le_bytes([orig_data[local_off + 18], orig_data[local_off + 19],
                                        orig_data[local_off + 20], orig_data[local_off + 21]]) as usize;
    let fn_len = u16::from_le_bytes([orig_data[local_off + 26], orig_data[local_off + 27]]) as usize;
    let extra_len = u16::from_le_bytes([orig_data[local_off + 28], orig_data[local_off + 29]]) as usize;
    let inner_name = String::from_utf8_lossy(&orig_data[local_off + 30..local_off + 30 + fn_len]);

    // 压缩数据从 local header 之后开始
    let data_start = local_off + 30 + fn_len + extra_len;
    let compressed = &orig_data[data_start..data_start + comp_size];

    // 解压
    use flate2::read::DeflateDecoder;
    let mut decoder = DeflateDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    // 3) 用新的 tzip 重新打包
    let mut tz = TorrentZipWriter::new_vec();
    tz.add_file(&inner_name, &decompressed).unwrap();
    tz.finish().unwrap();
    let repacked = tz.into_inner();

    // 4) 计算重新打包后的 SHA1
    let repack_hash = {
        let mut h = sha1_smol::Sha1::new();
        h.update(&repacked);
        h.digest().to_string()
    };

    println!("orig:  {}", orig_hash);
    println!("repack: {}", repack_hash);
    assert_eq!(repacked.len(), orig_data.len(), "byte size mismatch");
    assert_eq!(repacked, orig_data, "repacked ZIP should be byte-identical to original");
}
