use gpui::{
    AnyElement, Font, FontStyle, FontWeight, IntoElement, ParentElement, SharedString, Styled,
    StyledText, TextRun, div,
};

/// Minimal Pango-subset renderer: supports <b>, <i>, <br/>, HTML entities.
pub fn render_pango(
    content: &str,
    theme: &std::sync::Arc<crate::app::theme::ThemeData>,
) -> AnyElement {
    let (final_text, runs) = parse_pango(content, theme);

    div()
        .w_full()
        .overflow_hidden()
        .child(StyledText::new(SharedString::from(final_text)).with_runs(runs))
        .into_any_element()
}

fn unescape_html(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&apos;", "'")
}

fn get_attribute(tag: &str, attr: &str) -> Option<SharedString> {
    let pattern = format!("{}='", attr);
    if let Some(start) = tag.find(&pattern) {
        let remainder = &tag[start + pattern.len()..];
        if let Some(end) = remainder.find('\'') {
            return Some(remainder[..end].to_string().into());
        }
    }
    // Try double quotes too
    let pattern_dq = format!("{}=\"", attr);
    if let Some(start) = tag.find(&pattern_dq) {
        let remainder = &tag[start + pattern_dq.len()..];
        if let Some(end) = remainder.find('"') {
            return Some(remainder[..end].to_string().into());
        }
    }
    None
}

/// Tokenise `content` into alternating text/tag slices and build
/// (final_text, runs).  Returns empty runs if there is no markup.
fn parse_pango(
    content: &str,
    theme: &std::sync::Arc<crate::app::theme::ThemeData>,
) -> (String, Vec<TextRun>) {
    let mut final_text = String::with_capacity(content.len());
    let mut runs: Vec<TextRun> = Vec::new();
    let mut bold_depth: usize = 0;
    let mut italic_depth: usize = 0;
    let mut family_stack: Vec<SharedString> = Vec::new();

    let mut rest = content;

    while !rest.is_empty() {
        if let Some(tag_start) = rest.find('<') {
            if tag_start > 0 {
                let text = unescape_html(&rest[..tag_start]);
                push_run(
                    &text,
                    bold_depth,
                    italic_depth,
                    family_stack.last().cloned(),
                    theme,
                    &mut final_text,
                    &mut runs,
                );
            }
            rest = &rest[tag_start..];

            // Find closing >
            if let Some(tag_end) = rest.find('>') {
                let tag = &rest[..=tag_end];
                let inner = tag[1..tag.len() - 1].trim();
                let inner_lower = inner.to_ascii_lowercase();

                if inner_lower == "b" {
                    bold_depth += 1;
                } else if inner_lower == "/b" {
                    bold_depth = bold_depth.saturating_sub(1);
                } else if inner_lower == "i" {
                    italic_depth += 1;
                } else if inner_lower == "/i" {
                    italic_depth = italic_depth.saturating_sub(1);
                } else if inner_lower == "br" || inner_lower == "br/" || inner_lower == "br /" {
                    push_run(
                        "\n\n",
                        bold_depth,
                        italic_depth,
                        family_stack.last().cloned(),
                        theme,
                        &mut final_text,
                        &mut runs,
                    );
                } else if inner_lower.starts_with("span") {
                    if let Some(f) = get_attribute(inner, "font_desc") {
                        family_stack.push(f);
                    }
                } else if inner_lower == "/span" {
                    family_stack.pop();
                } else {
                    // Unknown tag — emit as literal
                    let text = unescape_html(tag);
                    push_run(
                        &text,
                        bold_depth,
                        italic_depth,
                        family_stack.last().cloned(),
                        theme,
                        &mut final_text,
                        &mut runs,
                    );
                }
                rest = &rest[tag_end + 1..];
            } else {
                // Unclosed <
                let text = unescape_html(rest);
                push_run(
                    &text,
                    bold_depth,
                    italic_depth,
                    family_stack.last().cloned(),
                    theme,
                    &mut final_text,
                    &mut runs,
                );
                break;
            }
        } else {
            let text = unescape_html(rest);
            push_run(
                &text,
                bold_depth,
                italic_depth,
                family_stack.last().cloned(),
                theme,
                &mut final_text,
                &mut runs,
            );
            break;
        }
    }

    (final_text, runs)
}

fn push_run(
    text: &str,
    bold_depth: usize,
    italic_depth: usize,
    family: Option<SharedString>,
    theme: &std::sync::Arc<crate::app::theme::ThemeData>,
    final_text: &mut String,
    runs: &mut Vec<TextRun>,
) {
    if text.is_empty() {
        return;
    }

    let start = final_text.len();
    final_text.push_str(text);
    let len = final_text.len() - start;

    let target_family = match family {
        Some(ref f) if f.as_ref() == "monospace" => theme.monospace.clone(),
        Some(f) => f,
        None => theme.font_family.clone(),
    };

    let target_weight = if bold_depth > 0 {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    let target_style = if italic_depth > 0 {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };

    // Merge adjacent run if style AND font family are identical
    if let Some(last) = runs.last_mut() {
        let same_bold = last.font.weight == target_weight;
        let same_italic = last.font.style == target_style;
        let same_family = last.font.family == target_family;

        if same_bold && same_italic && same_family {
            last.len += len;
            return;
        }
    }

    runs.push(TextRun {
        len,
        color: if bold_depth > 0 {
            theme.primary_text
        } else {
            theme.secondary_text
        },
        font: Font {
            family: target_family,
            weight: target_weight,
            style: target_style,
            ..Default::default()
        },
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{FontStyle, FontWeight};
    use std::sync::Arc;

    fn dummy_theme() -> Arc<crate::app::theme::ThemeData> {
        Arc::new(crate::app::theme::ThemeData::dark())
    }

    fn parse(s: &str) -> (String, Vec<TextRun>) {
        parse_pango(s, &dummy_theme())
    }

    #[test]
    fn plain_text_produces_one_run() {
        let (text, runs) = parse("hello world");
        assert_eq!(text, "hello world");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, 11);
        assert_eq!(runs[0].font.weight, FontWeight::NORMAL);
    }

    #[test]
    fn bold_tag_sets_weight() {
        let (text, runs) = parse("<b>bold</b>");
        assert_eq!(text, "bold");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].font.weight, FontWeight::BOLD);
    }

    #[test]
    fn italic_tag_sets_style() {
        let (text, runs) = parse("<i>slanted</i>");
        assert_eq!(text, "slanted");
        assert_eq!(runs[0].font.style, FontStyle::Italic);
    }

    #[test]
    fn mixed_bold_and_italic() {
        let (text, runs) = parse("<b><i>both</i></b>");
        assert_eq!(text, "both");
        assert_eq!(runs[0].font.weight, FontWeight::BOLD);
        assert_eq!(runs[0].font.style, FontStyle::Italic);
    }

    #[test]
    fn bold_wrapping_plain_text() {
        let (text, runs) = parse("before <b>bold</b> after");
        assert_eq!(text, "before bold after");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].font.weight, FontWeight::NORMAL);
        assert_eq!(runs[1].font.weight, FontWeight::BOLD);
        assert_eq!(runs[2].font.weight, FontWeight::NORMAL);
        // byte lengths
        assert_eq!(runs[0].len, 7); // "before "
        assert_eq!(runs[1].len, 4); // "bold"
        assert_eq!(runs[2].len, 6); // " after"
    }

    #[test]
    fn br_tag_inserts_newline() {
        let (text, runs) = parse("line1<br/>line2");
        assert_eq!(text, "line1\n\nline2");
        let total_run_len: usize = runs.iter().map(|r| r.len).sum();
        assert_eq!(total_run_len, text.len());

        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn br_without_slash_also_works() {
        let (text, _) = parse("a<br>b");
        assert_eq!(text, "a\n\nb");
    }

    #[test]
    fn html_entities_unescaped() {
        let (text, _) = parse("a &amp; b &lt;c&gt; &quot;d&quot;");
        assert_eq!(text, "a & b <c> \"d\"");
    }

    #[test]
    fn nbsp_entity() {
        let (text, _) = parse("a&nbsp;b");
        assert_eq!(text, "a b");
    }

    #[test]
    fn empty_string() {
        let (text, runs) = parse("");
        assert_eq!(text, "");
        assert!(runs.is_empty());
    }

    #[test]
    fn unclosed_tag_treated_as_text() {
        let (text, _) = parse("hello <b world");
        assert!(text.contains("hello"));
    }

    #[test]
    fn unknown_tag_emitted_as_literal() {
        let (text, _) = parse("hello <stan>world</stan>");
        assert!(text.contains("<stan>"));
        assert!(text.contains("world"));
        assert!(text.contains("</stan>"));
    }

    #[test]
    fn nested_bold() {
        let (text, runs) = parse("<b>a<b>b</b>c</b>");
        assert_eq!(text, "abc");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].font.weight, FontWeight::BOLD);
    }

    #[test]
    fn adjacent_same_style_runs_are_merged() {
        let (text, runs) = parse("hello <!-- comment --> world");
        let total_len: usize = runs.iter().map(|r| r.len).sum();
        assert_eq!(total_len, text.len());
    }

    #[test]
    fn run_lengths_sum_to_text_length() {
        let cases = [
            "plain",
            "<b>bold</b> normal <i>italic</i>",
            "a &amp; <b>b &lt; <i>c</i></b> d",
            "<br/><br/>",
            "",
        ];
        let theme = dummy_theme();
        for case in &cases {
            let (text, runs) = parse_pango(case, &theme);
            let total: usize = runs.iter().map(|r| r.len).sum();
            assert_eq!(
                total,
                text.len(),
                "run lengths don't sum to text length for: {case:?}"
            );
        }
    }

    #[test]
    fn span_applies_font_family() {
        let (text, runs) = parse("normal <span font_desc='Courier'>monospace</span> normal");
        assert_eq!(text, "normal monospace normal");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1].font.family.as_ref(), "Courier");
    }

    #[test]
    fn nested_spans_restore_family() {
        let (_text, runs) =
            parse("<span font_desc='A'>outer <span font_desc='B'>inner</span> outer</span>");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].font.family.as_ref(), "A");
        assert_eq!(runs[1].font.family.as_ref(), "B");
        assert_eq!(runs[2].font.family.as_ref(), "A");
    }
}
