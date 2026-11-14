use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::Backend, Terminal};

use crate::parse::Slide;
use crate::render;

const FRAME_MS: u64 = 16;

#[derive(Debug, Clone, Copy)]
pub struct Timings {
    pub title_ms: u64,
    pub content_stagger_ms: u64,
    pub tail_ms: u64,
}

impl Default for Timings {
    fn default() -> Self {
        Self {
            title_ms: 333,
            content_stagger_ms: 55,
            tail_ms: 50,
        }
    }
}

pub struct AnimFrame {
    pub title_progress: Vec<Option<f32>>,
    pub visible_blocks: usize,
    pub animating: bool,
}

pub struct App {
    slides: Vec<Slide>,
    idx: usize,
    start: Instant,
    scroll: usize,
    auto_scroll: bool,
    quit: bool,
    timings: Timings,
}

impl App {
    pub fn new(slides: Vec<Slide>, timings: Timings) -> Self {
        Self {
            slides,
            idx: 0,
            start: Instant::now(),
            scroll: 0,
            auto_scroll: true,
            quit: false,
            timings,
        }
    }

    pub fn frame(&self) -> AnimFrame {
        let slide = &self.slides[self.idx];
        let elapsed = self.start.elapsed().as_millis() as u64;
        let t = self.timings;

        // --- title block: title_ms total, overlapping slide-ins ---
        let n = slide.anim.len();
        let mut title_progress = vec![None; slide.stack.len()];
        if n > 0 {
            let dur = (t.title_ms.saturating_mul(2) / 3).max(1);
            let stagger = if n > 1 {
                t.title_ms.saturating_sub(dur) / (n as u64 - 1)
            } else {
                0
            };
            for (k, &i) in slide.anim.iter().enumerate() {
                let s = (k as u64).saturating_mul(stagger);
                let e = s.saturating_add(dur);
                let p = if elapsed >= e {
                    1.0
                } else if elapsed <= s {
                    0.0
                } else {
                    (elapsed - s) as f32 / (e - s) as f32
                };
                title_progress[i] = Some(p.clamp(0.0, 1.0));
            }
        }

        // --- content: one block every content_stagger_ms after the titles ---
        let mut visible_blocks = 0usize;
        for i in 0..slide.blocks.len() {
            let start = t
                .title_ms
                .saturating_add((i as u64).saturating_mul(t.content_stagger_ms));
            if elapsed >= start {
                visible_blocks += 1;
            }
        }

        let total_ms = t
            .title_ms
            .saturating_add((slide.blocks.len() as u64).saturating_mul(t.content_stagger_ms))
            .saturating_add(t.tail_ms);

        AnimFrame {
            title_progress,
            visible_blocks,
            animating: elapsed < total_ms,
        }
    }

    fn go(&mut self, idx: usize) {
        self.idx = idx;
        self.start = Instant::now();
        self.scroll = 0;
        self.auto_scroll = true;
    }

    fn handle(&mut self, k: KeyEvent) {
        let animating = self.frame().animating;
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,

            KeyCode::Char(' ') => {
                if !animating && self.idx + 1 < self.slides.len() {
                    self.go(self.idx + 1);
                }
            }

            KeyCode::Right | KeyCode::Char('l') => {
                if self.idx + 1 < self.slides.len() {
                    self.go(self.idx + 1);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.idx > 0 {
                    self.go(self.idx - 1);
                }
            }

            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
                self.auto_scroll = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                self.auto_scroll = false;
            }
            _ => {}
        }
    }
}

pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    slides: Vec<Slide>,
    timings: Timings,
) -> std::io::Result<()> {
    let mut app = App::new(slides, timings);

    loop {
        let sz = terminal.size()?;
        let f = app.frame();

        let layout = render::Layout::new(sz.width, sz.height, &app.slides[app.idx], f.visible_blocks);
        let max_scroll = layout.max_scroll();

        if f.animating && app.auto_scroll {
            app.scroll = max_scroll;
        } else {
            app.scroll = app.scroll.min(max_scroll);
        }
        let scroll = app.scroll;
        let idx = app.idx;
        let total = app.slides.len();

        terminal.draw(|tf| {
            let area = tf.area();
            render::draw(
                tf.buffer_mut(),
                area,
                &app.slides[idx],
                &f.title_progress,
                f.visible_blocks,
                scroll,
                &layout,
                idx,
                total,
            );
        })?;

        if app.quit {
            break;
        }

        let timeout = if f.animating {
            Duration::from_millis(FRAME_MS)
        } else {
            Duration::from_millis(120)
        };

        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    app.handle(k);
                }
            }
        }
    }
    Ok(())
}