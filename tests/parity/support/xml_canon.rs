//! XML 본문 의미 비교: 형제 요소를 이름순으로 정리하고 네임스페이스 선언을 뺀 문자열로 만든다.

use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<Node>,
}

impl Node {
    fn render(mut self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        self.attrs.sort();
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {k}=\"{v}\""));
        }
        out.push('>');
        out.push_str(self.text.trim());
        // 형제 요소는 이름순(같은 이름은 원래 순서 유지).
        self.children.sort_by(|a, b| a.name.cmp(&b.name));
        for child in self.children {
            child.render(out);
        }
        out.push_str("</");
        out.push_str(&self.name);
        out.push('>');
    }
}

fn open_node(e: &quick_xml::events::BytesStart<'_>) -> Node {
    let attrs = e
        .attributes()
        .flatten()
        .filter_map(|a| {
            let key = a.key.as_ref().to_string();
            // 네임스페이스 선언(`xmlns`, `xmlns:xsi`)은 접두사 표기 차이일 뿐이라 비교하지 않는다.
            if key == "xmlns" || key.starts_with("xmlns:") {
                return None;
            }
            let local = key.rsplit(':').next().unwrap().to_string();
            let value = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()?
                .to_string();
            Some((local, value))
        })
        .collect();
    Node {
        name: e.local_name().as_ref().to_string(),
        attrs,
        ..Node::default()
    }
}

/// XML 문서를 형제 요소 이름순으로 정리한 문자열. XML이 아니면 `None`.
pub fn canonical_xml(text: &str) -> Option<String> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Node> = vec![Node::default()];
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) => stack.push(open_node(&e)),
            Event::Empty(e) => {
                let node = open_node(&e);
                stack.last_mut()?.children.push(node);
            }
            Event::End(_) => {
                let node = stack.pop()?;
                stack.last_mut()?.children.push(node);
            }
            Event::Text(t) => stack.last_mut()?.text.push_str(&t.xml10_content()),
            Event::GeneralRef(r) => {
                let name = r.xml10_content();
                let value = match name.as_ref() {
                    "lt" => "<".to_string(),
                    "gt" => ">".to_string(),
                    "amp" => "&".to_string(),
                    "apos" => "'".to_string(),
                    "quot" => "\"".to_string(),
                    _ => r.resolve_char_ref().ok()??.to_string(),
                };
                stack.last_mut()?.text.push_str(&value);
            }
            Event::CData(c) => stack.last_mut()?.text.push_str(&c.xml10_content()),
            Event::Eof => break,
            _ => {}
        }
    }
    let mut root = stack.pop()?;
    if !stack.is_empty() || root.children.len() != 1 {
        return None;
    }
    let mut out = String::new();
    root.children.remove(0).render(&mut out);
    Some(out)
}
