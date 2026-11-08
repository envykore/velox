//! byte scanner for stripping debug calls and __DEV__ blocks before build.
//! avoids a full AST parse - just enough state to not get fooled by strings/comments.
//! blank-fills instead of deleting so line numbers stay intact in prod errors

use crate::config::BuildConfig;

pub struct StripResult {
    pub source: String,
    pub calls_removed: usize,
    pub dev_blocks_removed: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Normal,
    LineComment,
    BlockComment(u32), // = count
    ShortString(u8),   // quote byte
    LongString(u32),   // = count
}

pub fn process(source: &str, cfg: &BuildConfig) -> StripResult {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut out = bytes.to_vec();
    let mut calls_removed = 0;
    let mut dev_blocks_removed = 0;
    let mut state = State::Normal;
    let mut i = 0;

    while i < len {
        match state {
            State::Normal => {
                if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                    let eq = count_eq(bytes, i + 2);
                    if i + 2 + eq < len && bytes[i + 2 + eq] == b'[' {
                        state = State::BlockComment(eq as u32);
                        i += 3 + eq;
                    } else {
                        state = State::LineComment;
                        i += 2;
                    }
                    continue;
                }

                if bytes[i] == b'[' {
                    let eq = count_eq(bytes, i + 1);
                    if i + 1 + eq < len && bytes[i + 1 + eq] == b'[' {
                        state = State::LongString(eq as u32);
                        i += 2 + eq;
                        continue;
                    }
                }

                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    state = State::ShortString(bytes[i]);
                    i += 1;
                    continue;
                }

                if cfg.strip_debug {
                    if let Some(end) = try_match_dev_block(bytes, i) {
                        blank_fill(&mut out, i, end);
                        dev_blocks_removed += 1;
                        i = end;
                        continue;
                    }

                    let mut stripped = false;
                    for call in &cfg.strip_calls {
                        let cb = call.as_bytes();
                        if bytes[i..].starts_with(cb) {
                            let after = i + cb.len();
                            if after < len && bytes[after] == b'(' && !ident_before(bytes, i) {
                                if let Some(end) = scan_to_close(bytes, after) {
                                    blank_fill(&mut out, i, end);
                                    calls_removed += 1;
                                    i = end;
                                    stripped = true;
                                    break;
                                }
                            }
                        }
                    }
                    if stripped { continue; }
                }

                i += 1;
            }

            State::LineComment => {
                if bytes[i] == b'\n' { state = State::Normal; }
                i += 1;
            }

            State::BlockComment(eq) => {
                if bytes[i] == b']' {
                    let found = count_eq(bytes, i + 1);
                    if found == eq as usize && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                        state = State::Normal;
                        i += 2 + found;
                        continue;
                    }
                }
                i += 1;
            }

            State::ShortString(q) => {
                if bytes[i] == b'\\' { i += 2; continue; }
                if bytes[i] == q || bytes[i] == b'\n' { state = State::Normal; }
                i += 1;
            }

            State::LongString(eq) => {
                if bytes[i] == b']' {
                    let found = count_eq(bytes, i + 1);
                    if found == eq as usize && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                        state = State::Normal;
                        i += 2 + found;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }

    StripResult {
        source: String::from_utf8_lossy(&out).into_owned(),
        calls_removed,
        dev_blocks_removed,
    }
}

fn count_eq(bytes: &[u8], pos: usize) -> usize {
    let mut n = 0;
    while pos + n < bytes.len() && bytes[pos + n] == b'=' { n += 1; }
    n
}

fn blank_fill(out: &mut Vec<u8>, start: usize, end: usize) {
    for b in &mut out[start..end] {
        if *b != b'\n' { *b = b' '; }
    }
}

fn ident_before(bytes: &[u8], pos: usize) -> bool {
    pos > 0 && { let b = bytes[pos - 1]; b.is_ascii_alphanumeric() || b == b'_' }
}

fn ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// walks from opening (to matching), respects strings/comments inside
fn scan_to_close(bytes: &[u8], pos: usize) -> Option<usize> {
    let len = bytes.len();
    let mut depth: u32 = 0;
    let mut state = State::Normal;
    let mut i = pos;

    while i < len {
        match state {
            State::Normal => match bytes[i] {
                b'(' => { depth += 1; i += 1; }
                b')' => {
                    depth -= 1;
                    i += 1;
                    if depth == 0 { return Some(i); }
                }
                b'-' if i + 1 < len && bytes[i + 1] == b'-' => {
                    let eq = count_eq(bytes, i + 2);
                    if i + 2 + eq < len && bytes[i + 2 + eq] == b'[' {
                        state = State::BlockComment(eq as u32);
                        i += 3 + eq;
                    } else {
                        state = State::LineComment;
                        i += 2;
                    }
                }
                b'[' => {
                    let eq = count_eq(bytes, i + 1);
                    if i + 1 + eq < len && bytes[i + 1 + eq] == b'[' {
                        state = State::LongString(eq as u32);
                        i += 2 + eq;
                    } else { i += 1; }
                }
                b'"' | b'\'' => { state = State::ShortString(bytes[i]); i += 1; }
                _ => { i += 1; }
            }
            State::LineComment => {
                if bytes[i] == b'\n' { state = State::Normal; }
                i += 1;
            }
            State::BlockComment(eq) => {
                if bytes[i] == b']' {
                    let found = count_eq(bytes, i + 1);
                    if found == eq as usize && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                        state = State::Normal;
                        i += 2 + found;
                        continue;
                    }
                }
                i += 1;
            }
            State::ShortString(q) => {
                if bytes[i] == b'\\' { i += 2; continue; }
                if bytes[i] == q || bytes[i] == b'\n' { state = State::Normal; }
                i += 1;
            }
            State::LongString(eq) => {
                if bytes[i] == b']' {
                    let found = count_eq(bytes, i + 1);
                    if found == eq as usize && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                        state = State::Normal;
                        i += 2 + found;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }

    None
}

// strips the whole block including the guard. no else support < this is intentional
fn try_match_dev_block(bytes: &[u8], pos: usize) -> Option<usize> {
    let header = b"if __DEV__ then";
    if !bytes[pos..].starts_with(header) || ident_before(bytes, pos) {
        return None;
    }

    let after = pos + header.len();
    if after >= bytes.len() || !bytes[after].is_ascii_whitespace() {
        return None;
    }

    let mut depth: u32 = 1;
    let mut i = after;
    let len = bytes.len();

    while i < len && depth > 0 {
        // skip comments
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let eq = count_eq(bytes, i + 2);
            if i + 2 + eq < len && bytes[i + 2 + eq] == b'[' {
                i += 3 + eq;
                loop {
                    if i >= len { return None; }
                    if bytes[i] == b']' {
                        let found = count_eq(bytes, i + 1);
                        if found == eq && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                            i += 2 + found; break;
                        }
                    }
                    i += 1;
                }
            } else {
                while i < len && bytes[i] != b'\n' { i += 1; }
            }
            continue;
        }

        // skip strings
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            i += 1;
            while i < len {
                if bytes[i] == b'\\' { i += 2; continue; }
                if bytes[i] == q { i += 1; break; }
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'[' {
            let eq = count_eq(bytes, i + 1);
            if i + 1 + eq < len && bytes[i + 1 + eq] == b'[' {
                i += 2 + eq;
                loop {
                    if i >= len { return None; }
                    if bytes[i] == b']' {
                        let found = count_eq(bytes, i + 1);
                        if found == eq && i + 1 + found < len && bytes[i + 1 + found] == b']' {
                            i += 2 + found; break;
                        }
                    }
                    i += 1;
                }
                continue;
            }
        }

        if !ident_before(bytes, i) {
            let rest = &bytes[i..];

            if rest.starts_with(b"end") && (i + 3 >= len || !ident_char(bytes[i + 3])) {
                depth -= 1;
                if depth == 0 { return Some(i + 3); }
                i += 3;
                continue;
            }

            let opens: &[&[u8]] = &[b"if", b"do", b"while", b"for", b"function"];
            let mut opened = false;
            for kw in opens {
                if rest.starts_with(kw) {
                    let after = i + kw.len();
                    if after >= len || !ident_char(bytes[after]) {
                        depth += 1;
                        i += kw.len();
                        opened = true;
                        break;
                    }
                }
            }
            if opened { continue; }
        }

        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuildConfig;

    fn cfg(calls: &[&str]) -> BuildConfig {
        BuildConfig {
            strip_debug: true,
            strip_calls: calls.iter().map(|s| s.to_string()).collect(),
            inline_constants: false,
        }
    }

    #[test]
    fn strips_simple_print() {
        let src = "local x = 1\nprint(\"hello\")\nlocal y = 2\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 1);
        assert!(!r.source.contains("print"));
        assert_eq!(r.source.lines().count(), src.lines().count());
    }

    #[test]
    fn ignores_print_in_string() {
        let src = "local s = \"print(hi)\"\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 0);
    }

    #[test]
    fn ignores_print_in_comment() {
        let src = "-- print(\"debug\")\nlocal x = 1\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 0);
    }

    #[test]
    fn strips_multiline_call() {
        let src = "print(\n  \"hello\",\n  \"world\"\n)\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 1);
        assert_eq!(r.source.lines().count(), src.lines().count());
    }

    #[test]
    fn no_false_positive_my_print() {
        let src = "my_print(\"hello\")\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 0);
    }

    #[test]
    fn strips_dev_block() {
        let src = "local x = 1\nif __DEV__ then\n    print(\"dev only\")\nend\nlocal y = 2\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.dev_blocks_removed, 1);
        assert!(!r.source.contains("__DEV__"));
        assert_eq!(r.source.lines().count(), src.lines().count());
    }

    #[test]
    fn strips_nested_dev_block() {
        let src = "if __DEV__ then\n  if true then\n    print(\"hi\")\n  end\nend\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.dev_blocks_removed, 1);
        assert!(!r.source.contains("__DEV__"));
    }

    #[test]
    fn preserves_non_debug_code() {
        let src = "local function add(a, b)\n  return a + b\nend\n";
        let r = process(src, &cfg(&["print"]));
        assert_eq!(r.calls_removed, 0);
        assert_eq!(r.dev_blocks_removed, 0);
        assert!(r.source.contains("return a + b"));
    }
}