//! tzip-check — CLI 工具：检测 ZIP/TZip 文件的 flag、编码、有效性
//!
//! 用法:
//!   cargo run --bin tzip-check <file1.zip> [file2.zip ...]
//!   # 或编译后直接运行:
//!   cargo build --bin tzip-check && ./target/debug/tzip-check *.zip

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use tzip::TorrentZipValidator;

/// ZIP header 解析结果
struct ZipInfo {
    path: String,
    is_zip: bool,
    is_tzip: bool,
    entries: Vec<EntryInfo>,
    tzip_errors: Vec<String>,
}

/// 单个文件条目（ZIP 内文件）
struct EntryInfo {
    name: String,
    has_non_ascii: bool,
    local_flag: u16,
    cd_flag: u16,

    /// 诊断标记
    /// "OK" = 正常
    /// "MISSING_BIT11" = 非 ASCII 但没设 bit 11
    /// "UNWANTED_BIT11" = ASCII 却设了 bit 11
    diag: &'static str,
}

fn analyze_zip(data: &[u8]) -> Option<(Vec<EntryInfo>, bool)> {
    if data.len() < 30 || data[0..4] != [0x50, 0x4B, 0x03, 0x04] {
        return None;
    }

    // 判断是否是 TZip：查找 EOCD 注释 TORRENTZIPPED-XXXXXXXX
    let is_tzip = data.windows(22).any(|w| {
        w[..14] == *b"TORRENTZIPPED-"
            && w[14..22].iter().all(|&b| b.is_ascii_hexdigit())
    });

    let mut entries = Vec::new();

    // 扫描 local file headers
    let mut pos = 0;
    while let Some(off) = data[pos..].windows(4).position(|w| w == b"PK\x03\x04") {
        let abs = pos + off;

        // 需要至少 30 字节 header
        if abs + 30 > data.len() {
            break;
        }

        let local_flag = u16::from_le_bytes([data[abs + 6], data[abs + 7]]);
        let fn_len = u16::from_le_bytes([data[abs + 26], data[abs + 27]]) as usize;
        let extra_len = u16::from_le_bytes([data[abs + 28], data[abs + 29]]) as usize;

        if abs + 30 + fn_len > data.len() {
            break;
        }

        let name_bytes = &data[abs + 30..abs + 30 + fn_len];
        let name = String::from_utf8_lossy(name_bytes).to_string();
        let has_non_ascii = name_bytes.iter().any(|&b| b > 0x7F);

        let local_bit11 = local_flag & 0x0800 != 0;

        let diag = if has_non_ascii && !local_bit11 {
            "MISSING_BIT11"
        } else if !has_non_ascii && local_bit11 {
            "UNWANTED_BIT11"
        } else {
            "OK"
        };

        entries.push(EntryInfo {
            name,
            has_non_ascii,
            local_flag,
            cd_flag: 0, // 稍后从 central directory 读取
            diag,
        });

        // 跳到下一个 header（+fn_len +extra_len +compressed_len）
        let comp_size = u32::from_le_bytes([
            data[abs + 18], data[abs + 19], data[abs + 20], data[abs + 21],
        ]) as usize;
        pos = abs + 30 + fn_len + extra_len + comp_size;
    }

    // 扫描 central directory headers 更新 cd_flag
    let mut pos = 0;
    while let Some(off) = data[pos..].windows(4).position(|w| w == b"PK\x01\x02") {
        let abs = pos + off;
        if abs + 46 > data.len() {
            break;
        }

        let cd_flag = u16::from_le_bytes([data[abs + 8], data[abs + 9]]);
        let cd_bit11 = cd_flag & 0x0800 != 0;
        let fn_len = u16::from_le_bytes([data[abs + 28], data[abs + 29]]) as usize;
        let extra_len = u16::from_le_bytes([data[abs + 30], data[abs + 31]]) as usize;
        let comment_len = u16::from_le_bytes([data[abs + 32], data[abs + 33]]) as usize;

        if abs + 46 + fn_len > data.len() {
            break;
        }

        let name_bytes = &data[abs + 46..abs + 46 + fn_len];
        let cd_name = String::from_utf8_lossy(name_bytes);

        // 找到对应的 local entry 更新 cd_flag
        if let Some(entry) = entries.iter_mut().find(|e| e.name == cd_name) {
            entry.cd_flag = cd_flag;

            // 以 cd_flag 重新评估诊断（cd 和 local 可能不一致）
            entry.diag = if entry.has_non_ascii && !cd_bit11 {
                "MISSING_BIT11"
            } else if !entry.has_non_ascii && cd_bit11 {
                "UNWANTED_BIT11"
            } else {
                "OK"
            };
        }

        pos = abs + 46 + fn_len + extra_len + comment_len;
    }

    Some((entries, is_tzip))
}

fn print_entry(idx: usize, e: &EntryInfo) {
    let flag_icon = match e.diag {
        "OK" => "  ✓",
        "MISSING_BIT11" => "  ✗",
        "UNWANTED_BIT11" => "  ⚠",
        _ => "  ?",
    };

    let charset = if e.has_non_ascii { "非ASCII" } else { "ASCII  " };

    println!(
        "{} entry[{}]: {} ({}  local=0x{:04x}  cd=0x{:04x})",
        flag_icon, idx, e.name, charset, e.local_flag, e.cd_flag,
    );

    let details = match e.diag {
        "OK" => "    正常".to_string(),
        "MISSING_BIT11" => "    问题: 非ASCII文件名但缺少 bit 11 → 可能乱码".to_string(),
        "UNWANTED_BIT11" => "    注意: ASCII文件名却设了 bit 11 → outer hash 不兼容".to_string(),
        _ => String::new(),
    };
    if !details.is_empty() {
        println!("{}", details);
    }
}

fn analyze_file(path: &Path) -> ZipInfo {
    let path_str = path.to_string_lossy().to_string();
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            return ZipInfo {
                path: path_str,
                is_zip: false,
                is_tzip: false,
                entries: vec![],
                tzip_errors: vec![format!("读取失败: {}", e)],
            };
        }
    };

    // 基本 ZIP 检测
    let analyzed = analyze_zip(&data);
    let (entries, is_tzip) = match analyzed {
        Some(v) => v,
        None => {
            return ZipInfo {
                path: path_str,
                is_zip: false,
                is_tzip: false,
                entries: vec![],
                tzip_errors: vec![],
            };
        }
    };

    // 额外 TZip 校验
    let tzip_errors = if entries.is_empty() {
        vec!["ZIP 内无文件条目".to_string()]
    } else {
        match TorrentZipValidator::validate(&data) {
            Ok(result) => {
                if result.is_valid {
                    vec![]
                } else {
                    result.errors.iter().map(|e| format!("{:?}", e)).collect()
                }
            }
            Err(e) => {
                vec![format!("校验异常: {}", e)]
            }
        }
    };

    ZipInfo {
        path: path_str,
        is_zip: true,
        is_tzip,
        entries,
        tzip_errors,
    }
}

fn print_header() {
    println!("{}", "-".repeat(72));
    println!("  ZIP / TorrentZip 诊断工具");
    println!("{}", "-".repeat(72));
}

fn print_file_info(info: &ZipInfo) {
    println!();
    println!("文件: {}", info.path);

    if !info.is_zip {
        println!("  └─ 不是 ZIP 文件");
        return;
    }

    // 第一行：格式摘要
    let tzip_tag = if info.is_tzip { "是" } else { "否" };
    let entry_count = info.entries.len();
    println!("  TZip: {}  |  条目数: {}", tzip_tag, entry_count);

    // 打印每个 entry
    for (i, entry) in info.entries.iter().enumerate() {
        print_entry(i, entry);
    }

    // TZip 校验错误
    if !info.tzip_errors.is_empty() {
        println!("  TZip 校验问题:");
        for err in &info.tzip_errors {
            println!("    - {}", err);
        }
    }
}

fn print_summary(infos: &[ZipInfo]) {
    let total = infos.len();
    let zips = infos.iter().filter(|i| i.is_zip).count();
    let tzips = infos.iter().filter(|i| i.is_tzip).count();
    let non_ascii_ok = infos
        .iter()
        .flat_map(|i| &i.entries)
        .filter(|e| e.has_non_ascii && e.diag == "OK")
        .count();
    let missing_b11 = infos
        .iter()
        .flat_map(|i| &i.entries)
        .filter(|e| e.diag == "MISSING_BIT11")
        .count();
    let unwanted_b11 = infos
        .iter()
        .flat_map(|i| &i.entries)
        .filter(|e| e.diag == "UNWANTED_BIT11")
        .count();

    println!();
    println!("{}", "=".repeat(72));
    println!("  汇总");
    println!("{}", "=".repeat(72));
    println!("  扫描文件: {} (其中 ZIP: {}, TZip: {})", total, zips, tzips);
    println!("  非ASCII文件名 (正常有 bit11): {}", non_ascii_ok);
    println!("  非ASCII文件名 (缺少 bit11):   {}", missing_b11);
    println!("  ASCII文件名 (多余 bit11):     {}", unwanted_b11);
    println!();
}

fn print_usage() {
    eprintln!("用法: tzip-check <ZIP文件路径1> [ZIP文件路径2 ...]");
    eprintln!();
    eprintln!("示例:");
    eprintln!("  cargo run --bin tzip-check -- *.zip");
    eprintln!("  cargo run --bin tzip-check -- /path/to/roms/*.zip");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return ExitCode::FAILURE;
    }

    print_header();

    let mut infos = Vec::new();
    for arg in &args {
        let path = Path::new(arg);
        if !path.exists() {
            eprintln!("文件不存在: {}", arg);
            continue;
        }
        let info = analyze_file(path);
        print_file_info(&info);
        infos.push(info);
    }

    print_summary(&infos);

    // 如果没有任何问题返回 0，有问题返回 1
    let has_issues = infos.iter().flat_map(|i| &i.entries).any(|e| e.diag != "OK");
    if has_issues {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
