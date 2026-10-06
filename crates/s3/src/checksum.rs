//! TESTCore `Util/ChecksumCalculator.cs`, `Util/CRC64NVMe.cs`, `Util/S3ChecksumAlgorithm.cs` 이식.
//!
//! 결과는 S3 `x-amz-checksum-*` 헤더와 같은 형식이다. CRC 값은 빅엔디언 바이트, SHA 값은 다이제스트
//! 바이트를 Base64로 인코딩한다.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use crc::{CRC_32_ISCSI, CRC_32_ISO_HDLC, CRC_64_NVME, Crc};
use sha1::Sha1;
use sha2::{Digest, Sha256};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
const CRC32C: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);
const CRC64NVME: Crc<u64> = Crc::<u64>::new(&CRC_64_NVME);

/// 원본과 같은 4KiB 단위로 읽는다.
const BUFFER_SIZE: usize = 4096;

/// TESTCore `S3ChecksumAlgorithm`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ChecksumAlgorithm {
    #[default]
    None,
    Crc32,
    Crc32c,
    Crc64Nvme,
    Sha1,
    Sha256,
}

impl ChecksumAlgorithm {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Crc32,
        Self::Crc32c,
        Self::Crc64Nvme,
        Self::Sha1,
        Self::Sha256,
    ];

    /// 원본 `ToAlgorithm`: 대문자 또는 소문자 이름만 인식하고 나머지는 `None`.
    pub fn from_name(name: &str) -> Self {
        match name {
            "CRC32" | "crc32" => Self::Crc32,
            "CRC32C" | "crc32c" => Self::Crc32c,
            "CRC64NVME" | "crc64nvme" => Self::Crc64Nvme,
            "SHA1" | "sha1" => Self::Sha1,
            "SHA256" | "sha256" => Self::Sha256,
            _ => Self::None,
        }
    }

    /// 원본 `ToName`.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Crc32 => "CRC32",
            Self::Crc32c => "CRC32C",
            Self::Crc64Nvme => "CRC64NVME",
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
        }
    }
}

impl fmt::Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChecksumError {
    /// 원본 `FileNotFoundException("파일을 찾을 수 없습니다.", filePath)`.
    #[error("파일을 찾을 수 없습니다.")]
    NotFound(PathBuf),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// 원본 `CalculateChecksum`: 파일의 체크섬을 Base64 문자열로 계산한다. `None`이면 빈 문자열.
pub fn calculate_checksum(
    path: impl AsRef<Path>,
    algorithm: ChecksumAlgorithm,
) -> Result<String, ChecksumError> {
    if algorithm == ChecksumAlgorithm::None {
        return Ok(String::new());
    }
    let path = path.as_ref();
    // File.Exists는 디렉터리에 대해서도 false다.
    if !path.is_file() {
        return Err(ChecksumError::NotFound(path.to_path_buf()));
    }
    Ok(checksum_reader(File::open(path)?, algorithm)?)
}

/// 스트림 전체의 체크섬을 Base64 문자열로 계산한다. `None`이면 빈 문자열.
pub fn checksum_reader(reader: impl Read, algorithm: ChecksumAlgorithm) -> io::Result<String> {
    let bytes = match algorithm {
        ChecksumAlgorithm::None => return Ok(String::new()),
        ChecksumAlgorithm::Crc32 => {
            let mut digest = CRC32.digest();
            each_chunk(reader, |chunk| digest.update(chunk))?;
            digest.finalize().to_be_bytes().to_vec()
        }
        ChecksumAlgorithm::Crc32c => {
            let mut digest = CRC32C.digest();
            each_chunk(reader, |chunk| digest.update(chunk))?;
            digest.finalize().to_be_bytes().to_vec()
        }
        ChecksumAlgorithm::Crc64Nvme => {
            let mut digest = CRC64NVME.digest();
            each_chunk(reader, |chunk| digest.update(chunk))?;
            digest.finalize().to_be_bytes().to_vec()
        }
        ChecksumAlgorithm::Sha1 => {
            let mut digest = Sha1::new();
            each_chunk(reader, |chunk| digest.update(chunk))?;
            digest.finalize().to_vec()
        }
        ChecksumAlgorithm::Sha256 => {
            let mut digest = Sha256::new();
            each_chunk(reader, |chunk| digest.update(chunk))?;
            digest.finalize().to_vec()
        }
    };
    Ok(BASE64.encode(bytes))
}

/// 메모리에 있는 데이터의 체크섬을 Base64 문자열로 계산한다.
pub fn checksum_bytes(data: &[u8], algorithm: ChecksumAlgorithm) -> String {
    checksum_reader(data, algorithm).expect("메모리 읽기는 실패하지 않는다")
}

fn each_chunk(mut reader: impl Read, mut f: impl FnMut(&[u8])) -> io::Result<()> {
    let mut buffer = [0u8; BUFFER_SIZE];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(n) => f(&buffer[..n]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CRC 카탈로그의 표준 검사값("123456789")으로 알고리즘 매개변수를 확인한다.
    #[test]
    fn crc_check_values() {
        let input = b"123456789";
        assert_eq!(CRC32.checksum(input), 0xCBF4_3926);
        assert_eq!(CRC32C.checksum(input), 0xE306_9283);
        assert_eq!(CRC64NVME.checksum(input), 0xAE8B_1486_0A79_9888);
    }

    /// 원본 CRC64NVMe.cs의 테이블 구현(반사 다항식 0x9A6C9329AC4BC9B5, 초기값·최종 XOR 모두 1)과 비교한다.
    #[test]
    fn crc64_matches_original_table_algorithm() {
        let mut table = [0u64; 256];
        for (i, slot) in table.iter_mut().enumerate() {
            let mut crc = i as u64;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0x9A6C_9329_AC4B_C9B5
                } else {
                    crc >> 1
                };
            }
            *slot = crc;
        }
        let original = |data: &[u8]| {
            let mut crc = u64::MAX;
            for &b in data {
                crc = (crc >> 8) ^ table[((crc ^ b as u64) & 0xFF) as usize];
            }
            crc ^ u64::MAX
        };
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 31 % 251) as u8).collect();
        assert_eq!(CRC64NVME.checksum(&data), original(&data));
    }

    #[test]
    fn names_round_trip() {
        for algorithm in ChecksumAlgorithm::ALL {
            assert_eq!(ChecksumAlgorithm::from_name(algorithm.name()), algorithm);
        }
        assert_eq!(
            ChecksumAlgorithm::from_name("Crc32"),
            ChecksumAlgorithm::None
        );
        assert_eq!(
            ChecksumAlgorithm::from_name("crc32c"),
            ChecksumAlgorithm::Crc32c
        );
    }

    #[test]
    fn none_and_missing_file() {
        assert_eq!(
            calculate_checksum("missing", ChecksumAlgorithm::None).unwrap(),
            ""
        );
        assert!(matches!(
            calculate_checksum("missing", ChecksumAlgorithm::Crc32),
            Err(ChecksumError::NotFound(_))
        ));
    }
}
