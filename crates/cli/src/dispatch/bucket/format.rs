//! 버킷·ACL·멀티파트 메뉴가 같이 쓰는 출력·오류 도우미.
//!
//! (시각 서식은 `output::invariant_time`·`ko_kr_time`, S3 오류 형식은 `CommandError::s3`로 옮겼다.)

pub(in crate::dispatch) use crate::dispatch::input::{json_error, null_reference, read_all_text};
