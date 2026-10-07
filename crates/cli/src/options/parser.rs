//! Mono.Options 6.12 `OptionSet.Parse` 규칙.
//!
//! - 접두사 `--`, `-`, `/`. 값은 `=` 또는 `:` 뒤에 붙이거나 다음 인자로 준다(다음 인자는 무엇이든 값으로 소비).
//! - 이름 끝의 `+`/`-`는 bool 형식(`+`면 인자 전체, `-`면 `null`이 값).
//! - `-`로 시작하는 등록되지 않은 이름은 한 글자 옵션 묶음으로 읽는다(`-vh`, `-bname`).
//! - `--` 뒤의 인자와 처리하지 못한 인자는 `Extra`로 돌려준다.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::convert::{to_bool, to_int32};
use super::definitions::{Action, OPTIONS, OptionDef};
use super::{CommandOptions, ParseError};

/// 원본 `CliParseResult`(옵션 집합은 정적 표 `OPTIONS`라 따로 두지 않는다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub options: CommandOptions,
    pub extra: Vec<String>,
}

/// 옵션 이름(접두사·`=` 제외) → 표 위치.
static NAMES: LazyLock<HashMap<&'static str, usize>> = LazyLock::new(|| {
    let mut names = HashMap::new();
    for (index, def) in OPTIONS.iter().enumerate() {
        for name in option_names(def) {
            assert!(
                names.insert(name, index).is_none(),
                "중복 옵션 이름: {name}"
            );
        }
    }
    names
});

/// 프로토타입의 이름들(`c|config=` → `["c", "config"]`).
pub fn option_names(def: &OptionDef) -> impl Iterator<Item = &'static str> {
    def.prototype
        .split('|')
        .map(|name| name.split(['=', ':']).next().unwrap_or(name))
}

/// 원본 `OptionSet.Contains(name)` / `OptionSet[name]`(대소문자 구분).
pub fn find_option(name: &str) -> Option<&'static OptionDef> {
    NAMES.get(name).map(|&index| &OPTIONS[index])
}

/// 원본 `OptionContext`: 값을 기다리는 옵션.
#[derive(Default)]
struct Context {
    option: Option<&'static OptionDef>,
    option_name: String,
    values: Vec<Option<String>>,
}

/// 원본 `CliOptionParser.Parse(args)`.
pub fn parse(args: &[String]) -> Result<ParseResult, ParseError> {
    let mut options = CommandOptions::default();
    let mut context = Context::default();
    let mut extra = Vec::new();
    let mut process = true;
    for argument in args {
        if argument == "--" {
            process = false;
            continue;
        }
        if !process {
            extra.push(argument.clone());
            continue;
        }
        if !parse_argument(argument, &mut context, &mut options)? {
            extra.push(argument.clone());
        }
    }
    if context.option.is_some() {
        invoke(&mut context, &mut options)?;
    }
    Ok(ParseResult { options, extra })
}

fn parse_argument(
    argument: &str,
    c: &mut Context,
    options: &mut CommandOptions,
) -> Result<bool, ParseError> {
    if c.option.is_some() {
        parse_value(Some(argument.to_string()), c, options)?;
        return Ok(true);
    }
    let Some(parts) = option_parts(argument) else {
        return Ok(false);
    };
    if let Some(def) = find_option(parts.name) {
        c.option_name = format!("{}{}", parts.flag, parts.name);
        c.option = Some(def);
        if def.action.takes_value() {
            parse_value(parts.value.map(str::to_string), c, options)?;
        } else {
            c.values.push(Some(parts.name.to_string()));
            invoke(c, options)?;
        }
        return Ok(true);
    }
    if parse_bool(argument, parts.name, c, options)? {
        return Ok(true);
    }
    let bundle = format!(
        "{}{}{}",
        parts.name,
        parts.sep.unwrap_or_default(),
        parts.value.unwrap_or_default()
    );
    parse_bundled_value(parts.flag, &bundle, c, options)
}

/// 정규식 `^(?<flag>--|-|/)(?<name>[^:=]+)((?<sep>[:=])(?<value>.*))?$`로 나눈 결과.
struct OptionParts<'a> {
    flag: &'a str,
    name: &'a str,
    sep: Option<&'a str>,
    value: Option<&'a str>,
}

fn option_parts(argument: &str) -> Option<OptionParts<'_>> {
    // 정규식의 역추적처럼 `--`가 안 되면 `-`로 다시 시도한다.
    for flag in ["--", "-", "/"] {
        let Some(rest) = argument.strip_prefix(flag) else {
            continue;
        };
        let name_end = rest.find([':', '=']).unwrap_or(rest.len());
        if name_end == 0 {
            continue;
        }
        let name = &rest[..name_end];
        if name_end == rest.len() {
            return Some(OptionParts {
                flag,
                name,
                sep: None,
                value: None,
            });
        }
        // `.`은 줄바꿈을 받지 않고, `$`는 마지막 줄바꿈 앞에서도 맞는다.
        let value = &rest[name_end + 1..];
        let value = match value.find('\n') {
            None => value,
            Some(index) if index == value.len() - 1 => &value[..index],
            Some(_) => continue,
        };
        return Some(OptionParts {
            flag,
            name,
            sep: Some(&rest[name_end..=name_end]),
            value: Some(value),
        });
    }
    None
}

fn parse_value(
    value: Option<String>,
    c: &mut Context,
    options: &mut CommandOptions,
) -> Result<(), ParseError> {
    if let Some(value) = value {
        c.values.push(Some(value));
    }
    // MaxValueCount는 모두 1이다.
    if c.values.len() == 1 {
        invoke(c, options)?;
    }
    Ok(())
}

fn parse_bool(
    argument: &str,
    name: &str,
    c: &mut Context,
    options: &mut CommandOptions,
) -> Result<bool, ParseError> {
    let Some(last) = name.chars().last().filter(|c| matches!(c, '+' | '-')) else {
        return Ok(false);
    };
    let Some(def) = find_option(&name[..name.len() - 1]) else {
        return Ok(false);
    };
    c.option_name = argument.to_string();
    c.option = Some(def);
    c.values.push((last == '+').then(|| argument.to_string()));
    invoke(c, options)?;
    Ok(true)
}

fn parse_bundled_value(
    flag: &str,
    bundle: &str,
    c: &mut Context,
    options: &mut CommandOptions,
) -> Result<bool, ParseError> {
    if flag != "-" {
        return Ok(false);
    }
    for (i, (offset, ch)) in bundle.char_indices().enumerate() {
        let name = ch.to_string();
        let Some(def) = find_option(&name) else {
            if i == 0 {
                return Ok(false);
            }
            return Err(ParseError::option(
                format!("Cannot use unregistered option '{name}' in bundle '{flag}{bundle}'."),
                None,
            ));
        };
        c.option_name = format!("{flag}{name}");
        c.option = Some(def);
        if def.action.takes_value() {
            let rest = &bundle[offset + ch.len_utf8()..];
            parse_value((!rest.is_empty()).then(|| rest.to_string()), c, options)?;
            return Ok(true);
        }
        c.values.push(Some(bundle.to_string()));
        invoke(c, options)?;
    }
    Ok(true)
}

/// 원본 `Option.Invoke`: 동작을 실행하고 문맥을 비운다.
fn invoke(c: &mut Context, options: &mut CommandOptions) -> Result<(), ParseError> {
    let def = c.option.take().expect("invoke without option");
    let name = std::mem::take(&mut c.option_name);
    let values = std::mem::take(&mut c.values);
    // `OptionValueCollection[0]`: 값이 필요한 옵션인데 값이 없으면 OptionException.
    let value = || -> Result<Option<String>, ParseError> {
        match values.first() {
            Some(value) => Ok(value.clone()),
            None => Err(ParseError::option(
                format!("Missing required value for option '{name}'."),
                name.clone(),
            )),
        }
    };
    // `Parse<T>`: `null`이면 기본값, 변환 실패는 OptionException.
    let convert_error = |text: &str, type_name: &str| {
        ParseError::option(
            format!("Could not convert string `{text}' to type {type_name} for option `{name}'."),
            name.clone(),
        )
    };
    match def.action {
        Action::Menu(menu) => options.menu = menu,
        Action::Set(apply) => apply(options),
        Action::Text(apply) => apply(options, value()?),
        Action::TryText(apply) => apply(options, value()?)?,
        Action::Int(apply) => {
            let parsed = match value()? {
                None => 0,
                Some(text) => to_int32(&text).ok_or_else(|| convert_error(&text, "Int32"))?,
            };
            apply(options, parsed);
        }
        Action::Bool(apply) => {
            let parsed = match value()? {
                None => false,
                Some(text) => to_bool(&text).ok_or_else(|| convert_error(&text, "Boolean"))?,
            };
            apply(options, parsed);
        }
    }
    Ok(())
}
