//! TESTCore `Util/Utility.cs` 중 부하 클라이언트가 쓰는 파일 함수(`GetETag`, `CheckFile`,
//! `CreateParents`, `CreateRandomFile`).

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use md5::{Digest, Md5};
use rand::Rng;
use tracing::error;

const MIB: usize = 1024 * 1024;
/// 원본 `TEXT_ALL`.
const TEXT_ALL: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ&$@=;:+,?!\\{}^%`[]\"<>~#|/ -_.()*0123456789";

/// 파일 MD5를 스트리밍으로 계산한다(파일 전체를 메모리에 올리지 않는다).
fn file_md5(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut md5 = Md5::new();
    let mut buffer = vec![0u8; 81920];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        md5.update(&buffer[..read]);
    }
    Ok(md5.finalize().to_vec())
}

/// 원본 `GetETag(string fileName)`: 파일 MD5의 소문자 16진수.
pub fn file_etag(path: impl AsRef<Path>) -> io::Result<String> {
    Ok(hex::encode(file_md5(path)?))
}

/// 원본 `GetMD5(string fileName)`: 파일 MD5의 Base64.
pub fn file_md5_base64(path: impl AsRef<Path>) -> io::Result<String> {
    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(file_md5(path)?))
}

/// 데이터 MD5의 소문자 16진수(원본 `GetETag(GetObjectResponse)`).
pub fn bytes_etag(data: &[u8]) -> String {
    hex::encode(Md5::digest(data))
}

/// 원본 `CheckFile`: 크기가 같으면 `true`, 다르면 지우고 `false`. 오류는 로그를 남기고 `false`.
pub fn check_file(path: &Path, size: i64) -> bool {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {
            if meta.len() as i64 == size {
                return true;
            }
            if let Err(e) = fs::remove_file(path) {
                error!("{}", io_exception(&e));
            }
            false
        }
        Ok(_) => false,
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => {
            error!("{}", io_exception(&e));
            false
        }
    }
}

/// 원본 `CreateParents`.
pub fn create_parents(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::create_dir_all(parent),
        _ => Ok(()),
    }
}

/// 원본 `CreateRandomFile(filePath, fileSize, isOverwrite)`: `TEXT_ALL`에서 고른 1MiB 버퍼를 반복해 쓴다.
pub fn create_random_file(path: &Path, file_size: i64, is_overwrite: bool) -> bool {
    let result = (|| -> io::Result<bool> {
        if check_file(path, file_size) && !is_overwrite {
            return Ok(true);
        }
        create_parents(path)?;
        let mut rng = rand::rng();
        let buffer: Vec<u8> = (0..MIB)
            .map(|_| TEXT_ALL[rng.random_range(0..TEXT_ALL.len())])
            .collect();
        let mut file = io::BufWriter::new(fs::File::create(path)?);
        let mut written: i64 = 0;
        while written < file_size {
            let length = (buffer.len() as i64).min(file_size - written) as usize;
            file.write_all(&buffer[..length])?;
            written += buffer.len() as i64;
        }
        file.flush()?;
        Ok(true)
    })();
    match result {
        Ok(done) => done,
        Err(e) => {
            error!("{}", io_exception(&e));
            false
        }
    }
}

/// `log.Error(exception)`에 해당하는 문구(형식 이름과 메시지만, 스택 추적은 없다).
fn io_exception(error: &io::Error) -> String {
    format!("System.IO.IOException: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_file_size_and_charset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/file.bin");
        assert!(create_random_file(&path, 1_500_000, false));
        let data = fs::read(&path).unwrap();
        assert_eq!(data.len(), 1_500_000);
        assert!(data.iter().all(|b| TEXT_ALL.contains(b)));
        // 크기가 같으면 덮어쓰지 않는다.
        assert!(check_file(&path, 1_500_000));
        assert!(!check_file(&path, 10));
        assert!(!path.exists());
    }

    #[test]
    fn etag() {
        assert_eq!(bytes_etag(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }
}
