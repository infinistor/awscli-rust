//! `ListObjectVersions` 응답의 문서 순서.
//!
//! .NET SDK는 `<Version>`과 `<DeleteMarker>`를 응답 문서 순서대로 한 목록(`ListVersionsResponse.Versions`,
//! 삭제 마커는 `IsDeleteMarker = true`)에 담는다. Rust SDK는 둘을 따로 담아 순서를 잃으므로, 응답 본문에서
//! 두 요소의 순서를 기록해 두었다가 [`ListVersions::entries`]로 되살린다.

use std::ops::Deref;
use std::sync::{Arc, Mutex};

use aws_sdk_s3::config::interceptors::AfterDeserializationInterceptorContextRef;
use aws_sdk_s3::config::{ConfigBag, Intercept, RuntimeComponents};
use aws_sdk_s3::error::BoxError;
use aws_sdk_s3::operation::list_object_versions::ListObjectVersionsOutput;
use aws_sdk_s3::primitives::DateTime;
use aws_sdk_s3::types::{DeleteMarkerEntry, ObjectVersion, Owner};
use quick_xml::Reader;
use quick_xml::events::Event;

/// `ListVersions` 결과: SDK 출력(`Deref`)과 버전·삭제 마커의 문서 순서.
#[derive(Debug, Clone)]
pub struct ListVersions {
    output: ListObjectVersionsOutput,
    /// 루트 바로 아래 `Version`(false)·`DeleteMarker`(true)의 순서.
    order: Vec<bool>,
}

impl Deref for ListVersions {
    type Target = ListObjectVersionsOutput;

    fn deref(&self) -> &Self::Target {
        &self.output
    }
}

impl ListVersions {
    pub fn new(output: ListObjectVersionsOutput, order: Vec<bool>) -> Self {
        Self { output, order }
    }

    pub fn into_inner(self) -> ListObjectVersionsOutput {
        self.output
    }

    /// .NET `Versions`: 버전과 삭제 마커를 문서 순서로. 둘 다 없으면 `None`(.NET `null`).
    pub fn entries(&self) -> Option<Vec<VersionEntry<'_>>> {
        if self.output.versions.is_none() && self.output.delete_markers.is_none() {
            return None;
        }
        let mut versions = self.output.versions().iter();
        let mut markers = self.output.delete_markers().iter();
        let mut entries = Vec::with_capacity(versions.len() + markers.len());
        for &is_marker in &self.order {
            let next = if is_marker {
                markers.next().map(VersionEntry::DeleteMarker)
            } else {
                versions.next().map(VersionEntry::Version)
            };
            entries.extend(next);
        }
        // 기록과 개수가 맞지 않으면(본문을 읽지 못한 경우 등) 나머지를 버전, 삭제 마커 순으로 붙인다.
        entries.extend(versions.map(VersionEntry::Version));
        entries.extend(markers.map(VersionEntry::DeleteMarker));
        Some(entries)
    }
}

/// .NET `S3ObjectVersion` 한 항목(버전이나 삭제 마커).
#[derive(Debug, Clone, Copy)]
pub enum VersionEntry<'a> {
    Version(&'a ObjectVersion),
    DeleteMarker(&'a DeleteMarkerEntry),
}

impl<'a> VersionEntry<'a> {
    pub fn is_delete_marker(&self) -> bool {
        matches!(self, Self::DeleteMarker(_))
    }

    pub fn key(&self) -> Option<&'a str> {
        match self {
            Self::Version(v) => v.key(),
            Self::DeleteMarker(m) => m.key(),
        }
    }

    pub fn version_id(&self) -> Option<&'a str> {
        match self {
            Self::Version(v) => v.version_id(),
            Self::DeleteMarker(m) => m.version_id(),
        }
    }

    pub fn is_latest(&self) -> Option<bool> {
        match self {
            Self::Version(v) => v.is_latest(),
            Self::DeleteMarker(m) => m.is_latest(),
        }
    }

    pub fn last_modified(&self) -> Option<&'a DateTime> {
        match self {
            Self::Version(v) => v.last_modified(),
            Self::DeleteMarker(m) => m.last_modified(),
        }
    }

    pub fn owner(&self) -> Option<&'a Owner> {
        match self {
            Self::Version(v) => v.owner(),
            Self::DeleteMarker(m) => m.owner(),
        }
    }

    /// 삭제 마커에는 없다.
    pub fn size(&self) -> Option<i64> {
        match self {
            Self::Version(v) => v.size(),
            Self::DeleteMarker(_) => None,
        }
    }

    /// 삭제 마커에는 없다.
    pub fn e_tag(&self) -> Option<&'a str> {
        match self {
            Self::Version(v) => v.e_tag(),
            Self::DeleteMarker(_) => None,
        }
    }

    pub fn version(&self) -> Option<&'a ObjectVersion> {
        match self {
            Self::Version(v) => Some(v),
            Self::DeleteMarker(_) => None,
        }
    }
}

/// 응답 본문에서 루트 바로 아래 `Version`·`DeleteMarker` 요소의 순서를 기록한다.
#[derive(Debug, Default)]
pub(crate) struct VersionOrder {
    order: Arc<Mutex<Vec<bool>>>,
}

impl VersionOrder {
    pub(crate) fn slot(&self) -> Arc<Mutex<Vec<bool>>> {
        self.order.clone()
    }
}

impl Intercept for VersionOrder {
    fn name(&self) -> &'static str {
        "VersionOrder"
    }

    fn read_after_deserialization(
        &self,
        context: &AfterDeserializationInterceptorContextRef<'_>,
        _runtime_components: &RuntimeComponents,
        _cfg: &mut ConfigBag,
    ) -> Result<(), BoxError> {
        let order = context
            .response()
            .body()
            .bytes()
            .map(document_order)
            .unwrap_or_default();
        *self.order.lock().unwrap() = order;
        Ok(())
    }
}

/// 루트의 자식 중 `Version`(false)·`DeleteMarker`(true)의 순서. 해석 오류가 나면 그때까지 읽은 것.
fn document_order(body: &[u8]) -> Vec<bool> {
    let mut reader = Reader::from_reader(body);
    let mut depth = 0usize;
    let mut order = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if depth == 1 {
                    match e.local_name().into_inner() {
                        "Version" => order.push(false),
                        "DeleteMarker" => order.push(true),
                        _ => {}
                    }
                }
                depth += 1;
            }
            Ok(Event::Empty(e)) if depth == 1 => match e.local_name().into_inner() {
                "Version" => order.push(false),
                "DeleteMarker" => order.push(true),
                _ => {}
            },
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_document_order() {
        let body = br#"<?xml version="1.0"?><ListVersionsResult xmlns="x"><Name>b</Name><DeleteMarker><Key>z</Key></DeleteMarker><Version><Key>a</Key><Owner><ID>o</ID></Owner></Version><DeleteMarker><Key>a</Key></DeleteMarker><Version><Key>b</Key></Version></ListVersionsResult>"#;
        assert_eq!(document_order(body), vec![true, false, true, false]);
        assert!(document_order(b"").is_empty());
    }

    #[test]
    fn entries_follow_order() {
        let version = |k: &str| ObjectVersion::builder().key(k).build();
        let marker = |k: &str| DeleteMarkerEntry::builder().key(k).build();
        let output = ListObjectVersionsOutput::builder()
            .versions(version("a"))
            .versions(version("b"))
            .delete_markers(marker("z"))
            .delete_markers(marker("a"))
            .build();
        let list = ListVersions::new(output.clone(), vec![true, false, true, false]);
        let keys: Vec<(bool, &str)> = list
            .entries()
            .unwrap()
            .iter()
            .map(|e| (e.is_delete_marker(), e.key().unwrap()))
            .collect();
        assert_eq!(keys, [(true, "z"), (false, "a"), (true, "a"), (false, "b")]);
        // 기록이 없으면 버전, 삭제 마커 순.
        let unordered = ListVersions::new(output, Vec::new());
        let keys: Vec<&str> = unordered
            .entries()
            .unwrap()
            .iter()
            .map(|e| e.key().unwrap())
            .collect();
        assert_eq!(keys, ["a", "b", "z", "a"]);
        let empty = ListVersions::new(ListObjectVersionsOutput::builder().build(), Vec::new());
        assert!(empty.entries().is_none());
    }
}
