use std::num::NonZeroUsize;

pub fn format_parse_error(text: &str, error: nojson::JsonParseError) -> String {
    let (line_num, column_num) = error
        .get_line_and_column_numbers(text)
        .unwrap_or((NonZeroUsize::MIN, NonZeroUsize::MIN));

    let line = error.get_line(text).unwrap_or("");

    let prev_line = if line_num.get() == 1 {
        None
    } else {
        text.lines().nth(line_num.get() - 2)
    };

    let (display_line, display_column) = format_line_around_position(line, column_num.get());
    let prev_display_line = prev_line.map(|prev| {
        let (truncated, _) = format_line_around_position(prev, column_num.get());
        truncated
    });

    format!(
        "{error}\n\nINPUT:{}\n{line_num:4} |{display_line}\n     |{:>column$} error",
        if let Some(prev) = prev_display_line {
            format!("\n     |{prev}")
        } else {
            "".to_owned()
        },
        "^",
        column = display_column
    )
}

fn format_line_around_position(line: &str, column_pos: usize) -> (String, usize) {
    const MAX_ERROR_LINE_CHARS: usize = 80;

    let chars: Vec<char> = line.chars().collect();
    let max_context = MAX_ERROR_LINE_CHARS / 2;

    let error_pos = column_pos.saturating_sub(1).min(chars.len());
    let start_pos = error_pos.saturating_sub(max_context);
    let end_pos = (error_pos + max_context + 1).min(chars.len());

    let mut result = String::new();
    let mut new_column_pos = error_pos - start_pos + 1;

    if start_pos > 0 {
        result.push_str("...");
        new_column_pos += 3;
    }

    result.push_str(&chars[start_pos..end_pos].iter().collect::<String>());

    if end_pos < chars.len() {
        result.push_str("...");
    }

    (result, new_column_pos)
}
