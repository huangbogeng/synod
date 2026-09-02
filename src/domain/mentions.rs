use std::collections::HashSet;

/// Parses ordered, deduplicated mentions outside fenced and inline Markdown code.
#[must_use]
pub fn parse_mentions(markdown: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    let mut seen = HashSet::new();
    let mut fence: Option<char> = None;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let fence_marker = if trimmed.starts_with("```") {
            Some('`')
        } else if trimmed.starts_with("~~~") {
            Some('~')
        } else {
            None
        };

        if let Some(marker) = fence_marker {
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }

        parse_line(line, &mut mentions, &mut seen);
    }
    mentions
}

fn parse_line(line: &str, mentions: &mut Vec<String>, seen: &mut HashSet<String>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut in_inline_code = false;

    while index < bytes.len() {
        match bytes[index] {
            b'`' => {
                in_inline_code = !in_inline_code;
                index += 1;
            }
            b'@' if !in_inline_code && is_mention_boundary(bytes, index) => {
                let start = index + 1;
                let mut end = start;
                while end < bytes.len() && is_handle_byte(bytes[end]) {
                    end += 1;
                }
                if end > start && end - start <= 39 {
                    let handle = line[start..end].to_ascii_lowercase();
                    if valid_handle_edges(handle.as_bytes()) && seen.insert(handle.clone()) {
                        mentions.push(handle);
                    }
                }
                index = end.max(index + 1);
            }
            _ => index += 1,
        }
    }
}

fn is_mention_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0
        || (!bytes[index - 1].is_ascii_alphanumeric() && !matches!(bytes[index - 1], b'_' | b'\\'))
}

const fn is_handle_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

fn valid_handle_edges(bytes: &[u8]) -> bool {
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mentions_are_ordered_deduplicated_and_lowercase() {
        assert_eq!(
            parse_mentions("@Architect ask @security-team, then @architect again"),
            ["architect", "security-team"]
        );
    }

    #[test]
    fn code_email_and_escaped_mentions_are_inert() {
        let text = "mail a@b.com `@inline` \\@escaped\n```rust\n@fenced\n```\n@active";
        assert_eq!(parse_mentions(text), ["active"]);
    }
}
