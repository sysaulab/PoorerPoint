use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleLine {
    pub level: usize,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug)]
pub enum Block {
    Para {
        quote: usize,
        spans: Vec<Span>,
    },
    Bullet {
        quote: usize,
        depth: usize,
        marker: String,
        spans: Vec<Span>,
    },
    Code(Vec<String>),
    Rule,
    Table {
        aligns: Vec<Alignment>,
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
}

#[derive(Clone, Debug)]
pub struct Slide {
    pub stack: Vec<TitleLine>,
    pub anim: Vec<usize>,
    pub blocks: Vec<Block>,
}

fn hlevel(l: HeadingLevel) -> usize {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// One open list item. `has_children` is set when a nested `Start(Item)` fires
// while this item is on the stack, so empty parents still get an anchor bullet.
type OpenItem = (Vec<Span>, usize, String, Option<bool>, bool);

fn push_span(
    items: &mut Vec<OpenItem>,
    para: &mut Vec<Span>,
    cell: &mut Option<Vec<Span>>,
    span: Span,
) {
    if let Some(c) = cell.as_mut() {
        c.push(span);
    } else if let Some((top, _, _, _, _)) = items.last_mut() {
        top.push(span);
    } else {
        para.push(span);
    }
}

pub fn parse(md: &str) -> Vec<Slide> {
    let parser = Parser::new_ext(md, Options::ENABLE_TABLES);

    let mut stack: Vec<TitleLine> = Vec::new();
    let mut slides: Vec<Slide> = Vec::new();

    let mut cur_stack: Vec<TitleLine> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();

    let mut heading: Option<(usize, String)> = None;
    let mut para: Vec<Span> = Vec::new();
    let mut items: Vec<OpenItem> = Vec::new();
    // One entry per open `Tag::List`: Some(n) = ordered, next is n; None = bullet.
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut quote_depth: usize = 0;

    let mut bold = 0usize;
    let mut italic = 0usize;

    let mut in_code = false;
    let mut code_buf = String::new();

    // table state
    let mut table_aligns: Vec<Alignment> = Vec::new();
    let mut table_head: Vec<Vec<Span>> = Vec::new();
    let mut table_rows: Vec<Vec<Vec<Span>>> = Vec::new();
    let mut table_row: Vec<Vec<Span>> = Vec::new();
    let mut table_cell: Option<Vec<Span>> = None;

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((hlevel(level), String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                let (lvl, text) = heading.take().expect("heading end without start");
                stack.retain(|t| t.level < lvl);
                stack.push(TitleLine { level: lvl, text });

                if !blocks.is_empty() {
                    slides.push(Slide {
                        stack: cur_stack.clone(),
                        anim: Vec::new(),
                        blocks: std::mem::take(&mut blocks),
                    });
                }
                cur_stack = stack.clone();
            }

            // ---- blockquotes ----
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                quote_depth = quote_depth.saturating_sub(1);
            }

            // ---- tables ----
            Event::Start(Tag::Table(aligns)) => {
                table_aligns = aligns;
                table_head.clear();
                table_rows.clear();
                table_row.clear();
            }
            Event::End(TagEnd::Table) => {
                blocks.push(Block::Table {
                    aligns: std::mem::take(&mut table_aligns),
                    head: std::mem::take(&mut table_head),
                    rows: std::mem::take(&mut table_rows),
                });
            }
            Event::Start(Tag::TableHead) => table_row.clear(),
            Event::End(TagEnd::TableHead) => {
                table_head = std::mem::take(&mut table_row);
            }
            Event::Start(Tag::TableRow) => table_row.clear(),
            Event::End(TagEnd::TableRow) => {
                table_rows.push(std::mem::take(&mut table_row));
            }
            Event::Start(Tag::TableCell) => table_cell = Some(Vec::new()),
            Event::End(TagEnd::TableCell) => {
                let c = table_cell.take().unwrap_or_default();
                table_row.push(c);
            }

            // ---- lists ----
            Event::Start(Tag::List(start)) => list_stack.push(start),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let marker = match list_stack.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{}. ", *n);
                        *n += 1;
                        m
                    }
                    _ => "• ".to_string(),
                };
                if let Some(parent) = items.last_mut() {
                    parent.4 = true;
                }
                items.push((Vec::new(), blocks.len(), marker, None, false));
            }
            Event::End(TagEnd::Item) => {
                if let Some((mut spans, at, marker, task, has_children)) = items.pop() {
                    if let Some(checked) = task {
                        let t = if checked { "[x] " } else { "[ ] " };
                        spans.insert(
                            0,
                            Span {
                                text: t.to_string(),
                                style: Style { code: true, ..Default::default() },
                            },
                        );
                    }
                    if !spans.is_empty() || has_children {
                        let depth = items.len();
                        // Insert (not push) at the frozen start position so
                        // ancestors land before their descendants; siblings
                        // froze at higher `at` and are unaffected.
                        blocks.insert(
                            at,
                            Block::Bullet { quote: quote_depth, depth, marker, spans },
                        );
                    }
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = items.last_mut() {
                    item.3 = Some(checked);
                }
            }

            Event::End(TagEnd::Paragraph) => {
                if table_cell.is_none() && items.is_empty() && !para.is_empty() {
                    blocks.push(Block::Para {
                        quote: quote_depth,
                        spans: std::mem::take(&mut para),
                    });
                }
            }

            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                let mut lines: Vec<String> =
                    code_buf.split('\n').map(|s| s.to_string()).collect();
                while lines.last().is_some_and(|l| l.is_empty()) {
                    lines.pop();
                }
                if !lines.is_empty() {
                    blocks.push(Block::Code(lines));
                }
                code_buf.clear();
            }

            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),

            Event::Text(t) => {
                if in_code {
                    code_buf.push_str(&t);
                } else if let Some((_, h)) = heading.as_mut() {
                    h.push_str(&t);
                } else {
                    push_span(
                        &mut items,
                        &mut para,
                        &mut table_cell,
                        Span {
                            text: t.to_string(),
                            style: Style { bold: bold > 0, italic: italic > 0, code: false },
                        },
                    );
                }
            }
            Event::Code(t) => {
                if let Some((_, h)) = heading.as_mut() {
                    h.push_str(&t);
                } else {
                    push_span(
                        &mut items,
                        &mut para,
                        &mut table_cell,
                        Span {
                            text: t.to_string(),
                            style: Style { bold: bold > 0, italic: italic > 0, code: true },
                        },
                    );
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                push_span(
                    &mut items,
                    &mut para,
                    &mut table_cell,
                    Span { text: " ".into(), style: Style::default() },
                );
            }
            Event::Rule => blocks.push(Block::Rule),
            _ => {}
        }
    }

    if !blocks.is_empty() {
        slides.push(Slide { stack: cur_stack, anim: Vec::new(), blocks });
    }

    for i in 0..slides.len() {
        let prev: &[TitleLine] = if i == 0 { &[] } else { &slides[i - 1].stack };
        let mut common = 0;
        while common < prev.len()
            && common < slides[i].stack.len()
            && prev[common] == slides[i].stack[common]
        {
            common += 1;
        }
        slides[i].anim = (common..slides[i].stack.len()).collect();
    }

    slides
}






///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////






#[cfg(test)]
mod tests {
    use super::*;

    fn only(slides: &[Slide]) -> &Slide {
        assert_eq!(slides.len(), 1, "expected exactly one slide");
        &slides[0]
    }

    fn bullet_list(slide: &Slide) -> Vec<(usize, String, String)> {
        slide
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Bullet { depth, marker, spans, .. } => {
                    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                    Some((*depth, marker.clone(), text))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn empty_input_has_no_slides() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn paragraph_only() {
        let slides = parse("hello world\n");
        let s = only(&slides);
        assert!(s.stack.is_empty());
        assert_eq!(s.blocks.len(), 1);
        match &s.blocks[0] {
            Block::Para { quote, spans } => {
                assert_eq!(*quote, 0);
                assert_eq!(spans[0].text, "hello world");
            }
            other => panic!("expected Para, got {other:?}"),
        }
    }

    #[test]
    fn heading_becomes_title() {
        let slides = parse("# Hello\n\nbody\n");
        let s = only(&slides);
        assert_eq!(s.stack.len(), 1);
        assert_eq!(s.stack[0].level, 1);
        assert_eq!(s.stack[0].text, "Hello");
    }

    #[test]
    fn sibling_headings_split_slides() {
        let slides = parse("# A\n\ntext a\n\n# B\n\ntext b\n");
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].stack[0].text, "A");
        assert_eq!(slides[1].stack[0].text, "B");
    }

    #[test]
    fn nested_headings_stack() {
        let slides = parse("# H1\n\n## H2\n\n### H3\n\nbody\n");
        let s = only(&slides);
        assert_eq!(s.stack.len(), 3);
        assert_eq!(s.stack[0].text, "H1");
        assert_eq!(s.stack[1].text, "H2");
        assert_eq!(s.stack[2].text, "H3");
    }

    #[test]
    fn heading_anim_only_for_new_titles() {
        // Slide 2 shares H1 with slide 1, so only H2 should animate.
        let slides = parse("# Root\n\n## A\n\nbody a\n\n## B\n\nbody b\n");
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].anim, vec![0, 1]); // both titles animate
        assert_eq!(slides[1].anim, vec![1]); // only B, Root is carried over
    }

    #[test]
    fn flat_bullets() {
        let slides = parse("- a\n- b\n- c\n");
        let s = only(&slides);
        let items = bullet_list(s);
        assert_eq!(
            items,
            vec![
                (0, "• ".into(), "a".into()),
                (0, "• ".into(), "b".into()),
                (0, "• ".into(), "c".into()),
            ]
        );
    }

    #[test]
    fn nested_bullet_order_parent_before_child() {
        // Regression: End(Item) fires child-first. Insertion at the frozen
        // `blocks.len()` must still land the parent ahead of its children.
        let slides = parse("- parent\n  - child\n");
        let s = only(&slides);
        let order: Vec<(usize, String)> = bullet_list(s)
            .into_iter()
            .map(|(d, _, t)| (d, t))
            .collect();
        assert_eq!(
            order,
            vec![(0, "parent".into()), (1, "child".into())]
        );
    }

    #[test]
    fn four_level_nesting() {
        let slides = parse("- L0\n  - L1\n    - L2\n      - L3\n");
        let s = only(&slides);
        let depths: Vec<usize> = bullet_list(s).into_iter().map(|(d, _, _)| d).collect();
        assert_eq!(depths, vec![0, 1, 2, 3]);
    }

    #[test]
    fn orphan_parent_still_gets_a_bullet() {
        // Empty parent with a nested child must still anchor a bullet.
        let slides = parse("-\n  - child\n");
        let s = only(&slides);
        let items = bullet_list(s);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, 0);
        assert_eq!(items[0].2, "");
        assert_eq!(items[1].0, 1);
        assert_eq!(items[1].2, "child");
    }

    #[test]
    fn ordered_list_markers() {
        let slides = parse("1. one\n2. two\n3. three\n");
        let s = only(&slides);
        let markers: Vec<String> = bullet_list(s).into_iter().map(|(_, m, _)| m).collect();
        assert_eq!(markers, vec!["1. ", "2. ", "3. "]);
    }

    #[test]
    fn ordered_list_starting_at_non_one() {
        let slides = parse("5. five\n6. six\n");
        let s = only(&slides);
        let markers: Vec<String> = bullet_list(s).into_iter().map(|(_, m, _)| m).collect();
        assert_eq!(markers, vec!["5. ", "6. "]);
    }

    #[test]
    fn mixed_ordered_and_bullet() {
        let slides = parse("1. one\n   - a\n   - b\n2. two\n");
        let s = only(&slides);
        let items: Vec<(usize, String)> = bullet_list(s)
            .into_iter()
            .map(|(d, m, _)| (d, m))
            .collect();
        assert_eq!(
            items,
            vec![
                (0, "1. ".into()),
                (1, "• ".into()),
                (1, "• ".into()),
                (0, "2. ".into()),
            ]
        );
    }

    #[test]
    fn task_list_markers() {
        let slides = parse("- [x] done\n- [ ] todo\n");
        let s = only(&slides);
        let prefixes: Vec<String> = bullet_list(s).into_iter().map(|(_, _, t)| t).collect();
        assert_eq!(prefixes, vec!["[x] done", "[ ] todo"]);
    }

    #[test]
    fn blockquote_depth() {
        let slides = parse("> one\n> > two\n> > > three\n");
        let s = only(&slides);
        let depths: Vec<usize> = s
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Para { quote, .. } => Some(*quote),
                _ => None,
            })
            .collect();
        assert_eq!(depths, vec![1, 2, 3]);
    }

    #[test]
    fn quote_containing_list() {
        let slides = parse("> - a\n> - b\n");
        let s = only(&slides);
        let items: Vec<(usize, usize, String)> = s
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Bullet { quote, depth, spans, .. } => {
                    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                    Some((*quote, *depth, text))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            items,
            vec![(1, 0, "a".into()), (1, 0, "b".into())]
        );
    }

    #[test]
    fn code_block_preserved() {
        let slides = parse("```\nline one\n  line two\n```\n");
        let s = only(&slides);
        match &s.blocks[0] {
            Block::Code(lines) => {
                assert_eq!(lines, &vec!["line one".to_string(), "  line two".to_string()]);
            }
            other => panic!("expected Code, got {other:?}"),
        }
    }

    #[test]
    fn horizontal_rule() {
        let slides = parse("a\n\n---\n\nb\n");
        let s = only(&slides);
        assert!(s.blocks.iter().any(|b| matches!(b, Block::Rule)));
    }

    #[test]
    fn table_head_and_rows() {
        let slides = parse("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n");
        let s = only(&slides);
        match &s.blocks[0] {
            Block::Table { head, rows, aligns } => {
                assert_eq!(head.len(), 2);
                assert_eq!(head[0][0].text, "a");
                assert_eq!(head[1][0].text, "b");
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0][0][0].text, "1");
                assert_eq!(rows[1][1][0].text, "4");
                assert_eq!(aligns.len(), 2);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn inline_styles() {
        let slides = parse("**bold** *it* `code`\n");
        let s = only(&slides);
        match &s.blocks[0] {
            Block::Para { spans, .. } => {
                let bold = spans.iter().find(|s| s.text == "bold").unwrap();
                assert!(bold.style.bold);
                let italic = spans.iter().find(|s| s.text == "it").unwrap();
                assert!(italic.style.italic);
                let code = spans.iter().find(|s| s.text == "code").unwrap();
                assert!(code.style.code);
            }
            other => panic!("expected Para, got {other:?}"),
        }
    }
}