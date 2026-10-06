//! TESTCore `Util/INIParser.cs`(`IniFile`, `IniSection`, `IniValue`) 이식.
//!
//! TESTCore가 실제로 쓰는 읽기 기능만 옮겼다. 원본과 같게 동작하도록 맞춘 규칙:
//!
//! - 줄 구분은 `\r\n`, `\n`, `\r` 모두 허용한다(`StreamReader.ReadLine`).
//! - 앞 공백을 뺀 줄이 `[`로 시작하고 `]`가 있으면 섹션이다. 이름은 `[`와 첫 `]` 사이를 trim한 값이고,
//!   `]` 뒤의 내용은 무시한다. `]`가 없으면 그 줄을 무시하고 이전 섹션이 계속된다.
//! - 같은 이름의 섹션이 다시 나오면 앞 섹션의 값을 모두 버리고 새 섹션으로 바꾼다. 순서상 위치는 처음 자리를 유지한다.
//! - 주석은 `;`로 시작하는 줄뿐이다. `#`은 주석이 아니다(`=`가 있으면 키로 읽힌다).
//! - `key=value`는 원본 줄의 첫 `=` 위치로 나눈다. `=`가 줄 맨 앞이면 무시하지만,
//!   앞에 공백만 있으면 빈 문자열 키가 된다. 키는 trim하고 값은 원본 그대로 보관한다.
//! - 같은 섹션의 같은 키는 나중 값이 이긴다(위치는 처음 자리).
//! - 섹션·키 이름은 대소문자를 구분한다. 원본 비교자 이름은 `CaseInsensitiveStringComparer`지만
//!   `string.CompareTo`로 비교해 실제로는 대소문자를 구분한다.
//! - 파일 인코딩은 BOM으로 UTF-8/UTF-16을 판별하고, 없으면 UTF-8로 읽는다. 잘못된 바이트는 U+FFFD로 바꾼다.

use std::io;
use std::path::{Path, PathBuf};

/// INI 파일을 읽을 때 생기는 오류.
#[derive(Debug, thiserror::Error)]
pub enum IniError {
    /// 파일(또는 상위 디렉터리)이 없다. TESTCore `FileNotFoundException`에 대응한다.
    #[error("파일을 찾을 수 없습니다: {}", .0.display())]
    NotFound(PathBuf),
    #[error("파일을 읽을 수 없습니다: {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// INI 값 하나. 없는 키를 조회하면 값이 없는(`None`) 기본값이 돌아온다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IniValue(Option<String>);

impl IniValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Some(value.into()))
    }

    /// 파일에 적힌 그대로의 값(`=` 뒤 전체, 공백 포함).
    pub fn raw(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// 원본 `IniValue.ToString()`: 값이 없거나 공백뿐이면 빈 문자열, 아니면 앞뒤 공백을 제거한 값.
    pub fn text(&self) -> &str {
        self.0.as_deref().map(str::trim).unwrap_or("")
    }
}

/// 섹션 하나. 키 순서는 처음 나온 순서를 유지한다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IniSection {
    values: Vec<(String, IniValue)>,
}

impl IniSection {
    /// 키가 있으면 값을 돌려준다(원본 `TryGetValue`).
    pub fn get(&self, key: &str) -> Option<&IniValue> {
        self.values.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// 원본 인덱서 `section[key]`의 `ToString()` 결과. 없는 키는 빈 문자열.
    pub fn text(&self, key: &str) -> &str {
        self.get(key).map(IniValue::text).unwrap_or("")
    }

    /// 값을 넣는다. 같은 키가 있으면 위치는 두고 값만 바꾼다.
    pub fn insert(&mut self, key: impl Into<String>, value: IniValue) {
        let key = key.into();
        match self.values.iter_mut().find(|(k, _)| *k == key) {
            Some((_, slot)) => *slot = value,
            None => self.values.push((key, value)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &IniValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// INI 파일 전체. 섹션 순서는 처음 나온 순서를 유지한다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IniFile {
    sections: Vec<(String, IniSection)>,
}

impl IniFile {
    /// 파일을 읽어 파싱한다.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, IniError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| match source.kind() {
            io::ErrorKind::NotFound => IniError::NotFound(path.to_path_buf()),
            _ => IniError::Io {
                path: path.to_path_buf(),
                source,
            },
        })?;
        Ok(Self::from_bytes(&bytes))
    }

    /// BOM으로 인코딩을 판별해 파싱한다.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::parse(&decode(bytes))
    }

    /// 문자열을 파싱한다.
    pub fn parse(text: &str) -> Self {
        let mut ini = Self::default();
        let mut current: Option<usize> = None;
        for line in lines(text) {
            let trim_start = line.trim_start();
            if trim_start.is_empty() {
                continue;
            }
            if let Some(rest) = trim_start.strip_prefix('[') {
                if let Some(end) = rest.find(']') {
                    current = Some(ini.replace_section(rest[..end].trim()));
                }
            } else if let Some(index) = current
                && !trim_start.starts_with(';')
                && let Some((key, value)) = parse_value(line)
            {
                ini.sections[index].1.insert(key, IniValue::new(value));
            }
        }
        ini
    }

    /// 같은 이름의 섹션을 빈 섹션으로 바꾸거나 새로 추가하고 위치를 돌려준다.
    fn replace_section(&mut self, name: &str) -> usize {
        match self.sections.iter().position(|(n, _)| n == name) {
            Some(index) => {
                self.sections[index].1 = IniSection::default();
                index
            }
            None => {
                self.sections
                    .push((name.to_string(), IniSection::default()));
                self.sections.len() - 1
            }
        }
    }

    /// 섹션이 있으면 돌려준다(원본 `TryGetSection`).
    pub fn section(&self, name: &str) -> Option<&IniSection> {
        self.sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s)
    }

    pub fn contains_section(&self, name: &str) -> bool {
        self.section(name).is_some()
    }

    /// 원본 `ini[section][key].ToString()`. 섹션이나 키가 없으면 빈 문자열.
    pub fn text(&self, section: &str, key: &str) -> &str {
        self.section(section).map(|s| s.text(key)).unwrap_or("")
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &IniSection)> {
        self.sections.iter().map(|(n, s)| (n.as_str(), s))
    }

    pub fn len(&self) -> usize {
        self.sections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
}

/// 원본 `LoadValue`: 첫 `=` 기준으로 나누고, `=`가 맨 앞이거나 없으면 실패.
fn parse_value(line: &str) -> Option<(&str, &str)> {
    match line.find('=') {
        Some(index) if index > 0 => Some((line[..index].trim(), &line[index + 1..])),
        _ => None,
    }
}

/// `StreamReader.ReadLine`처럼 `\r\n`, `\n`, `\r`을 모두 줄 끝으로 본다.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(text).filter(|t| !t.is_empty());
    std::iter::from_fn(move || {
        let current = rest?;
        match current.find(['\r', '\n']) {
            Some(index) => {
                let skip = if current[index..].starts_with("\r\n") {
                    2
                } else {
                    1
                };
                rest = Some(&current[index + skip..]).filter(|t| !t.is_empty());
                Some(&current[..index])
            }
            None => {
                rest = None;
                Some(current)
            }
        }
    })
}

/// `StreamReader` 기본 동작처럼 BOM으로 UTF-8/UTF-16을 판별한다.
fn decode(bytes: &[u8]) -> String {
    fn utf16(bytes: &[u8], to_u16: fn([u8; 2]) -> u16) -> String {
        let units: Vec<u16> = bytes
            .chunks(2)
            .map(|pair| match *pair {
                [a, b] => to_u16([a, b]),
                _ => 0xFFFD,
            })
            .collect();
        String::from_utf16_lossy(&units)
    }
    if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        String::from_utf8_lossy(rest).into_owned()
    } else if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        utf16(rest, u16::from_le_bytes)
    } else if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        utf16(rest, u16::from_be_bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_endings() {
        let ini = IniFile::parse("[a]\r\nx=1\ry=2\nz=3");
        let a = ini.section("a").unwrap();
        assert_eq!(
            a.iter().map(|(k, v)| (k, v.text())).collect::<Vec<_>>(),
            [("x", "1"), ("y", "2"), ("z", "3")]
        );
    }

    #[test]
    fn missing_values_are_empty() {
        let ini = IniFile::parse("[a]\nx=  \n");
        assert_eq!(ini.text("a", "x"), "");
        assert_eq!(ini.text("a", "missing"), "");
        assert_eq!(ini.text("missing", "x"), "");
        assert_eq!(
            ini.section("a").unwrap().get("x").unwrap().raw(),
            Some("  ")
        );
    }

    #[test]
    fn keys_before_any_section_are_ignored() {
        let ini = IniFile::parse("x=1\n[a]\n");
        assert_eq!(ini.len(), 1);
        assert!(ini.section("a").unwrap().is_empty());
    }

    #[test]
    fn bom_detection() {
        assert_eq!(
            IniFile::from_bytes(b"\xEF\xBB\xBF[a]\nx=1").text("a", "x"),
            "1"
        );
        let utf16: Vec<u8> = "\u{FEFF}[a]\nx=한글"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(IniFile::from_bytes(&utf16).text("a", "x"), "한글");
    }

    #[test]
    fn not_found() {
        assert!(matches!(
            IniFile::load("does/not/exist.ini"),
            Err(IniError::NotFound(_))
        ));
    }
}
