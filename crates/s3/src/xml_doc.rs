//! .NET `XmlSerializer`로 읽던 응답을 옮기기 위한 작은 XML 트리.
//!
//! 오류 위치는 `XmlSerializer`의 `There is an error in XML document (줄, 칸).`과 맞추기 위해
//! 1부터 세는 (줄, 칸)으로 기록한다. 빈 문서는 (0, 0), 루트 앞의 잘못된 문자는 그 문자 위치,
//! 요소 관련 오류는 요소 이름의 위치(`<` 다음 칸)다. 깨진 XML의 세부 위치는 근사값이다.

use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;

/// (줄, 칸)
pub type Position = (usize, usize);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("There is an error in XML document ({}, {}).", .0.0, .0.1)]
pub struct XmlError(pub Position);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Element {
    /// 네임스페이스 URI(없으면 빈 문자열).
    pub ns: String,
    /// 접두사를 뺀 이름.
    pub name: String,
    /// 직접 포함한 텍스트·CDATA를 이어 붙인 값.
    pub text: String,
    pub children: Vec<Element>,
    /// 요소 이름의 위치.
    pub position: Position,
}

impl Element {
    pub fn is(&self, ns: &str, name: &str) -> bool {
        self.ns == ns && self.name == name
    }

    /// 이름과 네임스페이스가 같은 마지막 자식(`XmlSerializer`는 같은 요소가 여러 번 나오면 마지막 값을 쓴다).
    pub fn last_child(&self, ns: &str, name: &str) -> Option<&Element> {
        self.children.iter().rev().find(|c| c.is(ns, name))
    }

    pub fn children_named<'a>(
        &'a self,
        ns: &'a str,
        name: &'a str,
    ) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.is(ns, name))
    }

    /// 자식 요소의 문자열 값(`XmlSerializer`의 `string` 속성). 없으면 `None`.
    pub fn child_text(&self, ns: &str, name: &str) -> Option<String> {
        self.last_child(ns, name).map(|c| c.text.clone())
    }
}

/// 바이트 위치를 (줄, 칸)으로 바꾼다.
fn position_of(text: &str, offset: usize) -> Position {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit_once('\n')
        .map_or(before, |(_, last)| last)
        .chars()
        .count()
        + 1;
    (line, column)
}

/// 문서를 읽어 루트 요소를 돌려준다. 루트 뒤의 내용은 읽지 않는다.
pub fn parse(text: &str) -> Result<Element, XmlError> {
    if text.is_empty() {
        return Err(XmlError((0, 0)));
    }
    let mut reader = NsReader::from_str(text);
    let mut stack: Vec<Element> = Vec::new();
    loop {
        let start = reader.buffer_position() as usize;
        let error_at = |offset: usize| XmlError(position_of(text, offset));
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(value) => value,
            Err(_) => return Err(error_at(reader.error_position() as usize)),
        };
        let open = |e: &quick_xml::events::BytesStart<'_>| -> Result<Element, XmlError> {
            let ns = match &resolved {
                ResolveResult::Bound(ns) => ns.as_ref().to_string(),
                ResolveResult::Unbound => String::new(),
                ResolveResult::Unknown(_) => return Err(error_at(start + 1)),
            };
            Ok(Element {
                ns,
                name: e.local_name().as_ref().to_string(),
                position: position_of(text, start + 1),
                ..Element::default()
            })
        };
        match event {
            Event::Start(e) => stack.push(open(&e)?),
            Event::Empty(e) => {
                let element = open(&e)?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => return Ok(element),
                }
            }
            Event::End(_) => {
                let element = stack.pop().ok_or_else(|| error_at(start))?;
                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => return Ok(element),
                }
            }
            Event::Text(t) => {
                let value = t.xml10_content();
                match stack.last_mut() {
                    Some(current) => current.text.push_str(&value),
                    // 루트 앞에는 공백만 올 수 있다.
                    None => {
                        if let Some(index) = value.find(|c: char| !c.is_whitespace()) {
                            return Err(error_at(start + index));
                        }
                    }
                }
            }
            Event::GeneralRef(r) => {
                let name = r.xml10_content();
                let resolved = match name.as_ref() {
                    "lt" => "<".to_string(),
                    "gt" => ">".to_string(),
                    "amp" => "&".to_string(),
                    "apos" => "'".to_string(),
                    "quot" => "\"".to_string(),
                    _ => match r.resolve_char_ref() {
                        Ok(Some(c)) => c.to_string(),
                        _ => return Err(error_at(start)),
                    },
                };
                match stack.last_mut() {
                    Some(current) => current.text.push_str(&resolved),
                    None => return Err(error_at(start)),
                }
            }
            Event::CData(c) => {
                let value = c.xml10_content();
                match stack.last_mut() {
                    Some(current) => current.text.push_str(&value),
                    None => return Err(error_at(start)),
                }
            }
            Event::Eof => return Err(error_at(text.len())),
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions() {
        assert_eq!(parse("").unwrap_err(), XmlError((0, 0)));
        assert_eq!(parse("oops").unwrap_err(), XmlError((1, 1)));
        let root =
            parse("<?xml version=\"1.0\"?>\n<a xmlns=\"urn:x\"><b>1 &amp; 2</b><c/></a>").unwrap();
        assert_eq!((root.ns.as_str(), root.name.as_str()), ("urn:x", "a"));
        assert_eq!(root.position, (2, 2));
        assert_eq!(root.child_text("urn:x", "b").as_deref(), Some("1 & 2"));
        assert_eq!(root.child_text("urn:x", "c").as_deref(), Some(""));
        assert_eq!(root.child_text("", "b"), None);
    }
}
