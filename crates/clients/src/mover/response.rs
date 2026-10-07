//! TESTCore `Mover/Response/*` 이식. 상속(`ResponseMoverStart : ResponseMover`)은 기본 속성을 함께 읽는 것으로 옮겼다.

use super::data::MoverStatus;
use crate::json::{Deserializer, FromJson, JsonError, Token};

/// Mover 기본 응답
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResponseMover {
    pub result: Option<String>,
    pub message: Option<String>,
}

impl ResponseMover {
    fn set_property(
        d: &mut Deserializer<'_>,
        object: &mut Self,
        name: &str,
    ) -> Result<bool, JsonError> {
        match name {
            "Result" => object.result = d.read_nullable()?,
            "Message" => object.message = d.read_nullable()?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl FromJson for ResponseMover {
    fn type_name() -> String {
        "TestCore.Mover.Response.ResponseMover".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        d.read_object(tok, &Self::type_name(), Self::default(), Self::set_property)
    }
}

/// Mover 시작 응답
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResponseMoverStart {
    pub base: ResponseMover,
    pub job_id: i32,
}

impl FromJson for ResponseMoverStart {
    fn type_name() -> String {
        "TestCore.Mover.Response.ResponseMoverStart".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        d.read_object(tok, &Self::type_name(), Self::default(), |d, o, name| {
            if name == "JobId" {
                o.job_id = d.read_value()?;
                return Ok(true);
            }
            ResponseMover::set_property(d, &mut o.base, name)
        })
    }
}

/// Mover 상태 응답. `Items`가 JSON `null`이면 `None`(원본은 이후 `foreach`에서 `NullReferenceException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseMoverStatus {
    pub base: ResponseMover,
    pub items: Option<Vec<Option<MoverStatus>>>,
}

impl Default for ResponseMoverStatus {
    /// 원본 기본값은 빈 목록(`Items = []`)
    fn default() -> Self {
        Self {
            base: ResponseMover::default(),
            items: Some(Vec::new()),
        }
    }
}

impl FromJson for ResponseMoverStatus {
    fn type_name() -> String {
        "TestCore.Mover.Response.ResponseMoverStatus".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        d.read_object(tok, &Self::type_name(), Self::default(), |d, o, name| {
            if name == "Items" {
                let tok = d.read_token()?;
                o.items = d.read_list::<MoverStatus>(tok)?;
                return Ok(true);
            }
            ResponseMover::set_property(d, &mut o.base, name)
        })
    }
}
