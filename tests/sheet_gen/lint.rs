//! Static formula checks for the GEN sheets (a stand-in for LibreOffice recalculation):
//! string literals stripped, sheet references and defined-name references extracted.

/// Formula text outside string literals.
pub fn code(f: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    for ch in f.chars() {
        if ch == '"' {
            in_str = !in_str;
            out.push(' ');
        } else if !in_str {
            out.push(ch);
        }
    }
    out
}

/// Sheet names a formula refers to (`Name!` and `'Name x'!`).
pub fn sheet_refs(f: &str) -> Vec<String> {
    let c = code(f);
    let mut v = Vec::new();
    let b = c.as_bytes();
    for (i, ch) in c.char_indices() {
        if ch != '!' {
            continue;
        }
        if i > 0 && b[i - 1] == b'\'' {
            let start = c[..i - 1].rfind('\'').unwrap();
            v.push(c[start + 1..i - 1].to_string());
        } else {
            let start = c[..i].rfind(|x: char| !(x.is_ascii_alphanumeric() || x == '_')).map_or(0, |s| s + 1);
            v.push(c[start..i].to_string());
        }
    }
    v
}

/// Bare identifiers that are neither functions, cells, sheet names nor booleans: defined names.
pub fn name_refs(f: &str) -> Vec<String> {
    let c = code(f) + " ";
    let mut v = Vec::new();
    let mut cur = String::new();
    let mut prev = ' ';
    let chars: Vec<char> = c.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        if ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '.' {
            if cur.is_empty() {
                prev = if i > 0 { chars[i - 1] } else { ' ' };
            }
            cur.push(*ch);
            continue;
        }
        if !cur.is_empty() {
            let next = *ch;
            let is_cell = {
                let t = cur.trim_start_matches('$');
                let letters = t.chars().take_while(char::is_ascii_alphabetic).count();
                let rest = t[letters..].trim_start_matches('$');
                letters > 0 && letters <= 3 && !rest.is_empty() && rest.chars().all(|x| x.is_ascii_digit())
            };
            let is_num = cur.chars().next().unwrap().is_ascii_digit();
            if next != '('
                && next != '!'
                && prev != '$'
                && prev != '!'
                && !is_cell
                && !is_num
                && cur != "TRUE"
                && cur != "FALSE"
            {
                v.push(cur.clone());
            }
            cur.clear();
        }
    }
    v
}
