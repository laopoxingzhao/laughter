//! `.lgpack`：类似 JAR 的程序包（ZIP 容器）。
//!
//! ## 和 Java JAR 的对应关系
//!
//! | JAR | Laughter `.lgpack` |
//! |-----|-------------------|
//! | ZIP 文件 | ZIP 文件 |
//! | `META-INF/MANIFEST.MF` | 同路径，键值清单 |
//! | `.class` 字节码 | `.lgb` 字节码 |
//! | `Main-Class` | `Main-Bytecode`（包内 .lgb 路径） |
//! | 可选资源文件 | 同理，任意路径的资源 |
//!
//! ## 清单示例 `META-INF/MANIFEST.MF`
//!
//! ```text
//! Manifest-Version: 1.0
//! Created-By: laughter
//! Laughter-Package-Version: 1
//! Main-Bytecode: app.lgb
//! ```
//!
//! ## CLI
//!
//! ```text
//! laughter pack main.lg [-o app.lgpack] [--resource path]...
//! laughter exec app.lgpack
//! laughter disasm app.lgpack
//! ```
//!
//! `pack` 会把入口源码编译成一份 `Module`（import 已在 loader 阶段合并），
//! 写入包内 `Main-Bytecode` 指定的 `.lgb`，并附带清单与资源。

use std::io::{Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::codegen::lgb::{decode_module, encode_module, exec_module, FORMAT_VERSION};
use crate::module_loader::compile_file;

pub const MANIFEST_PATH: &str = "META-INF/MANIFEST.MF";
pub const PACKAGE_VERSION: &str = "1";

#[derive(Debug)]
pub struct PackError {
    pub message: String,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn err(msg: impl Into<String>) -> PackError {
    PackError {
        message: msg.into(),
    }
}

/// 包内容：入口字节码在 ZIP 内的路径 + 清单字段。
pub struct PackMeta {
    pub main_bytecode: String,
    pub package_version: String,
}

/// 解析 MANIFEST.MF 的简易 `Key: value` 行。
/// 解析清单 Key: value 行；缺省 Main-Bytecode 为 app.lgb
pub fn parse_manifest(text: &str) -> PackMeta {
    let mut main_bytecode = String::from("app.lgb");
    let mut package_version = String::from(PACKAGE_VERSION);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "Main-Bytecode" => main_bytecode = v.to_string(),
                "Laughter-Package-Version" => package_version = v.to_string(),
                _ => {}
            }
        }
    }
    PackMeta {
        main_bytecode,
        package_version,
    }
}

fn build_manifest(main_bytecode: &str) -> String {
    format!(
        "Manifest-Version: 1.0\n\
         Created-By: laughter\n\
         Laughter-Package-Version: {PACKAGE_VERSION}\n\
         Main-Bytecode: {main_bytecode}\n\
         Bytecode-Format-Version: {FORMAT_VERSION}\n"
    )
}

/// 把入口 `.lg` 编译并连同资源打成 `.lgpack`（ZIP）。
///
/// - `main_lg`：入口源码（可含 import，loader 会合并）
/// - `output`：输出包路径，如 `app.lgpack`
/// - `resources`：额外打进包里的文件；路径保持文件名或相对路径
/// - `entry_name`：包内字节码文件名，默认 `app.lgb`
/// 打包 .lgpack（类 JAR）。步骤：
/// 1. 编译入口 .lg（import 已合并）→ 得到 Module
/// 2. encode 成 .lgb 字节
/// 3. 写 ZIP：清单 META-INF/MANIFEST.MF + 入口 app.lgb + resources/*
pub fn pack_program(
    main_lg: &Path,
    output: &Path,
    resources: &[std::path::PathBuf],
    entry_name: &str,
) -> Result<(), PackError> {
    let module =
        compile_file(&main_lg.display().to_string()).map_err(|e| err(format!("编译失败: {e}")))?;
    let lgb = encode_module(&module).map_err(|e| err(e.message))?;
    let manifest = build_manifest(entry_name);

    let file = std::fs::File::create(output)
        .map_err(|e| err(format!("cannot create {}: {e}", output.display())))?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file(MANIFEST_PATH, opts)
        .map_err(|e| err(format!("zip: {e}")))?;
    zip.write_all(manifest.as_bytes())
        .map_err(|e| err(format!("zip: {e}")))?;

    zip.start_file(entry_name, opts)
        .map_err(|e| err(format!("zip: {e}")))?;
    zip.write_all(&lgb).map_err(|e| err(format!("zip: {e}")))?;

    for res in resources {
        let data = std::fs::read(res)
            .map_err(|e| err(format!("cannot read resource {}: {e}", res.display())))?;
        // 包内路径：使用文件名（教学包不做深层目录映射）
        let name = res
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .ok_or_else(|| err(format!("bad resource path {}", res.display())))?;
        let inner = format!("resources/{name}");
        zip.start_file(&inner, opts)
            .map_err(|e| err(format!("zip: {e}")))?;
        zip.write_all(&data).map_err(|e| err(format!("zip: {e}")))?;
    }

    zip.finish().map_err(|e| err(format!("zip finish: {e}")))?;
    Ok(())
}

/// 从 `.lgpack` 读出清单与入口 `.lgb` 字节。
/// 打开 ZIP 包：读清单 → 找到 Main-Bytecode 指向的条目 → 取出 .lgb 字节
pub fn read_package(path: &Path) -> Result<(PackMeta, Vec<u8>), PackError> {
    let file = std::fs::File::open(path)
        .map_err(|e| err(format!("cannot open {}: {e}", path.display())))?;
    let mut zip = ZipArchive::new(file).map_err(|e| err(format!("not a zip/lgpack: {e}")))?;

    let manifest = {
        let mut f = zip
            .by_name(MANIFEST_PATH)
            .map_err(|_| err(format!("missing {MANIFEST_PATH} in package")))?;
        let mut s = String::new();
        f.read_to_string(&mut s)
            .map_err(|e| err(format!("read manifest: {e}")))?;
        s
    };
    let meta = parse_manifest(&manifest);

    let bytecode = {
        let mut f = zip
            .by_name(&meta.main_bytecode)
            .map_err(|e| err(format!("missing `{}` in package: {e}", meta.main_bytecode)))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)
            .map_err(|e| err(format!("read bytecode: {e}")))?;
        buf
    };
    Ok((meta, bytecode))
}

/// 执行 `.lgpack`。
/// 执行 .lgpack：读包 → 校验包版本 → 解码字节码 → VM 执行
pub fn exec_package(path: &Path) -> Result<Vec<String>, String> {
    let (meta, bytes) = read_package(path).map_err(|e| e.to_string())?;
    if meta.package_version != PACKAGE_VERSION {
        return Err(format!(
            "unsupported lgpack version {} (this build supports {PACKAGE_VERSION})",
            meta.package_version
        ));
    }
    let module = decode_module(&bytes).map_err(|e| e.to_string())?;
    exec_module(&module)
}

/// 反汇编包内入口字节码。
pub fn disasm_package(path: &Path) -> Result<String, PackError> {
    let (meta, bytes) = read_package(path)?;
    let module = decode_module(&bytes).map_err(|e| err(e.message))?;
    let mut out = format!(
        "; lgpack {}\n; Main-Bytecode: {}\n; package version: {}\n\n",
        path.display(),
        meta.main_bytecode,
        meta.package_version
    );
    out.push_str(&crate::codegen::lgb::disassemble_module(&module));
    Ok(out)
}

/// 列出包内条目（文件名），供 `pack --list` / 教学查看。
pub fn list_package(path: &Path) -> Result<Vec<String>, PackError> {
    let file = std::fs::File::open(path)
        .map_err(|e| err(format!("cannot open {}: {e}", path.display())))?;
    let mut zip = ZipArchive::new(file).map_err(|e| err(format!("not a zip/lgpack: {e}")))?;
    let mut names = Vec::new();
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| err(e.to_string()))?;
        names.push(f.name().to_string());
    }
    Ok(names)
}

pub fn is_lgpack(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("lgpack") || e.eq_ignore_ascii_case("jar"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_roundtrip() {
        let m = build_manifest("main.lgb");
        let meta = parse_manifest(&m);
        assert_eq!(meta.main_bytecode, "main.lgb");
        assert_eq!(meta.package_version, PACKAGE_VERSION);
    }

    #[test]
    fn pack_and_exec_fib() {
        let out = Path::new("target/fib_test.lgpack");
        std::fs::create_dir_all("target").ok();
        pack_program(Path::new("examples/fib.lg"), out, &[], "app.lgb").expect("pack");
        let out_lines = exec_package(out).expect("exec");
        assert_eq!(out_lines, vec!["55"]);
        let names = list_package(out).expect("list");
        assert!(names.iter().any(|n| n == MANIFEST_PATH));
        assert!(names.iter().any(|n| n == "app.lgb"));
        let _ = std::fs::remove_file(out);
    }
}
