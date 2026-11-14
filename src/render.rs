use pulldown_cmark::Alignment;
use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier, Style as RStyle},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::parse::{Block, Slide, Span, Style};

fn rstyle(s: Style) -> RStyle {
    let mut r = RStyle::default();
    if s.bold {
        r = r.add_modifier(Modifier::BOLD);
    }
    if s.italic {
        r = r.add_modifier(Modifier::ITALIC);
    }
    if s.code {
        r = r.fg(Color::Cyan);
    }
    r
}

fn title_style() -> RStyle {
    RStyle::default().add_modifier(Modifier::BOLD)
}

pub struct Layout {
    pub title_rows: usize,
    pub content_rows: usize,
    pub lines: Vec<Vec<(String, Style)>>,
}

impl Layout {
    pub fn new(width: u16, height: u16, slide: &Slide, visible: usize) -> Self {
        let w = width as usize;
        let h = height as usize;
        let title_rows = if slide.stack.is_empty() {
            0
        } else {
            slide.stack.len() + 1
        };
        let content_rows = h.saturating_sub(title_rows + 1);

        let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
        for (i, b) in slide.blocks.iter().take(visible).enumerate() {
            if i > 0 {
                lines.push(Vec::new());
            }
            lines.extend(layout_block(b, w));
        }

        Layout { title_rows, content_rows, lines }
    }

    pub fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.content_rows)
    }
}

fn layout_block(b: &Block, width: usize) -> Vec<Vec<(String, Style)>> {
    match b {
        Block::Para { quote, spans } => {
            let p = "│ ".repeat(*quote);
            wrap_spans(spans, &p, &p, width)
        }
        Block::Bullet { quote, depth, marker, spans } => {
            let q = "│ ".repeat(*quote);
            let indent = format!("{}{}", "  ".repeat(*depth), marker);
            let first = format!("{}{}", q, indent);
            let cont = format!(
                "{}{}",
                q,
                " ".repeat(UnicodeWidthStr::width(indent.as_str())),
            );
            wrap_spans(spans, &first, &cont, width)
        }
        Block::Code(lines) => {
            let st = Style { code: true, ..Default::default() };
            lines
                .iter()
                .map(|l| vec![(format!("  {l}"), st)])
                .collect()
        }
        Block::Rule => vec![vec![("─".repeat(width.min(60)), Style::default())]],
        Block::Table { aligns, head, rows } => layout_table(aligns, head, rows, width),
    }
}

fn to_words(spans: &[Span]) -> Vec<(String, Style)> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut cur_style = Style::default();
    let mut started = false;

    for s in spans {
        for ch in s.text.chars() {
            if ch.is_whitespace() {
                if started {
                    words.push((std::mem::take(&mut cur), cur_style));
                    started = false;
                }
            } else {
                if !started {
                    cur_style = s.style;
                    started = true;
                }
                cur.push(ch);
            }
        }
    }
    if started {
        words.push((cur, cur_style));
    }
    words
}

fn wrap_words(
    words: &[(String, Style)],
    first: &str,
    cont: &str,
    width: usize,
) -> Vec<Vec<(String, Style)>> {
    let mut out: Vec<Vec<(String, Style)>> = Vec::new();
    let mut cur: Vec<(String, Style)> = Vec::new();
    let mut prefix = first.to_string();
    let mut cur_w = UnicodeWidthStr::width(prefix.as_str());

    let flush = |out: &mut Vec<Vec<(String, Style)>>,
                 cur: &mut Vec<(String, Style)>,
                 prefix: &mut String,
                 cur_w: &mut usize| {
        let mut line = vec![(
            std::mem::replace(prefix, cont.to_string()),
            Style::default(),
        )];
        line.append(cur);
        out.push(line);
        *cur_w = UnicodeWidthStr::width(prefix.as_str());
    };

    for (wd, st) in words {
        let ww = UnicodeWidthStr::width(wd.as_str());

        // Soft break before the word if it would overflow.
        if !cur.is_empty() && cur_w + 1 + ww > width {
            flush(&mut out, &mut cur, &mut prefix, &mut cur_w);
        }
        if !cur.is_empty() {
            // Inter-word space picks up the *previous* word's style, so
            // `**bold** plain` no longer bolds the trailing space.
            let space_style = cur.last().map(|(_, s)| *s).unwrap_or_default();
            cur.push((" ".into(), space_style));
            cur_w += 1;
        }

        if cur_w + ww <= width {
            cur.push((wd.clone(), *st));
            cur_w += ww;
            continue;
        }

        // Doesn't fit even after a flush: break the word into chunks that
        // do, so `draw` never has to truncate anything.
        let mut remaining: &str = wd.as_str();
        while !remaining.is_empty() {
            let avail = width.saturating_sub(cur_w);
            if avail == 0 {
                flush(&mut out, &mut cur, &mut prefix, &mut cur_w);
                continue;
            }
            let mut chunk_w = 0usize;
            let mut split = 0usize;
            for (i, ch) in remaining.char_indices() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                if chunk_w + cw > avail && i > 0 {
                    break;
                }
                chunk_w += cw;
                split = i + ch.len_utf8();
            }
            // Guaranteed non-zero: the `i > 0` guard above always allows the
            // first character through even if it overflows on its own.
            let chunk = remaining[..split].to_string();
            remaining = &remaining[split..];
            cur.push((chunk, *st));
            cur_w += chunk_w;
            if !remaining.is_empty() {
                flush(&mut out, &mut cur, &mut prefix, &mut cur_w);
            }
        }
    }

    if !cur.is_empty() || !prefix.is_empty() {
        let mut line = vec![(prefix, Style::default())];
        line.append(&mut cur);
        out.push(line);
    }
    out
}

fn wrap_spans(spans: &[Span], first: &str, cont: &str, width: usize) -> Vec<Vec<(String, Style)>> {
    wrap_words(&to_words(spans), first, cont, width)
}

fn measure_row(row: &[Vec<Span>], nat: &mut [usize], floor: &mut [usize]) {
    for (c, cell) in row.iter().enumerate() {
        if c >= nat.len() {
            break;
        }
        let words = to_words(cell);
        if words.is_empty() {
            continue;
        }
        let full: usize = words
            .iter()
            .map(|(w, _)| UnicodeWidthStr::width(w.as_str()))
            .sum::<usize>()
            + words.len().saturating_sub(1);
        if full > nat[c] {
            nat[c] = full;
        }
        let longest = words
            .iter()
            .map(|(w, _)| UnicodeWidthStr::width(w.as_str()))
            .max()
            .unwrap_or(0);
        if longest > floor[c] {
            floor[c] = longest;
        }
    }
}

fn layout_table(
    aligns: &[Alignment],
    head: &[Vec<Span>],
    rows: &[Vec<Vec<Span>>],
    width: usize,
) -> Vec<Vec<(String, Style)>> {
    let ncols = head
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if ncols == 0 {
        return vec![];
    }

    let mut nat = vec![0usize; ncols];
    let mut floor = vec![1usize; ncols];
    measure_row(head, &mut nat, &mut floor);
    for r in rows {
        measure_row(r, &mut nat, &mut floor);
    }

    let overhead = 3 * ncols + 1;
    let avail = width.saturating_sub(overhead);

    let mut widths: Vec<usize> = nat.clone();
    if widths.iter().sum::<usize>() > avail {
        let floor_sum: usize = floor.iter().sum();
        if floor_sum >= avail {
            widths = floor.clone();
        } else {
            let extras: Vec<usize> = nat
                .iter()
                .zip(floor.iter())
                .map(|(&n, &f)| n.saturating_sub(f))
                .collect();
            let extra_sum: usize = extras.iter().sum();
            let budget = avail - floor_sum;

            if extra_sum == 0 {
                widths = floor.clone();
            } else {
                let mut given: Vec<usize> = extras
                    .iter()
                    .map(|&e| e * budget / extra_sum)
                    .collect();
                let mut leftover = budget - given.iter().sum::<usize>();

                let mut order: Vec<usize> = (0..ncols).collect();
                order.sort_by_key(|&c| std::cmp::Reverse((extras[c] * budget) % extra_sum));
                for &c in &order {
                    if leftover == 0 {
                        break;
                    }
                    if given[c] < extras[c] {
                        given[c] += 1;
                        leftover -= 1;
                    }
                }

                widths = (0..ncols).map(|c| floor[c] + given[c]).collect();
            }
        }
    }

    let wrap_row = |row: &[Vec<Span>]| -> Vec<Vec<Vec<(String, Style)>>> {
        (0..ncols)
            .map(|c| {
                let empty: Vec<Span> = Vec::new();
                let cell = row.get(c).unwrap_or(&empty);
                let lines = wrap_spans(cell, "", "", widths[c]);
                if lines.is_empty() {
                    vec![vec![]]
                } else {
                    lines
                }
            })
            .collect()
    };
    let head_lines = wrap_row(head);
    let body_lines: Vec<_> = rows.iter().map(|r| wrap_row(r)).collect();

    let border_style = Style::default();
    let mut out: Vec<Vec<(String, Style)>> = Vec::new();

    let rule = |left: char, mid: char, right: char| -> String {
        let mut s = String::new();
        s.push(left);
        for (i, w) in widths.iter().enumerate() {
            if i > 0 {
                s.push(mid);
            }
            for _ in 0..(w + 2) {
                s.push('─');
            }
        }
        s.push(right);
        s
    };
    let push_rule = |out: &mut Vec<Vec<(String, Style)>>, s: String| {
        out.push(vec![(s, border_style)]);
    };

    let emit_row = |out: &mut Vec<Vec<(String, Style)>>,
                    cells: &[Vec<Vec<(String, Style)>>]| {
        let h = cells.iter().map(|c| c.len()).max().unwrap_or(1);
        for line_i in 0..h {
            let mut line: Vec<(String, Style)> = Vec::new();
            line.push(("│ ".into(), border_style));
            for c in 0..ncols {
                if c > 0 {
                    line.push((" │ ".into(), border_style));
                }
                let empty: Vec<(String, Style)> = Vec::new();
                let cell_line = cells[c].get(line_i).unwrap_or(&empty);
                line.extend(pad_cell(cell_line, widths[c], aligns.get(c).copied()));
            }
            line.push((" │".into(), border_style));
            out.push(line);
        }
    };

    push_rule(&mut out, rule('┌', '┬', '┐'));
    emit_row(&mut out, &head_lines);
    push_rule(&mut out, rule('├', '┼', '┤'));
    for r in &body_lines {
        emit_row(&mut out, r);
    }
    push_rule(&mut out, rule('└', '┴', '┘'));

    out
}

fn pad_cell(
    content: &[(String, Style)],
    w: usize,
    align: Option<Alignment>,
) -> Vec<(String, Style)> {
    let len: usize = content
        .iter()
        .map(|(t, _)| UnicodeWidthStr::width(t.as_str()))
        .sum();
    let extra = w.saturating_sub(len);
    let (l, r) = match align {
        Some(Alignment::Right) => (extra, 0),
        Some(Alignment::Center) => (extra / 2, extra - extra / 2),
        _ => (0, extra),
    };
    let mut out = Vec::new();
    if l > 0 {
        out.push((" ".repeat(l), Style::default()));
    }
    out.extend(content.iter().cloned());
    if r > 0 {
        out.push((" ".repeat(r), Style::default()));
    }
    out
}

fn clear(buf: &mut Buffer, area: Rect) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(c) = buf.cell_mut((x, y)) {
                *c = Cell::default();
            }
        }
    }
}

fn draw_str(buf: &mut Buffer, area: Rect, x: i32, y: u16, s: &str, style: RStyle) {
    let x0 = area.x as i32;
    let x1 = (area.x + area.width) as i32;
    let mut cx = x;
    for ch in s.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0) as i32;
        // Zero-width (combining) marks can't compose in a cell buffer; drop them.
        if cw == 0 {
            continue;
        }
        if cx >= x1 {
            break;
        }
        // Don't split a wide glyph at the right edge.
        if cx >= x0 && cx + cw <= x1 {
            if let Some(c) = buf.cell_mut((cx as u16, y)) {
                c.set_char(ch).set_style(style);
            }
        }
        cx += cw;
    }
}

fn ease_out(p: f32) -> f32 {
    let q = 1.0 - p.clamp(0.0, 1.0);
    1.0 - q * q * q
}

pub struct AnimFrameView<'a> {
    pub title_progress: &'a [Option<f32>],
}

pub fn draw(
    buf: &mut Buffer,
    area: Rect,
    slide: &Slide,
    title_progress: &[Option<f32>],
    visible_blocks: usize,
    scroll: usize,
    layout: &Layout,
    idx: usize,
    total: usize,
) {
    clear(buf, area);
    let w = area.width as usize;

    // --- title stack ---
    for (i, t) in slide.stack.iter().enumerate() {
        let indent = t.level.saturating_sub(1);
        let col: i32 = match title_progress.get(i).copied().flatten() {
            None => indent as i32,
            Some(p) => {
                let e = ease_out(p);
                let start = w as f32 + 1.0;
                (start + (indent as f32 - start) * e).round() as i32
            }
        };
        draw_str(buf, area, area.x as i32 + col, area.y + i as u16, &t.text, title_style());
    }

    // --- separator (skip if it would collide with the footer row) ---
    if !slide.stack.is_empty() && (slide.stack.len() + 1) < area.height as usize {
        let y = area.y + slide.stack.len() as u16;
        let sep = RStyle::default().add_modifier(Modifier::DIM);
        for x in area.x..area.x + area.width {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_char('─').set_style(sep);
            }
        }
    }

    // --- content ---
    let _ = visible_blocks;
    for (row, line) in layout
        .lines
        .iter()
        .skip(scroll)
        .take(layout.content_rows)
        .enumerate()
    {
        let y = area.y + (layout.title_rows + row) as u16;
        if y >= area.y + area.height {
            break;
        }
        let mut x = area.x as usize;
        let xmax = (area.x + area.width) as usize;
        'line: for (text, st) in line {
            let s = rstyle(*st);
            for ch in text.chars() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                if cw == 0 {
                    continue;
                }
                // `wrap_words` never produces an over-wide line, so this
                // guard is a defensive no-op in normal use.
                if x + cw > xmax {
                    break 'line;
                }
                if let Some(c) = buf.cell_mut((x as u16, y)) {
                    c.set_char(ch).set_style(s);
                }
                x += cw;
            }
        }
    }

    // --- footer ---
    if area.height >= 1 {
        let s = format!(" poorerpoint  {}/{} ", idx + 1, total);
        let s_w = UnicodeWidthStr::width(s.as_str()) as u16;
        let fx = (area.x + area.width).saturating_sub(s_w);
        let st = RStyle::default().add_modifier(Modifier::DIM);
        draw_str(buf, area, fx as i32, area.y + area.height - 1, &s, st);
    }
}








///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////







#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn text(lines: &[Vec<(String, Style)>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.iter().map(|(s, _)| s.as_str()).collect::<String>())
            .collect()
    }

    fn buf_to_string(buf: &Buffer, area: Rect) -> String {
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    // ---------- wrapping ----------

    #[test]
    fn to_words_splits_on_whitespace() {
        let spans = vec![Span { text: "hello world".into(), style: Style::default() }];
        let words = to_words(&spans);
        assert_eq!(words.len(), 2);
        assert_eq!(words[0].0, "hello");
        assert_eq!(words[1].0, "world");
    }

    #[test]
    fn wrap_words_basic_fits() {
        let words = vec![
            ("hello".to_string(), Style::default()),
            ("world".to_string(), Style::default()),
        ];
        let lines = wrap_words(&words, "", "", 20);
        assert_eq!(text(&lines), vec!["hello world"]);
    }

    #[test]
    fn wrap_words_wraps_at_width() {
        let words = vec![
            ("aaaa".to_string(), Style::default()),
            ("bbbb".to_string(), Style::default()),
            ("cccc".to_string(), Style::default()),
        ];
        let lines = wrap_words(&words, "", "", 10);
        assert_eq!(text(&lines), vec!["aaaa bbbb", "cccc"]);
    }

    #[test]
    fn wrap_words_with_prefix_indent() {
        let words = vec![
            ("word1".to_string(), Style::default()),
            ("word2".to_string(), Style::default()),
            ("word3".to_string(), Style::default()),
        ];
        let lines = wrap_words(&words, "• ", "  ", 10);
        assert_eq!(text(&lines), vec!["• word1", "  word2", "  word3"]);
    }

    #[test]
    fn wrap_words_breaks_long_word_mid_word() {
        let words = vec![("abcdefghijklmno".to_string(), Style::default())];
        let lines = wrap_words(&words, "", "", 5);
        assert_eq!(text(&lines), vec!["abcde", "fghij", "klmno"]);
    }

    #[test]
    fn wrap_words_wide_chars_count_two() {
        // 你好 and 世界 are each 4 columns; together + space = 9 > 5.
        let words = vec![
            ("你好".to_string(), Style::default()),
            ("世界".to_string(), Style::default()),
        ];
        let lines = wrap_words(&words, "", "", 5);
        assert_eq!(text(&lines), vec!["你好", "世界"]);
    }

    // ---------- cell padding ----------

    #[test]
    fn pad_cell_left() {
        let content = vec![("ab".to_string(), Style::default())];
        let padded = pad_cell(&content, 5, Some(Alignment::Left));
        let s: String = padded.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(s, "ab   ");
    }

    #[test]
    fn pad_cell_right() {
        let content = vec![("ab".to_string(), Style::default())];
        let padded = pad_cell(&content, 5, Some(Alignment::Right));
        let s: String = padded.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(s, "   ab");
    }

    #[test]
    fn pad_cell_center() {
        let content = vec![("ab".to_string(), Style::default())];
        let padded = pad_cell(&content, 6, Some(Alignment::Center));
        let s: String = padded.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(s, "  ab  ");
    }

    #[test]
    fn pad_cell_wide_char() {
        // 你好 is 4 columns; padded to 6 should add 2 spaces, not 4.
        let content = vec![("你好".to_string(), Style::default())];
        let padded = pad_cell(&content, 6, Some(Alignment::Left));
        let s: String = padded.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(s, "你好  ");
    }

    // ---------- layout ----------

    #[test]
    fn layout_title_and_content_rows() {
        let slides = parse("# T\n\nbody\n");
        let layout = Layout::new(80, 24, &slides[0], 99);
        assert_eq!(layout.title_rows, 2); // title + separator
        assert_eq!(layout.content_rows, 21); // 24 - 2 - 1 (footer)
    }

    #[test]
    fn layout_blank_line_between_blocks() {
        let slides = parse("first\n\nsecond\n");
        let layout = Layout::new(80, 24, &slides[0], 99);
        assert_eq!(text(&layout.lines), vec!["first", "", "second"]);
    }

    #[test]
    fn layout_visible_truncates_blocks() {
        let slides = parse("one\n\ntwo\n\nthree\n");
        let layout = Layout::new(80, 24, &slides[0], 2);
        assert_eq!(text(&layout.lines), vec!["one", "", "two"]);
    }

    #[test]
    fn layout_max_scroll_when_overflowing() {
        let md = "a\n\nb\n\nc\n\nd\n\ne\n\nf\n\ng\n\nh\n\ni\n\nj\n";
        let slides = parse(md);
        let layout = Layout::new(40, 5, &slides[0], 99);
        // content_rows = 5 - 0 - 1 = 4. 10 lines + 9 blanks = 19.
        assert_eq!(layout.content_rows, 4);
        assert_eq!(layout.lines.len(), 19);
        assert_eq!(layout.max_scroll(), 15);
    }

    #[test]
    fn layout_bullet_prefix() {
        let slides = parse("- alpha\n- beta\n");
        let layout = Layout::new(40, 10, &slides[0], 99);
        let lines = text(&layout.lines);
        assert_eq!(lines[0], "• alpha");
        assert_eq!(lines[2], "• beta");
    }

    #[test]
    fn layout_nested_bullet_indent() {
        let slides = parse("- parent\n  - child\n");
        let layout = Layout::new(40, 10, &slides[0], 99);
        let lines = text(&layout.lines);
        assert_eq!(lines[0], "• parent");
        assert_eq!(lines[2], "  • child");
    }

    #[test]
    fn layout_ordered_bullet_prefix() {
        let slides = parse("1. one\n2. two\n");
        let layout = Layout::new(40, 10, &slides[0], 99);
        let lines = text(&layout.lines);
        assert_eq!(lines[0], "1. one");
        assert_eq!(lines[2], "2. two");
    }

    #[test]
    fn layout_blockquote_prefix() {
        let slides = parse("> quoted\n");
        let layout = Layout::new(40, 10, &slides[0], 99);
        assert_eq!(text(&layout.lines)[0], "│ quoted");
    }

    #[test]
    fn layout_table_has_borders() {
        let slides = parse("| a | b |\n|---|---|\n| 1 | 2 |\n");
        let layout = Layout::new(40, 20, &slides[0], 99);
        let lines = text(&layout.lines);
        assert!(lines[0].starts_with('┌'));
        assert!(lines[0].ends_with('┐'));
        let head = lines
            .iter()
            .find(|l| l.contains('a') && l.contains('b'))
            .expect("head row present");
        assert!(head.starts_with("│ "));
    }

    // ---------- draw ----------

    #[test]
    fn draw_smoke() {
        let slides = parse("# T\n\nhello\n");
        let layout = Layout::new(20, 6, &slides[0], 99);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
        let area = Rect::new(0, 0, 20, 6);
        draw(&mut buf, area, &slides[0], &[Some(1.0)], 1, 0, &layout, 0, 1);
        let rendered = buf_to_string(&buf, area);
        assert!(rendered.contains('T'));
        assert!(rendered.contains("hello"));
        assert!(rendered.contains("1/1"));
    }

    #[test]
    fn draw_footer_right_aligned() {
        let slides = parse("# T\n\nbody\n");
        let layout = Layout::new(20, 6, &slides[0], 99);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
        let area = Rect::new(0, 0, 20, 6);
        draw(&mut buf, area, &slides[0], &[Some(1.0)], 1, 0, &layout, 0, 5);
        let footer: String = (0..20).map(|x| buf[(x, 5)].symbol()).collect();
        assert!(footer.contains("poorerpoint"));
        assert!(footer.ends_with(' '));
    }

    #[test]
    fn draw_clips_content_to_area() {
        // Narrow terminal with long text; ensure it doesn't panic.
        let slides = parse("a very long line of text that should be clipped\n");
        let layout = Layout::new(10, 4, &slides[0], 99);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 4));
        let area = Rect::new(0, 0, 10, 4);
        draw(&mut buf, area, &slides[0], &[], 1, 0, &layout, 0, 1);
        let _ = buf_to_string(&buf, area);
    }

    #[test]
    fn draw_wide_char_does_not_panic() {
        let slides = parse("你好世界\n");
        let layout = Layout::new(5, 4, &slides[0], 99);
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 4));
        let area = Rect::new(0, 0, 5, 4);
        draw(&mut buf, area, &slides[0], &[], 1, 0, &layout, 0, 1);
        // Wide chars must not straddle the right edge (x=4 is odd for a
        // width-2 glyph). Reaching here without panic is the assertion.
    }
}