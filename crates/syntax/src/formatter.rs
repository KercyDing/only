use crate::snapshot;

/// Formats an Onlyfile using the built-in deterministic style.
pub fn format_source(source: &str) -> Result<String, String> {
    let parsed = snapshot(source);
    if let Some(diagnostic) = parsed
        .diagnostics()
        .iter()
        .find(|item| item.severity == only_diagnostic::DiagnosticSeverity::Error)
    {
        return Err(diagnostic.message.clone());
    }

    let cst_source = parsed.root().text().to_string();
    let mut output = String::new();
    let mut lines = cst_source.lines().peekable();
    let mut pending_blank = false;
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            pending_blank = true;
            continue;
        }

        let top_level = !line.starts_with([' ', '\t']);
        if top_level
            && (trimmed.starts_with('!') || trimmed.starts_with('#') || trimmed.starts_with('['))
        {
            emit_blank(&mut output, &mut pending_blank);
            output.push_str(&format_top_level(trimmed));
            output.push('\n');
            continue;
        }

        if top_level && looks_like_task_header(trimmed) {
            emit_blank(&mut output, &mut pending_blank);
            let mut header = vec![trimmed.to_owned()];
            while !header_ends(&header) {
                let Some(next) = lines.next() else { break };
                header.push(next.trim().to_owned());
            }
            output.push_str(&format_header(&header));
            output.push('\n');
            while let Some(next) = lines.peek().copied() {
                if !next.starts_with([' ', '\t']) || next.trim().is_empty() {
                    break;
                }
                let body = lines.next().expect("peeked body line");
                let body = body.trim_start_matches([' ', '\t']);
                if let Some(block) = body.strip_prefix('|') {
                    let suffix = block.strip_prefix(' ').unwrap_or(block);
                    output.push_str("    |");
                    if !suffix.is_empty() {
                        output.push(' ');
                        output.push_str(suffix);
                    }
                    output.push('\n');
                } else {
                    output.push_str("    ");
                    output.push_str(body);
                    output.push('\n');
                }
            }
            continue;
        }

        emit_blank(&mut output, &mut pending_blank);
        output.push_str(line.trim_end());
        output.push('\n');
    }

    while output.ends_with("\n\n") {
        output.pop();
    }
    if !output.ends_with('\n') && !output.is_empty() {
        output.push('\n');
    }
    Ok(output)
}

fn format_top_level(line: &str) -> String {
    if line.starts_with("//") || line.starts_with('#') {
        return line.to_owned();
    }
    if line.starts_with('[') {
        let label = line
            .trim_matches(['[', ']'])
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        return format!("[{label}]");
    }
    for keyword in ["!version", "!shell", "!var"] {
        if let Some(rest) = line.strip_prefix(keyword) {
            if keyword == "!var"
                && let Some((name, value)) = rest.trim().split_once('=')
            {
                return format!("{keyword} {} = {}", name.trim(), value.trim());
            }
            return format!("{keyword} {}", rest.trim());
        }
    }
    line.to_owned()
}

fn looks_like_task_header(line: &str) -> bool {
    line.contains('(') && (line.contains(':') || line.ends_with(')'))
}

fn header_ends(lines: &[String]) -> bool {
    let joined = lines.join(" ");
    joined.contains(':') && joined.matches('(').count() == joined.matches(')').count()
}

fn format_header(lines: &[String]) -> String {
    let joined = lines.join(" ");
    let joined = joined.trim().trim_end_matches(':').trim();
    let joined = normalize_header_spacing(joined);
    if joined.len() <= 88 && lines.len() == 1 {
        return format!("{joined}:");
    }
    let Some(open) = joined.find('(') else {
        return format!("{joined}:");
    };
    let Some(close) = joined[open..].find(')').map(|index| open + index) else {
        return format!("{joined}:");
    };
    let name = &joined[..open];
    let params = joined[open + 1..close].trim();
    let tail = joined[close + 1..].trim();
    let mut out = format!("{name}(");
    for param in split_top_level(params, ',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }
        out.push_str("\n    ");
        out.push_str(param);
        out.push(',');
    }
    if !params.is_empty() {
        out.push('\n');
    }
    out.push(')');
    if tail.is_empty() {
        out.push(':');
    } else {
        for clause in split_clauses(tail) {
            out.push_str("\n    ");
            out.push_str(&clause);
        }
        out.push_str("\n:");
    }
    out
}

fn split_clauses(tail: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for character in tail.chars() {
        if quoted {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
            current.push(character);
            continue;
        }
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '?' | '&' if depth == 0 && !current.trim().is_empty() => {
                clauses.push(current.trim().to_owned());
                current.clear();
            }
            _ => {}
        }
        current.push(character);
    }
    if !current.trim().is_empty() {
        clauses.push(current.trim().to_owned());
    }
    clauses
}

fn normalize_header_spacing(input: &str) -> String {
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut pending_space = false;
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if quoted {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            pending_space = false;
            quoted = true;
            output.push(character);
        } else if character == ',' {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push(',');
            pending_space = true;
        } else if character == '&' || (character == '?' && characters.peek() != Some(&'=')) {
            while output.ends_with(' ') {
                output.pop();
            }
            if !output.is_empty() {
                output.push(' ');
            }
            output.push(character);
            pending_space = true;
        } else if character.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !output.is_empty() {
                output.push(' ');
            }
            pending_space = false;
            output.push(character);
        }
    }
    output.trim().to_owned()
}

fn split_top_level(input: &str, separator: char) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for character in input.chars() {
        if quoted {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if character == separator && depth == 0 {
            pieces.push(std::mem::take(&mut current));
        } else {
            current.push(character);
        }
    }
    pieces.push(current);
    pieces
}

fn emit_blank(output: &mut String, pending_blank: &mut bool) {
    if *pending_blank && !output.is_empty() && !output.ends_with("\n\n") {
        output.push('\n');
    }
    *pending_blank = false;
}
