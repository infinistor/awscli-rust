//! 파일 입출력 도우미: `File.ReadAllText`, `Utility.SaveFile`, `Utility.GetFileList`, `Utility.GetMD5*`.
//!
//! .NET 예외의 형식 이름과 메시지(전체 경로, `File name:` 줄)를 `CommandError`로 옮긴다.

use std::io;
use std::path::{Path, PathBuf};

use aws_sdk_s3::primitives::ByteStream;
use awscli_rest_s3::S3Error;
use base64::Engine;
use md5::{Digest, Md5};
use tokio::io::AsyncWriteExt;
use tracing::error;

use crate::dispatch::CommandError;

/// `Path.GetFullPath`.
pub(super) fn full_path(path: &str) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// 파일 입출력 오류를 .NET 예외로 바꾼다. `FileNotFoundException`은 `File name:` 줄이 붙는다.
pub(super) fn io_error(path: &Path, error: &io::Error) -> CommandError {
    let mapped = S3Error::io(path, error);
    let mut message = mapped.to_string();
    if mapped.dotnet_type() == "System.IO.FileNotFoundException" {
        message.push_str(&format!("\nFile name: '{}'", path.display()));
    }
    CommandError::new(mapped.dotnet_type(), message)
}

/// `File.Exists`: 디렉터리는 `false`.
pub(super) fn file_exists(path: &str) -> bool {
    Path::new(path).is_file()
}

/// `File.ReadAllText`: BOM으로 UTF-8/UTF-16을 판별하고 잘못된 바이트는 U+FFFD로 바꾼다.
pub(super) fn read_all_text(path: &str) -> Result<String, CommandError> {
    let full = full_path(path);
    let bytes = std::fs::read(&full).map_err(|e| io_error(&full, &e))?;
    Ok(decode_text(&bytes))
}

/// `StreamReader` 기본 인코딩 판별.
fn decode_text(bytes: &[u8]) -> String {
    let utf16 = |data: &[u8], little: bool| {
        let units: Vec<u16> = data
            .chunks(2)
            .map(|c| match (c, little) {
                ([a, b], true) => u16::from_le_bytes([*a, *b]),
                ([a, b], false) => u16::from_be_bytes([*a, *b]),
                _ => 0xFFFD,
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// `Utility.GetMD5(fileName)`: 파일 MD5의 Base64. 파일이 없으면 `FileNotFoundException`.
pub(super) fn file_md5_base64(path: &str) -> Result<String, CommandError> {
    let full = full_path(path);
    awscli_rest_clients::file_util::file_md5_base64(&full).map_err(|e| io_error(&full, &e))
}

/// `Utility.GetMD5FromString(content)`: UTF-8 바이트 MD5의 Base64.
pub(super) fn string_md5_base64(content: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(Md5::digest(content.as_bytes()))
}

/// `Utility.GetFileList(root, null)`: 하위 디렉터리를 먼저, 그다음 이 디렉터리의 파일을 담는다(전체 경로).
pub(super) fn file_list(root: &str) -> Result<Vec<String>, CommandError> {
    let mut list = Vec::new();
    collect_files(&full_path(root), &mut list)?;
    Ok(list)
}

fn collect_files(dir: &Path, list: &mut Vec<String>) -> Result<(), CommandError> {
    let entries = std::fs::read_dir(dir).map_err(|e| io_error(dir, &e))?;
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_error(dir, &e))?;
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
        } else {
            files.push(path);
        }
    }
    for directory in directories {
        collect_files(&directory, list)?;
    }
    list.extend(files.into_iter().map(|p| p.display().to_string()));
    Ok(())
}

/// `Path.GetDirectoryName`이 돌려주는 값. `null`은 `None`.
fn directory_name(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .map(|parent| parent.display().to_string())
}

/// `Utility.SaveFile(filePath, stream)`: 필요한 디렉터리를 만들고 덮어쓴다. 실패하면 예외를 로그로 남기고
/// `false`다(원본 `catch (Exception e) { log.Error(e); return false; }`).
///
/// 원본은 `Directory.CreateDirectory(Path.GetDirectoryName(filePath))`를 부르므로 디렉터리가 없는 상대 경로
/// (`out.txt`)는 `ArgumentException`으로 실패한다(원본 버그, 그대로 둔다).
pub(super) async fn save_file(path: Option<&str>, body: ByteStream) -> bool {
    match write_stream(path, body).await {
        Ok(()) => true,
        Err(e) => {
            error!("{e}");
            false
        }
    }
}

async fn write_stream(path: Option<&str>, mut body: ByteStream) -> Result<(), CommandError> {
    let path = path.unwrap_or("");
    match directory_name(path) {
        // `GetDirectoryName`이 null: `Directory.CreateDirectory(null)`
        None => {
            return Err(CommandError::new(
                "System.ArgumentNullException",
                "Value cannot be null. (Parameter 'path')",
            ));
        }
        // `GetDirectoryName`이 "": `Directory.CreateDirectory("")`
        Some(dir) if dir.is_empty() => {
            return Err(CommandError::new(
                "System.ArgumentException",
                "The value cannot be an empty string. (Parameter 'path')",
            ));
        }
        Some(dir) => {
            let dir = full_path(&dir);
            if !dir.is_dir() {
                std::fs::create_dir_all(&dir).map_err(|e| io_error(&dir, &e))?;
            }
        }
    }
    let full = full_path(path);
    let mut file = tokio::fs::File::create(&full)
        .await
        .map_err(|e| io_error(&full, &e))?;
    while let Some(chunk) = body.try_next().await.map_err(read_error)? {
        file.write_all(&chunk)
            .await
            .map_err(|e| io_error(&full, &e))?;
    }
    file.flush().await.map_err(|e| io_error(&full, &e))
}

/// 응답 본문을 읽다 실패한 경우.
pub(super) fn read_error(error: impl std::fmt::Display) -> CommandError {
    CommandError::new("System.IO.IOException", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_decoding_follows_bom() {
        assert_eq!(decode_text(b"\xEF\xBB\xBFabc"), "abc");
        assert_eq!(decode_text(b"\xFF\xFEa\0b\0"), "ab");
        assert_eq!(decode_text(b"\xFE\xFF\0a\0b"), "ab");
        assert_eq!(decode_text("한글".as_bytes()), "한글");
        assert_eq!(decode_text(b"a\xFFb"), "a\u{FFFD}b");
    }

    #[test]
    fn md5_of_string_is_base64() {
        // 원본 기준 출력(`put-object-body-lock-enable`)
        assert_eq!(string_md5_base64("locked body"), "muw8FM2HgWZtaRrOw2BpKA==");
    }

    #[test]
    fn directory_names() {
        assert_eq!(directory_name("out.txt").as_deref(), Some(""));
        assert_eq!(directory_name("dl/out.txt").as_deref(), Some("dl"));
        assert_eq!(directory_name(""), None);
    }

    #[test]
    fn file_list_puts_subdirectories_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("top.txt"), "t").unwrap();
        std::fs::write(dir.path().join("a/mid.txt"), "m").unwrap();
        std::fs::write(dir.path().join("a/b/deep.txt"), "d").unwrap();
        let list = file_list(&dir.path().display().to_string()).unwrap();
        let names: Vec<_> = list
            .iter()
            .map(|p| {
                Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["deep.txt", "mid.txt", "top.txt"]);
        assert!(file_list(&dir.path().join("none").display().to_string()).is_err());
    }
}
