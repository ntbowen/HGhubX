//! Literal-array parsing for publisher scripts (data only, never executed).

const PUBLISH_ARRAY_MAX: usize = 256;

#[derive(Debug, PartialEq)]
enum JsToken<'a> {
    Ident(&'a str),
    Quoted(&'a str),
    Symbol(u8),
    Other,
}

/// A conservative lexer: comments, strings, templates and regular expressions
/// cannot introduce declarations. No remote JavaScript is ever evaluated.
fn js_tokens(source: &str) -> Result<Vec<JsToken<'_>>, String> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if bytes[at..].starts_with(b"//") {
            at += 2;
            while at < bytes.len() && bytes[at] != 10 {
                at += 1;
            }
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            at += 2;
            let Some(end) = source[at..].find("*/") else {
                return Err("发布脚本注释未闭合".into());
            };
            at += end + 2;
            continue;
        }
        if matches!(byte, 39 | 34 | 96) {
            let start = at;
            at += 1;
            while at < bytes.len() && bytes[at] != byte {
                if bytes[at] == 92 {
                    at += 1;
                }
                at += 1;
            }
            if at >= bytes.len() {
                return Err("发布脚本字符串未闭合".into());
            }
            at += 1;
            out.push(if byte == 96 {
                JsToken::Other
            } else {
                JsToken::Quoted(&source[start..at])
            });
            continue;
        }
        // Division is deliberately treated as a regex as well. Publisher data
        // requires no arithmetic; ambiguous input therefore fails closed.
        if byte == b'/' {
            at += 1;
            let mut in_class = false;
            while at < bytes.len() {
                match bytes[at] {
                    92 => {
                        at += 2;
                        continue;
                    }
                    b'[' => in_class = true,
                    b']' => in_class = false,
                    b'/' if !in_class => break,
                    10 | 13 => return Err("发布脚本包含不支持的表达式".into()),
                    _ => {}
                }
                at += 1;
            }
            if at >= bytes.len() {
                return Err("发布脚本表达式未闭合".into());
            }
            at += 1;
            while at < bytes.len() && bytes[at].is_ascii_alphabetic() {
                at += 1;
            }
            out.push(JsToken::Other);
            continue;
        }
        if byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$') {
            let start = at;
            at += 1;
            while at < bytes.len()
                && (bytes[at].is_ascii_alphanumeric() || matches!(bytes[at], b'_' | b'$'))
            {
                at += 1;
            }
            out.push(JsToken::Ident(&source[start..at]));
        } else {
            out.push(JsToken::Symbol(byte));
            at += 1;
        }
    }
    Ok(out)
}

fn js_string(raw: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = raw[1..raw.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch != char::from(92) {
            if ch.is_control() {
                return Err("镜像字符串含控制字符".into());
            }
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some(ch) if matches!(ch as u32, 92 | 39 | 34 | 47) => result.push(ch),
            Some('x') => {
                let code: String = chars.by_ref().take(2).collect();
                let byte = u8::from_str_radix(&code, 16)
                    .ok()
                    .filter(|_| code.len() == 2)
                    .ok_or("镜像字符串转义无效")?;
                result.push(char::from(byte));
            }
            Some('u') => {
                let code: String = chars.by_ref().take(4).collect();
                let number = u32::from_str_radix(&code, 16)
                    .ok()
                    .filter(|_| code.len() == 4)
                    .ok_or("镜像字符串转义无效")?;
                result.push(char::from_u32(number).ok_or("镜像字符串转义无效")?);
            }
            _ => return Err("镜像字符串含不支持的转义".into()),
        }
    }
    Ok(result)
}

/// Only literal top-level var/let/const arrays are accepted. Malformed
/// subdomains must not fall back to trusting the bare domains.
pub(crate) fn js_array(source: &str, name: &str) -> Result<Option<Vec<String>>, String> {
    let tokens = js_tokens(source)?;
    let mut depth = 0usize;
    let mut result = None;
    for (at, token) in tokens.iter().enumerate() {
        let declaration = matches!(token, JsToken::Ident("var" | "let" | "const"));
        if depth == 0 && declaration && tokens.get(at + 1) == Some(&JsToken::Ident(name)) {
            if result.is_some() {
                return Err(format!("发布脚本重复声明 {name}"));
            }
            if tokens.get(at + 2) != Some(&JsToken::Symbol(b'='))
                || tokens.get(at + 3) != Some(&JsToken::Symbol(b'['))
            {
                return Err(format!("发布脚本的 {name} 必须是字符串数组"));
            }
            let mut cursor = at + 4;
            let mut values = Vec::new();
            while tokens.get(cursor) != Some(&JsToken::Symbol(b']')) {
                let Some(JsToken::Quoted(raw)) = tokens.get(cursor) else {
                    return Err(format!("发布脚本的 {name} 含非字符串值"));
                };
                if values.len() >= PUBLISH_ARRAY_MAX {
                    return Err(format!("{name} exceeds the publisher array size limit"));
                }
                values.push(js_string(raw)?);
                cursor += 1;
                match tokens.get(cursor) {
                    Some(JsToken::Symbol(b',')) => cursor += 1,
                    Some(JsToken::Symbol(b']')) => {}
                    _ => return Err(format!("发布脚本的 {name} 数组格式无效")),
                }
            }
            if !matches!(tokens.get(cursor + 1), None | Some(JsToken::Symbol(b';'))) {
                return Err(format!("发布脚本的 {name} 不能使用动态表达式"));
            }
            result = Some(values);
        }
        match token {
            JsToken::Symbol(b'{' | b'(' | b'[') => depth += 1,
            JsToken::Symbol(b'}' | b')' | b']') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(result)
}
