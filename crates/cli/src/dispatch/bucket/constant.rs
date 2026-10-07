//! AWS SDK `ConstantClass`(`S3CannedACL`, `ObjectOwnership`, `VersionStatus`)의 `FindValue`.
//!
//! `FindValue(value)`는 알려진 상수를 대소문자 구분 없이 찾아 그 상수의 정식 값을 돌려주고,
//! 모르는 값이면 입력을 그대로 새 상수로 만든다.

/// `S3CannedACL`의 알려진 값.
const CANNED_ACL: &[&str] = &[
    "private",
    "public-read",
    "public-read-write",
    "authenticated-read",
    "aws-exec-read",
    "bucket-owner-read",
    "bucket-owner-full-control",
    "log-delivery-write",
];

/// `ObjectOwnership`의 알려진 값.
const OBJECT_OWNERSHIP: &[&str] = &[
    "BucketOwnerPreferred",
    "ObjectWriter",
    "BucketOwnerEnforced",
];

/// `VersionStatus`의 알려진 값.
const VERSION_STATUS: &[&str] = &["Off", "Enabled", "Suspended"];

fn find_value(value: &str, known: &[&'static str]) -> String {
    known
        .iter()
        .find(|k| k.eq_ignore_ascii_case(value))
        .map_or_else(|| value.to_string(), |k| (*k).to_string())
}

/// `S3CannedACL.FindValue`.
pub(in crate::dispatch) fn canned_acl(value: &str) -> String {
    find_value(value, CANNED_ACL)
}

/// `ObjectOwnership.FindValue`.
pub(in crate::dispatch) fn object_ownership(value: &str) -> String {
    find_value(value, OBJECT_OWNERSHIP)
}

/// `VersionStatus.FindValue`.
pub(in crate::dispatch) fn version_status(value: &str) -> String {
    find_value(value, VERSION_STATUS)
}

/// `string.CompareTo`(현재 문화권 비교)의 ASCII 부분.
///
/// .NET은 ICU 기본 정렬(CLDR 루트)을 쓴다. 공백 < `_ - , ; : ! ? . ' " ( ) [ ] { } @ * / \ & # % ` ^ + < = > | ~ $`
/// < 숫자 < 알파벳 순으로 1차(문자) 비교를 하고, 1차가 모두 같으면 첫 번째로 대소문자가 다른 자리에서
/// 소문자가 앞선다. 버킷 이름은 ASCII이므로 ASCII만 옮겼고, 그 밖의 문자가 있으면 UTF-16 서수 비교로 대신한다
/// (원본과 다를 수 있다).
pub(in crate::dispatch) fn culture_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    const SYMBOLS: &str = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    let rank = |c: char| -> Option<u32> {
        if let Some(i) = SYMBOLS.find(c) {
            Some(i as u32)
        } else if c.is_ascii_digit() {
            Some(100 + c as u32 - '0' as u32)
        } else if c.is_ascii_alphabetic() {
            Some(200 + c.to_ascii_lowercase() as u32 - 'a' as u32)
        } else {
            None
        }
    };
    let ranks = |s: &str| s.chars().map(rank).collect::<Option<Vec<u32>>>();
    let (Some(ra), Some(rb)) = (ranks(a), ranks(b)) else {
        return a.encode_utf16().cmp(b.encode_utf16());
    };
    match ra.cmp(&rb) {
        Ordering::Equal => {}
        other => return other,
    }
    // 1차가 같으면 길이도 같다. 첫 대소문자 차이에서 소문자가 앞선다.
    for (x, y) in a.chars().zip(b.chars()) {
        if x != y {
            return if x.is_ascii_lowercase() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_value_is_case_insensitive() {
        assert_eq!(canned_acl("PUBLIC-READ"), "public-read");
        assert_eq!(canned_acl("custom"), "custom");
        assert_eq!(object_ownership("objectwriter"), "ObjectWriter");
        assert_eq!(version_status("suspended"), "Suspended");
    }

    #[test]
    fn culture_order() {
        let mut names = [
            "B2", "ab", "a-b", "A-b", "a-B", "a1", "AB", "a_b", "b1", "a", "A",
        ];
        names.sort_by(|x, y| culture_compare(x, y));
        assert_eq!(
            names,
            [
                "a", "A", "a_b", "a-b", "a-B", "A-b", "a1", "ab", "AB", "b1", "B2"
            ]
        );
    }
}
