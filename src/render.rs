use pulldown_cmark::Alignment;
use ratatui::{
    buffer::{Buffer, Cell},
    layout::Rect,
    style::{Color, Modifier, Style as RStyle},
};

use crate::parse::{Block, Slide, Span, Style};

// ---------- style mapping ----------

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

// ---------- layout ----------

pub struct Layout {
    pub title_rows: usize,
    pub content_rows: usize,
    /// Fully laid-out content lines for the *visible* blocks, pre-wrapped.
    pub lines: Vec<Vec<(String, Style)>>,
}

impl Layout {
    pub fn new(width: u16, height: u16, slide: &Slide, visible: usize) -> Self {
        let w = width as usize;
        let h = height as usize;
        let title_rows = if slide.stack.is_empty() {
            0
        } else {
            slide.stack.len() + 1 // + separator
        };
        let content_rows = h.saturating_sub(title_rows + 1); // -1 footer

        let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
        for (i, b) in slide.blocks.iter().take(visible).enumerate() {
            if i > 0 {
                lines.push(Vec::new()); // blank between blocks
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
        Block::Para(spans) => wrap_spans(spans, "", "", width),
        Block::Bullet { depth, spans } => {
            let p = format!("{}• ", "  ".repeat(*depth));
            let c = " ".repeat(p.chars().count());
            wrap_spans(spans, &p, &c, width)
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

// ---------- whitespace-aware wrapping ----------

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
    let mut cur_w = prefix.chars().count();

    for (wd, st) in words {
        let ww = wd.chars().count();
        if !cur.is_empty() && cur_w + 1 + ww > width {
            let mut line = vec![(prefix.clone(), Style::default())];
            line.append(&mut cur);
            out.push(line);
            prefix = cont.to_string();
            cur_w = prefix.chars().count();
        }
        if !cur.is_empty() {
            cur.push((" ".into(), *st));
            cur_w += 1;
        }
        cur.push((wd.clone(), *st));
        cur_w += ww;
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

// ---------- tables ----------

fn measure_row(row: &[Vec<Span>], nat: &mut [usize], floor: &mut [usize]) {
    for (c, cell) in row.iter().enumerate() {
        if c >= nat.len() {
            break;
        }
        let words = to_words(cell);
        if words.is_empty() {
            continue;
        }
        // Natural: full text width including the spaces between words.
        let full: usize = words.iter().map(|(w, _)| w.chars().count()).sum::<usize>()
            + words.len().saturating_sub(1);
        if full > nat[c] {
            nat[c] = full;
        }
        // Floor: longest single word — a column narrower than this can't wrap.
        let longest = words.iter().map(|(w, _)| w.chars().count()).max().unwrap_or(0);
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

    // Pass 1: per-column natural (full text) and floor (longest word).
    let mut nat = vec![0usize; ncols];
    let mut floor = vec![1usize; ncols];
    measure_row(head, &mut nat, &mut floor);
    for r in rows {
        measure_row(r, &mut nat, &mut floor);
    }

    // Pass 2: fit to terminal, keeping each column ≥ its floor.
    let overhead = 3 * ncols + 1;
    let avail = width.saturating_sub(overhead);

    let mut widths: Vec<usize> = nat.clone();
    if widths.iter().sum::<usize>() > avail {
        let floor_sum: usize = floor.iter().sum();
        if floor_sum >= avail {
            // Even floors don't fit. Hand out floors and let the table clip.
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
                // Proportional share, largest-remainder method for the leftovers.
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

    // Pass 3: wrap each cell to its final column width.
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
    let len: usize = content.iter().map(|(t, _)| t.chars().count()).sum();
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

// ---------- drawing ----------

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
        if cx >= x1 {
            break;
        }
        if cx >= x0 {
            if let Some(c) = buf.cell_mut((cx as u16, y)) {
                c.set_char(ch).set_style(style);
            }
        }
        cx += 1;
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

    // --- separator ---
    if !slide.stack.is_empty() {
        let y = area.y + slide.stack.len() as u16;
        let sep = RStyle::default().add_modifier(Modifier::DIM);
        for x in area.x..area.x + area.width {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_char('─').set_style(sep);
            }
        }
    }

    // --- content ---
    let _ = visible_blocks; // already baked into layout.lines
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
        let mut x = area.x;
        let xmax = area.x + area.width;
        'line: for (text, st) in line {
            let s = rstyle(*st);
            for ch in text.chars() {
                if x >= xmax {
                    break 'line;
                }
                if let Some(c) = buf.cell_mut((x, y)) {
                    c.set_char(ch).set_style(s);
                }
                x += 1;
            }
        }
    }

    // --- footer ---
    if area.height >= 1 {
        let s = format!(" poorerpoint  {}/{} ", idx + 1, total);
        let fx = (area.x + area.width).saturating_sub(s.chars().count() as u16);
        let st = RStyle::default().add_modifier(Modifier::DIM);
        draw_str(buf, area, fx as i32, area.y + area.height - 1, &s, st);
    }
}