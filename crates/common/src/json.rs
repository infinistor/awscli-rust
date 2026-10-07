//! .NET `JsonSerializer.Deserialize<T>(string[, options])`(`System.Text.Json`) 읽기 규칙 이식.
//!
//! 응답을 어떻게 읽는지(속성 이름 대소문자, 숫자 열거형, 문자열로 온 숫자, 끝 쉼표)뿐 아니라
//! 읽기에 실패했을 때의 `JsonException` 메시지(`Path`, `LineNumber`, `BytePositionInLine`)까지
//! 원본과 같게 맞추려고 serde 대신 `Utf8JsonReader`와 같은 순서로 토큰을 읽는다.
//!
//! - 속성 이름은 대소문자를 구분한다. 모르는 속성은 건너뛰고, 같은 속성이 두 번 나오면 마지막 값이 남는다.
//! - 열거형은 숫자만 받는다(정의되지 않은 값도 그대로 받는다). 문자열은 변환 오류다.
//! - 주석은 허용하지 않는다. 끝 쉼표는 [`ReadOptions::allow_trailing_commas`]일 때만 허용한다.
//! - [`ReadOptions::number_from_string`]이면 숫자 형식 속성이 `"7"` 같은 문자열도 받는다
//!   (`JsonNumberHandling.AllowReadingFromString`).
//! - 읽기 오류의 위치는 .NET이 보고하는 UTF-8 바이트 단위 줄 번호와 줄 안 위치다.

use crate::DotnetDateTime;
use crate::dotnet_datetime::DateTimeKind;
use chrono::{FixedOffset, NaiveDate, TimeZone};

/// `JsonSerializerOptions` 중 읽기에 영향을 주는 값(나머지는 기본값).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadOptions {
    /// `AllowTrailingCommas`
    pub allow_trailing_commas: bool,
    /// `NumberHandling = JsonNumberHandling.AllowReadingFromString`
    pub number_from_string: bool,
}

/// `System.Text.Json.JsonException`. 메시지는 .NET과 같은 형식이다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct JsonError(pub String);

/// `Utf8JsonReader`가 돌려주는 토큰.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StartObject,
    EndObject,
    StartArray,
    EndArray,
    PropertyName(String),
    String(String),
    /// 숫자 원문
    Number(String),
    True,
    False,
    Null,
}

struct SyntaxError {
    message: String,
    offset: usize,
}

fn syntax(message: impl Into<String>, offset: usize) -> SyntaxError {
    SyntaxError {
        message: message.into(),
        offset,
    }
}

/// `ThrowHelper.GetPrintableString`: 출력 가능한 ASCII가 아니면 `0xNN`.
fn printable(byte: u8) -> String {
    if (0x20..=0x7E).contains(&byte) {
        (byte as char).to_string()
    } else {
        format!("0x{byte:02X}")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Last {
    None,
    StartObject,
    StartArray,
    PropertyName,
    Value,
}

/// `Utf8JsonReader`(마지막 블록, 주석 불허).
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// 열린 컨테이너. `true`가 객체.
    stack: Vec<bool>,
    started: bool,
    last: Last,
    allow_trailing_commas: bool,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str, allow_trailing_commas: bool) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
            stack: Vec::new(),
            started: false,
            last: Last::None,
            allow_trailing_commas,
        }
    }

    fn skip_ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.pos += 1;
        }
    }

    /// 다음 토큰. 입력이 끝났으면 `None`.
    fn read(&mut self) -> Result<Option<Token>, SyntaxError> {
        self.skip_ws();
        let len = self.bytes.len();
        let eof = self.pos >= len;
        if self.stack.is_empty() {
            if !self.started {
                if eof {
                    return Err(syntax(
                        "The input does not contain any JSON tokens. Expected the input to start with a valid JSON token, when isFinalBlock is true.",
                        self.pos,
                    ));
                }
                self.started = true;
                return self.consume_value().map(Some);
            }
            if eof {
                return Ok(None);
            }
            return Err(syntax(
                format!(
                    "'{}' is invalid after a single JSON value. Expected end of data.",
                    printable(self.bytes[self.pos])
                ),
                self.pos,
            ));
        }
        if eof {
            return Err(syntax(
                "Expected depth to be zero at the end of the JSON payload. There is an open JSON object or array that should be closed.",
                self.pos,
            ));
        }
        let in_object = *self.stack.last().expect("열린 컨테이너");
        let byte = self.bytes[self.pos];
        match self.last {
            Last::StartObject => match byte {
                b'"' => self.property_name().map(Some),
                b'}' => Ok(Some(self.end_object())),
                _ => Err(not_property_start(byte, self.pos)),
            },
            Last::StartArray => {
                if byte == b']' {
                    Ok(Some(self.end_array()))
                } else {
                    self.consume_value().map(Some)
                }
            }
            Last::PropertyName => self.consume_value().map(Some),
            Last::Value | Last::None => match byte {
                b',' => {
                    let comma = self.pos;
                    self.pos += 1;
                    // 쉼표 바로 뒤에서 입력이 끝나면 위치는 쉼표를 가리키고, 공백이 있으면 입력 끝이다.
                    let no_space = self.pos >= self.bytes.len();
                    self.skip_ws();
                    let Some(&next) = self.bytes.get(self.pos) else {
                        return Err(syntax(
                            "Expected start of a property name or value, but instead reached end of data.",
                            if no_space { comma } else { self.pos },
                        ));
                    };
                    if in_object {
                        match next {
                            b'"' => self.property_name().map(Some),
                            b'}' if self.allow_trailing_commas => Ok(Some(self.end_object())),
                            b'}' => Err(syntax(
                                "The JSON object contains a trailing comma at the end which is not supported in this mode. Change the reader options.",
                                self.pos,
                            )),
                            _ => Err(not_property_start(next, self.pos)),
                        }
                    } else {
                        match next {
                            b']' if self.allow_trailing_commas => Ok(Some(self.end_array())),
                            b']' => Err(syntax(
                                "The JSON array contains a trailing comma at the end which is not supported in this mode. Change the reader options.",
                                self.pos,
                            )),
                            _ => self.consume_value().map(Some),
                        }
                    }
                }
                b'}' if in_object => Ok(Some(self.end_object())),
                b']' if !in_object => Ok(Some(self.end_array())),
                _ => Err(syntax(
                    format!(
                        "'{}' is invalid after a value. Expected either ',', '}}', or ']'.",
                        printable(byte)
                    ),
                    self.pos,
                )),
            },
        }
    }

    fn end_object(&mut self) -> Token {
        self.pos += 1;
        self.stack.pop();
        self.last = Last::Value;
        Token::EndObject
    }

    fn end_array(&mut self) -> Token {
        self.pos += 1;
        self.stack.pop();
        self.last = Last::Value;
        Token::EndArray
    }

    fn property_name(&mut self) -> Result<Token, SyntaxError> {
        let name = self.string()?;
        self.skip_ws();
        match self.bytes.get(self.pos) {
            None => Err(syntax(
                "Expected a value, but instead reached end of data.",
                self.pos,
            )),
            Some(b':') => {
                self.pos += 1;
                self.last = Last::PropertyName;
                Ok(Token::PropertyName(name))
            }
            Some(&other) => Err(syntax(
                format!(
                    "'{}' is invalid after a property name. Expected a ':'.",
                    printable(other)
                ),
                self.pos,
            )),
        }
    }

    fn consume_value(&mut self) -> Result<Token, SyntaxError> {
        let byte = self.bytes[self.pos];
        let token = match byte {
            b'"' => Token::String(self.string()?),
            b'{' => {
                self.pos += 1;
                self.stack.push(true);
                self.last = Last::StartObject;
                return Ok(Token::StartObject);
            }
            b'[' => {
                self.pos += 1;
                self.stack.push(false);
                self.last = Last::StartArray;
                return Ok(Token::StartArray);
            }
            b'-' | b'0'..=b'9' => self.number()?,
            b't' => self.literal("true", Token::True)?,
            b'f' => self.literal("false", Token::False)?,
            b'n' => self.literal("null", Token::Null)?,
            _ => {
                return Err(syntax(
                    format!("'{}' is an invalid start of a value.", printable(byte)),
                    self.pos,
                ));
            }
        };
        self.last = Last::Value;
        Ok(token)
    }

    fn literal(&mut self, text: &str, token: Token) -> Result<Token, SyntaxError> {
        let start = self.pos;
        let matched = text
            .bytes()
            .enumerate()
            .take_while(|(i, b)| self.bytes.get(start + i) == Some(b))
            .count();
        if matched < text.len() {
            // 메시지에는 첫 구분자(`,` `}` `]`)까지, 없으면 남은 입력 전체를 보여 준다.
            let rest = &self.bytes[start..];
            let end = rest
                .iter()
                .position(|b| matches!(b, b',' | b'}' | b']'))
                .map_or(rest.len(), |index| index + 1);
            return Err(syntax(
                format!(
                    "'{}' is an invalid JSON literal. Expected the literal '{text}'.",
                    String::from_utf8_lossy(&rest[..end])
                ),
                start + matched,
            ));
        }
        self.pos += text.len();
        Ok(token)
    }

    /// 현재 위치의 `"`부터 문자열을 읽는다.
    fn string(&mut self) -> Result<String, SyntaxError> {
        let end_of_data = |len: usize| {
            syntax(
                "Expected end of string, but instead reached end of data.",
                len,
            )
        };
        let len = self.bytes.len();
        let mut i = self.pos + 1;
        let mut out: Vec<u8> = Vec::new();
        let mut pending_high: Option<u16> = None;
        loop {
            let Some(&byte) = self.bytes.get(i) else {
                return Err(end_of_data(len));
            };
            match byte {
                b'"' => break,
                0..=0x1F => {
                    return Err(syntax(
                        format!(
                            "'{}' is invalid within a JSON string. The string should be correctly escaped.",
                            printable(byte)
                        ),
                        i,
                    ));
                }
                b'\\' => {
                    i += 1;
                    let Some(&escape) = self.bytes.get(i) else {
                        return Err(end_of_data(len));
                    };
                    let simple = match escape {
                        b'"' => Some(b'"'),
                        b'\\' => Some(b'\\'),
                        b'/' => Some(b'/'),
                        b'b' => Some(0x08),
                        b'f' => Some(0x0C),
                        b'n' => Some(b'\n'),
                        b'r' => Some(b'\r'),
                        b't' => Some(b'\t'),
                        b'u' => None,
                        other => {
                            return Err(syntax(
                                format!(
                                    "'{}' is an invalid escapable character within a JSON string. The string should be correctly escaped.",
                                    printable(other)
                                ),
                                i,
                            ));
                        }
                    };
                    match simple {
                        Some(b) => out.push(b),
                        None => {
                            let mut unit: u32 = 0;
                            for _ in 0..4 {
                                i += 1;
                                let Some(&hex) = self.bytes.get(i) else {
                                    return Err(end_of_data(len));
                                };
                                let Some(digit) = (hex as char).to_digit(16) else {
                                    return Err(syntax(
                                        format!(
                                            "'{}' is not a hex digit following '\\u' within a JSON string. The string should be correctly escaped.",
                                            printable(hex)
                                        ),
                                        i,
                                    ));
                                };
                                unit = unit * 16 + digit;
                            }
                            push_utf16_unit(&mut out, &mut pending_high, unit as u16);
                            i += 1;
                            continue;
                        }
                    }
                    i += 1;
                }
                _ => {
                    out.push(byte);
                    i += 1;
                }
            }
        }
        self.pos = i + 1;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn number(&mut self) -> Result<Token, SyntaxError> {
        let len = self.bytes.len();
        let start = self.pos;
        let mut i = start;
        let digit_at = |i: usize| self.bytes.get(i).is_some_and(u8::is_ascii_digit);
        let end_of_data = || {
            syntax(
                "Expected a digit ('0'-'9'), but instead reached end of data.",
                len,
            )
        };
        if self.bytes[i] == b'-' {
            i += 1;
            if i >= len {
                return Err(end_of_data());
            }
            if !digit_at(i) {
                return Err(syntax(
                    format!(
                        "'{}' is invalid within a number, immediately after a sign character ('+' or '-'). Expected a digit ('0'-'9').",
                        printable(self.bytes[i])
                    ),
                    i,
                ));
            }
        }
        if self.bytes[i] == b'0' {
            i += 1;
            if digit_at(i) {
                return Err(syntax(
                    format!(
                        "Invalid leading zero before '{}'.",
                        printable(self.bytes[i])
                    ),
                    i,
                ));
            }
        } else {
            while digit_at(i) {
                i += 1;
            }
        }
        let mut after_fraction = false;
        if self.bytes.get(i) == Some(&b'.') {
            after_fraction = true;
            i += 1;
            if i >= len {
                return Err(end_of_data());
            }
            if !digit_at(i) {
                return Err(syntax(
                    format!(
                        "'{}' is invalid within a number, immediately after a decimal point ('.'). Expected a digit ('0'-'9').",
                        printable(self.bytes[i])
                    ),
                    i,
                ));
            }
            while digit_at(i) {
                i += 1;
            }
        }
        if matches!(self.bytes.get(i), Some(b'e' | b'E')) {
            after_fraction = false;
            i += 1;
            if matches!(self.bytes.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            if i >= len {
                return Err(end_of_data());
            }
            if !digit_at(i) {
                return Err(syntax(
                    format!(
                        "'{}' is invalid within a number, immediately after a sign character ('+' or '-'). Expected a digit ('0'-'9').",
                        printable(self.bytes[i])
                    ),
                    i,
                ));
            }
            while digit_at(i) {
                i += 1;
            }
        }
        match self.bytes.get(i) {
            None => {
                // 최상위 숫자는 데이터 끝에서 끝나도 되지만, 컨테이너 안에서는 구분자가 필요하다.
                if !self.stack.is_empty() {
                    return Err(syntax(
                        format!(
                            "'{}' is an invalid end of a number. Expected a delimiter.",
                            printable(self.bytes[i - 1])
                        ),
                        len,
                    ));
                }
            }
            Some(&c) if !matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b',' | b']' | b'}') => {
                // 소수부 뒤에는 지수가 올 수 있어 메시지가 다르다.
                let expected = if after_fraction {
                    "'E' or 'e'"
                } else {
                    "a delimiter"
                };
                return Err(syntax(
                    format!(
                        "'{}' is an invalid end of a number. Expected {expected}.",
                        printable(c)
                    ),
                    i,
                ));
            }
            Some(_) => {}
        }
        self.pos = i;
        Ok(Token::Number(
            String::from_utf8_lossy(&self.bytes[start..i]).into_owned(),
        ))
    }
}

fn not_property_start(byte: u8, offset: usize) -> SyntaxError {
    syntax(
        format!(
            "'{}' is an invalid start of a property name. Expected a '\"'.",
            printable(byte)
        ),
        offset,
    )
}

/// `\uXXXX` 하나를 UTF-8로 덧붙인다. 상위 서로게이트는 하위 서로게이트가 이어질 때까지 보류한다.
fn push_utf16_unit(out: &mut Vec<u8>, pending_high: &mut Option<u16>, unit: u16) {
    let mut push = |c: char| {
        let mut buf = [0u8; 4];
        out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    };
    match (pending_high.take(), unit) {
        (Some(high), 0xDC00..=0xDFFF) => {
            let code = 0x10000 + (((high as u32) - 0xD800) << 10) + ((unit as u32) - 0xDC00);
            push(char::from_u32(code).unwrap_or('\u{FFFD}'));
        }
        (Some(_), _) => {
            push('\u{FFFD}');
            handle_single(unit, pending_high, &mut push);
        }
        (None, _) => handle_single(unit, pending_high, &mut push),
    }
}

fn handle_single(unit: u16, pending_high: &mut Option<u16>, push: &mut impl FnMut(char)) {
    match unit {
        0xD800..=0xDBFF => *pending_high = Some(unit),
        0xDC00..=0xDFFF => push('\u{FFFD}'),
        _ => push(char::from_u32(unit as u32).unwrap_or('\u{FFFD}')),
    }
}

#[derive(Debug, Clone)]
enum PathSegment {
    Property(String),
    Index(usize),
}

/// 역직렬화 상태. 읽는 위치(`Path`)를 함께 관리한다.
pub struct Deserializer<'a> {
    reader: Reader<'a>,
    path: Vec<PathSegment>,
    options: ReadOptions,
}

/// JSON 값을 읽는 타입. 원본의 `JsonConverter<T>`에 해당한다.
pub trait FromJson: Sized {
    /// 변환 오류 메시지에 들어가는 .NET 형식 이름
    fn type_name() -> String;

    /// 첫 토큰(`tok`)을 이미 읽은 상태에서 값을 읽는다. `null`이면 `None`(값 형식은 변환 오류).
    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError>;
}

/// `JsonSerializer.Deserialize<T>(json, options)`. JSON `null`이면 `None`.
pub fn deserialize<T: FromJson>(text: &str, options: ReadOptions) -> Result<Option<T>, JsonError> {
    let mut d = Deserializer {
        reader: Reader::new(text, options.allow_trailing_commas),
        path: Vec::new(),
        options,
    };
    let tok = d.read_token()?;
    let value = T::from_json(&mut d, tok)?;
    // 값 뒤에 다른 내용이 있으면 오류다.
    d.read_end()?;
    Ok(value)
}

impl Deserializer<'_> {
    pub fn options(&self) -> ReadOptions {
        self.options
    }

    fn path_text(&self) -> String {
        let mut text = String::from("$");
        for segment in &self.path {
            match segment {
                PathSegment::Property(name) => {
                    text.push('.');
                    text.push_str(name);
                }
                PathSegment::Index(index) => text.push_str(&format!("[{index}]")),
            }
        }
        text
    }

    fn error_at(&self, message: &str, offset: usize) -> JsonError {
        let bytes = self.reader.bytes;
        let before = &bytes[..offset.min(bytes.len())];
        let line = before.iter().filter(|&&b| b == b'\n').count();
        let line_start = before
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |index| index + 1);
        JsonError(format!(
            "{message} Path: {} | LineNumber: {line} | BytePositionInLine: {}.",
            self.path_text(),
            offset - line_start
        ))
    }

    fn syntax_error(&self, error: SyntaxError) -> JsonError {
        self.error_at(&error.message, error.offset)
    }

    /// `The JSON value could not be converted to {type_name}.` (위치는 마지막으로 읽은 토큰의 끝)
    pub fn conversion_error(&self, type_name: &str) -> JsonError {
        self.error_at(
            &format!("The JSON value could not be converted to {type_name}."),
            self.reader.pos,
        )
    }

    pub fn read_token(&mut self) -> Result<Token, JsonError> {
        match self.reader.read() {
            Ok(Some(token)) => Ok(token),
            // 값 하나를 읽는 중에 입력이 끝나는 경우는 리더가 오류로 알린다.
            Ok(None) => Err(self.error_at("Unexpected end of data.", self.reader.pos)),
            Err(error) => Err(self.syntax_error(error)),
        }
    }

    fn read_end(&mut self) -> Result<(), JsonError> {
        match self.reader.read() {
            Ok(_) => Ok(()),
            Err(error) => Err(self.syntax_error(error)),
        }
    }

    /// 다음 값을 읽는다(`null`이면 `None`).
    pub fn read_nullable<T: FromJson>(&mut self) -> Result<Option<T>, JsonError> {
        let tok = self.read_token()?;
        T::from_json(self, tok)
    }

    /// 다음 값을 읽는다. 값 형식(숫자, 불리언, 열거형 등)용이며 `null`은 변환 오류다.
    pub fn read_value<T: FromJson>(&mut self) -> Result<T, JsonError> {
        let tok = self.read_token()?;
        match T::from_json(self, tok)? {
            Some(value) => Ok(value),
            None => Err(self.conversion_error(&T::type_name())),
        }
    }

    /// 값 하나를 읽지 않고 건너뛴다(`Utf8JsonReader.Skip`).
    pub fn skip(&mut self, tok: Token) -> Result<(), JsonError> {
        let mut depth = match tok {
            Token::StartObject | Token::StartArray => 1usize,
            _ => return Ok(()),
        };
        while depth > 0 {
            match self.read_token()? {
                Token::StartObject | Token::StartArray => depth += 1,
                Token::EndObject | Token::EndArray => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    /// 객체를 읽는다. `set`은 속성 이름마다 호출되며, 값을 읽어 필드에 넣고 `true`를,
    /// 모르는 속성이면 값을 읽지 않고 `false`를 돌려준다(그러면 값을 건너뛴다).
    pub fn read_object<T>(
        &mut self,
        tok: Token,
        type_name: &str,
        mut object: T,
        mut set: impl FnMut(&mut Self, &mut T, &str) -> Result<bool, JsonError>,
    ) -> Result<Option<T>, JsonError> {
        match tok {
            Token::Null => return Ok(None),
            Token::StartObject => {}
            _ => return Err(self.conversion_error(type_name)),
        }
        loop {
            match self.read_token()? {
                Token::EndObject => return Ok(Some(object)),
                Token::PropertyName(name) => {
                    self.path.push(PathSegment::Property(name.clone()));
                    if !set(self, &mut object, &name)? {
                        let tok = self.read_token()?;
                        self.skip(tok)?;
                    }
                    self.path.pop();
                }
                _ => unreachable!("객체 안에서는 속성 이름이나 객체 끝만 나온다"),
            }
        }
    }

    /// `List<T>`를 읽는다. 요소가 `null`이면 `None`이다.
    pub fn read_list<T: FromJson>(
        &mut self,
        tok: Token,
    ) -> Result<Option<Vec<Option<T>>>, JsonError> {
        match tok {
            Token::Null => return Ok(None),
            Token::StartArray => {}
            _ => return Err(self.conversion_error(&list_type_name::<T>())),
        }
        let mut items = Vec::new();
        loop {
            self.path.push(PathSegment::Index(items.len()));
            let tok = self.read_token()?;
            if tok == Token::EndArray {
                self.path.pop();
                return Ok(Some(items));
            }
            items.push(T::from_json(self, tok)?);
            self.path.pop();
        }
    }
}

/// `System.Collections.Generic.List`1[T]`
pub fn list_type_name<T: FromJson>() -> String {
    format!("System.Collections.Generic.List`1[{}]", T::type_name())
}

impl FromJson for String {
    fn type_name() -> String {
        "System.String".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        match tok {
            Token::Null => Ok(None),
            Token::String(s) => Ok(Some(s)),
            _ => Err(d.conversion_error("System.String")),
        }
    }
}

impl FromJson for bool {
    fn type_name() -> String {
        "System.Boolean".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        match tok {
            Token::True => Ok(Some(true)),
            Token::False => Ok(Some(false)),
            _ => Err(d.conversion_error("System.Boolean")),
        }
    }
}

/// 정수 형식. 숫자 토큰은 `Utf8Parser`처럼 소수점·지수 없이 전체가 정수여야 하고, 문자열 숫자
/// 허용 옵션이 켜져 있으면 문자열도 같은 규칙으로 읽는다(부호 `+`도 허용).
macro_rules! impl_int {
    ($t:ty, $name:literal) => {
        impl FromJson for $t {
            fn type_name() -> String {
                $name.into()
            }

            fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
                let text = match tok {
                    Token::Number(text) => text,
                    Token::String(text) if d.options.number_from_string => text,
                    _ => return Err(d.conversion_error($name)),
                };
                text.parse::<$t>()
                    .map(Some)
                    .map_err(|_| d.conversion_error($name))
            }
        }
    };
}

impl_int!(i32, "System.Int32");
impl_int!(i64, "System.Int64");
impl_int!(u64, "System.UInt64");

impl FromJson for f32 {
    fn type_name() -> String {
        "System.Single".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        let text = match tok {
            Token::Number(text) => text,
            Token::String(text) if d.options.number_from_string => text,
            _ => return Err(d.conversion_error("System.Single")),
        };
        // 범위를 넘는 값은 오류가 아니라 무한대가 된다(.NET loat.Parse와 같다).
        text.parse::<f32>()
            .map(Some)
            .map_err(|_| d.conversion_error("System.Single"))
    }
}

/// .NET `decimal`. 자릿수(스케일)를 그대로 보존해 `1000.50`을 `1000.50`으로 출력한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal(String);

impl Default for Decimal {
    fn default() -> Self {
        Self("0".into())
    }
}

impl Decimal {
    /// `decimal.ToString()`과 같은 표기.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// JSON 숫자 원문(지수 표기 포함)을 .NET `Number.NumberToDecimal`과 같은 규칙으로 읽는다.
    ///
    /// 자릿수를 96비트 정수에 하나씩 담되 소수 자릿수는 28자리까지만 쓰고, 담지 못한 첫 자리로 반올림한다
    /// (정확히 5이고 그 뒤가 모두 0이면 짝수로). 정수부가 29자리를 넘거나 담지 못하면 변환 오류다.
    fn parse(text: &str) -> Option<Self> {
        const MAX: u128 = (1u128 << 96) - 1;
        let (mantissa, exponent) = match text.split_once(['e', 'E']) {
            Some((m, e)) => (m, e.parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (negative, mantissa) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa),
        };
        let (int_part, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        // 값 = 0.d1d2d3... x 10^e (앞의 0은 버린다)
        let all = format!("{int_part}{fraction}");
        let digits = all.trim_start_matches('0').as_bytes();
        let mut e = int_part.len() as i32 + exponent - (all.len() - digits.len()) as i32;

        let mut value: u128 = 0;
        if digits.is_empty() {
            // 0은 자릿수(스케일)만 남는다.
            e = e.clamp(-28, 0);
        } else {
            if e > 29 {
                return None;
            }
            let mut next = 0;
            // 정수부는 끝까지(모자라는 자릿수는 0으로), 소수부는 남은 자릿수가 있고 28자리 안에서만 담는다.
            while e > 0 || (next < digits.len() && e > -28) {
                let digit = digits.get(next).map_or(0, |d| u128::from(d - b'0'));
                match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(v) if v <= MAX => value = v,
                    _ => break,
                }
                if next < digits.len() {
                    next += 1;
                }
                e -= 1;
            }
            if e > 0 {
                return None;
            }
            if let Some(&dropped) = digits.get(next).filter(|&&d| d >= b'5') {
                let mut round = true;
                // 정확히 5로 끝나면 짝수 쪽으로 반올림한다. .NET 구현은 하위 64비트만 보므로 값이 64비트를 넘으면
                // 항상 올린다(오라클로 확인).
                if dropped == b'5' && value <= u128::from(u64::MAX) && value.is_multiple_of(2) {
                    let rest = &digits[next + 1..];
                    let zeros = rest.iter().take(20).take_while(|&&d| d == b'0').count();
                    if zeros == rest.len() || zeros == 20 {
                        round = false;
                    }
                }
                if round {
                    value += 1;
                    if value > MAX {
                        // 올림으로 96비트를 넘으면 10으로 나누고 스케일을 하나 줄인다.
                        value = (value + 5) / 10;
                        e += 1;
                        if e > 0 {
                            return None;
                        }
                    }
                }
            }
        }
        // 소수 자릿수는 28자리까지(0.5E-30처럼 값이 0이어도 28자리로 맞춘다).
        let scale = (-e).min(28) as usize;
        let mut digits = value.to_string();
        if digits.len() <= scale {
            digits = format!("{}{digits}", "0".repeat(scale + 1 - digits.len()));
        }
        let (int_digits, frac_digits) = digits.split_at(digits.len() - scale);
        let sign = if negative && value != 0 { "-" } else { "" };
        // .NET의 decimal -0.0은 부호를 유지하지 않는다(0.0).
        Some(Self(if scale == 0 {
            format!("{sign}{int_digits}")
        } else {
            format!("{sign}{int_digits}.{frac_digits}")
        }))
    }
}
impl FromJson for Decimal {
    fn type_name() -> String {
        "System.Decimal".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        let Token::Number(text) = tok else {
            return Err(d.conversion_error("System.Decimal"));
        };
        Decimal::parse(&text)
            .map(Some)
            .ok_or_else(|| d.conversion_error("System.Decimal"))
    }
}

impl serde::Serialize for Decimal {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let raw = serde_json::value::RawValue::from_string(self.0.clone())
            .map_err(serde::ser::Error::custom)?;
        raw.serialize(serializer)
    }
}

impl FromJson for DotnetDateTime {
    fn type_name() -> String {
        "System.DateTime".into()
    }

    fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
        let Token::String(text) = tok else {
            return Err(d.conversion_error("System.DateTime"));
        };
        parse_iso8601(&text)
            .map(Some)
            .ok_or_else(|| d.conversion_error("System.DateTime"))
    }
}

/// `System.Text.Json`의 `DateTime` 읽기(ISO 8601-1:2019 부분집합):
/// `YYYY-MM-DD[Thh:mm[:ss[.f*]][Z|±hh[:mm]]]`. 소수부는 7자리까지만 쓰고, `Z`는 `Utc`,
/// 오프셋은 이 PC의 현지 시각(`Local`), 없으면 `Unspecified`.
fn parse_iso8601(text: &str) -> Option<DotnetDateTime> {
    let b = text.as_bytes();
    let digits = |from: usize, count: usize| -> Option<u32> {
        let slice = b.get(from..from + count)?;
        if !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(slice).ok()?.parse().ok()
    };
    if !text.is_ascii() || b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (year, month, day) = (digits(0, 4)?, digits(5, 2)?, digits(8, 2)?);
    let date = NaiveDate::from_ymd_opt(year as i32, month, day)?;
    if b.len() == 10 {
        return Some(DotnetDateTime {
            value: date.and_hms_opt(0, 0, 0)?,
            kind: DateTimeKind::Unspecified,
        });
    }
    if b[10] != b'T' || b.len() < 16 || b[13] != b':' {
        return None;
    }
    let (hour, minute) = (digits(11, 2)?, digits(14, 2)?);
    let mut i = 16;
    let mut second = 0;
    let mut ticks = 0u32;
    if b.get(i) == Some(&b':') {
        second = digits(i + 1, 2)?;
        i += 3;
        if b.get(i) == Some(&b'.') {
            i += 1;
            let start = i;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            let fraction: String = text[start..i].chars().take(7).collect();
            if fraction.is_empty() {
                // 소수점 뒤에 숫자가 없으면 뒤에 시간대가 있어야 한다(.Z는 되지만 끝의 .은 안 된다).
                if i == b.len() {
                    return None;
                }
            } else {
                ticks = format!("{fraction:0<7}").parse().ok()?;
            }
        }
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let value = date.and_hms_nano_opt(hour, minute, second, ticks * 100)?;
    let zone = &text[i..];
    match zone {
        "" => Some(DotnetDateTime {
            value,
            kind: DateTimeKind::Unspecified,
        }),
        "Z" => Some(DotnetDateTime {
            value,
            kind: DateTimeKind::Utc,
        }),
        _ => {
            let sign = match zone.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let zb = zone.as_bytes();
            let (offset_hour, offset_minute) = match zb.len() {
                3 => (parse_two(&zb[1..3])?, 0),
                6 if zb[3] == b':' => (parse_two(&zb[1..3])?, parse_two(&zb[4..6])?),
                _ => return None,
            };
            // 오프셋은 ±14:00까지(DateTimeOffset의 범위)
            let total = offset_hour * 3600 + offset_minute * 60;
            if offset_minute > 59 || total > 14 * 3600 {
                return None;
            }
            let offset = FixedOffset::east_opt(sign * total as i32)?;
            let instant = offset.from_local_datetime(&value).single()?;
            // UTC 시각이 DateTime.MinValue보다 이르면 변환할 수 없다.
            if instant.naive_utc() < min_date_time()? {
                return None;
            }
            let mut local = DotnetDateTime::local(instant);
            // 현지 시각이 DateTime.MaxValue를 넘으면 최대값으로 맞춘다(.NET 변환 규칙).
            let max =
                NaiveDate::from_ymd_opt(9999, 12, 31)?.and_hms_nano_opt(23, 59, 59, 999_999_900)?;
            if local.value > max {
                local.value = max;
            }
            Some(local)
        }
    }
}

fn min_date_time() -> Option<chrono::NaiveDateTime> {
    NaiveDate::from_ymd_opt(1, 1, 1)?.and_hms_opt(0, 0, 0)
}

fn parse_two(bytes: &[u8]) -> Option<u32> {
    if !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

/// 열거형(숫자). 정의되지 않은 값도 그대로 보존한다. `Debug`·`Display`는 `Enum.ToString()`처럼
/// 이름이 있으면 이름, 없으면 숫자를 출력한다.
#[macro_export]
macro_rules! dotnet_enum {
    ($(#[$meta:meta])* $name:ident, $full_name:literal, { $($variant:ident = $value:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i32);

        #[allow(non_upper_case_globals)]
        impl $name {
            $(pub const $variant: Self = Self($value);)+

            /// .NET `Enum.ToString()`: 이름이 있으면 이름, 없으면 숫자
            pub fn dotnet_name(&self) -> String {
                match self.0 {
                    $(v if v == $value => stringify!($variant).to_string(),)+
                    other => other.to_string(),
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.dotnet_name())
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.dotnet_name())
            }
        }

        impl $crate::json::FromJson for $name {
            fn type_name() -> String {
                $full_name.into()
            }

            fn from_json(
                d: &mut $crate::json::Deserializer<'_>,
                tok: $crate::json::Token,
            ) -> Result<Option<Self>, $crate::json::JsonError> {
                match tok {
                    $crate::json::Token::Number(text) => text
                        .parse::<i32>()
                        .map(|v| Some(Self(v)))
                        .map_err(|_| d.conversion_error($full_name)),
                    _ => Err(d.conversion_error($full_name)),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, PartialEq)]
    struct Sample {
        a: i32,
        b: Option<String>,
    }

    impl FromJson for Sample {
        fn type_name() -> String {
            "Sample".into()
        }

        fn from_json(d: &mut Deserializer<'_>, tok: Token) -> Result<Option<Self>, JsonError> {
            d.read_object(tok, "Sample", Sample::default(), |d, o, name| {
                match name {
                    "A" => o.a = d.read_value()?,
                    "B" => o.b = d.read_nullable()?,
                    _ => return Ok(false),
                }
                Ok(true)
            })
        }
    }

    fn read(text: &str) -> Result<Option<Sample>, String> {
        deserialize::<Sample>(text, ReadOptions::default()).map_err(|e| e.0)
    }

    #[test]
    fn reads_object() {
        assert_eq!(
            read(r#"{"A":1,"B":"x","C":[1,{"d":null}]}"#),
            Ok(Some(Sample {
                a: 1,
                b: Some("x".into())
            }))
        );
        assert_eq!(read("null"), Ok(None));
        // 대소문자 구분, 마지막 값 우선
        assert_eq!(read(r#"{"a":1,"A":2,"A":3}"#).unwrap().unwrap().a, 3);
    }

    #[test]
    fn error_messages() {
        assert_eq!(
            read(""),
            Err("The input does not contain any JSON tokens. Expected the input to start with a valid JSON token, when isFinalBlock is true. Path: $ | LineNumber: 0 | BytePositionInLine: 0.".into())
        );
        assert_eq!(
            read(r#"{"A":"1"}"#),
            Err("The JSON value could not be converted to System.Int32. Path: $.A | LineNumber: 0 | BytePositionInLine: 8.".into())
        );
        assert_eq!(
            read("{\"A\":1,\n \"B\":5}"),
            Err("The JSON value could not be converted to System.String. Path: $.B | LineNumber: 1 | BytePositionInLine: 6.".into())
        );
        assert_eq!(
            read(r#"{"A":1} x"#),
            Err("'x' is invalid after a single JSON value. Expected end of data. Path: $ | LineNumber: 0 | BytePositionInLine: 8.".into())
        );
    }

    #[test]
    fn decimal_scale() {
        for (text, expected) in [
            ("1000", "1000"),
            ("1000.50", "1000.50"),
            ("1E+2", "100"),
            ("1.5e2", "150"),
            ("1000E-2", "10.00"),
            ("0.0", "0.0"),
            ("-0.0", "0.0"),
            ("-12.5", "-12.5"),
            ("0.001e1", "0.01"),
            (
                "0.00000000000000000000000000001",
                "0.0000000000000000000000000000",
            ),
            (
                "0.00000000000000000000000000005",
                "0.0000000000000000000000000000",
            ),
            (
                "0.00000000000000000000000000015",
                "0.0000000000000000000000000002",
            ),
            (
                "9.99999999999999999999999999999",
                "10.000000000000000000000000000",
            ),
            (
                "12345678901234567890.123456789012345678901",
                "12345678901234567890.123456789",
            ),
            (
                "79228162514264337593543950335",
                "79228162514264337593543950335",
            ),
            ("0.0000", "0.0000"),
            ("1E-30", "0.0000000000000000000000000000"),
            (
                "1.00000000000000000000000000005",
                "1.0000000000000000000000000001",
            ),
            ("2.5E-28", "0.0000000000000000000000000002"),
            ("3.5E-28", "0.0000000000000000000000000004"),
            (
                "7.92281625142643375935439503355",
                "7.922816251426433759354395034",
            ),
        ] {
            assert_eq!(Decimal::parse(text).unwrap().as_str(), expected, "{text}");
        }
        assert!(Decimal::parse("1e29").is_none());
    }

    #[test]
    fn iso_datetime() {
        assert!(parse_iso8601("2024-05-06").is_some());
        assert!(parse_iso8601("2024-05-06T07:08").is_some());
        assert!(parse_iso8601("2024-05-06T07:08:09.123456789Z").is_some());
        assert!(parse_iso8601("2024-05-06 07:08:09").is_none());
        assert!(parse_iso8601("2024-05-06T07:08:09+0900").is_none());
        assert!(parse_iso8601("2024-02-30T00:00:00").is_none());
    }
}
