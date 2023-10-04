use parser_combinator;

/// tokenize program code
fn tokenize(line: &str) -> Vec<(usize, String)> {
    use std::mem::take;

    let len = line.len();
    let mut result = vec![];
    let mut chars = line.chars().peekable();
    let mut token = String::new();

    while let Some(c) = chars.next() {
        match c {
            // 空白読み飛ばし
            c if c.is_whitespace() => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
            }
            // -> は 2 文字トークン
            c if c == '-' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
                if let Some(&next_c) = chars.peek() {
                    if next_c == '>' {
                        chars.next();
                        let cc = String::from_utf8(vec![c as u8, next_c as u8]).unwrap();
                        result.push((len - chars.clone().count() - 2, cc));
                        continue;
                    }
                }

                result.push((len - chars.clone().count() - 1, c.to_string()));
            }
            // これらは 1 文字トークン
            '{' | '}' | '(' | ')' | '<' | '>' | ':' | ';' | ',' | '*' => {
                if token.len() > 0 {
                    result.push((
                        len - chars.clone().count() - token.len() - 1,
                        take(&mut token),
                    ));
                }
                result.push((len - chars.clone().count() - 1, c.to_string()));
            }
            _ => token.push(c),
        }
    }
    if token.len() > 0 {
        result.push((len - chars.clone().count() - token.len(), token.to_string()));
    }
    result
}
