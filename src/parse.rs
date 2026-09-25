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
    Para(Vec<Span>),
    Bullet { depth: usize, spans: Vec<Span> },
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

fn push_span(
    items: &mut Vec<Vec<Span>>,
    para: &mut Vec<Span>,
    cell: &mut Option<Vec<Span>>,
    span: Span,
) {
    if let Some(c) = cell.as_mut() {
        c.push(span);
    } else if let Some(top) = items.last_mut() {
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
    let mut items: Vec<Vec<Span>> = Vec::new();

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

            Event::Start(Tag::Item) => items.push(Vec::new()),
            Event::End(TagEnd::Item) => {
                if let Some(spans) = items.pop() {
                    if !spans.is_empty() {
                        let depth = items.len();
                        blocks.push(Block::Bullet { depth, spans });
                    }
                }
            }

            Event::End(TagEnd::Paragraph) => {
                if table_cell.is_none() && items.is_empty() && !para.is_empty() {
                    blocks.push(Block::Para(std::mem::take(&mut para)));
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