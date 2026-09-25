mod app;
mod parse;
mod render;

use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> io::Result<()> {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: poorerpoint <deck.md>");
            std::process::exit(2);
        }
    };
    let src = std::fs::read_to_string(&path)?;
    let slides = parse::parse(&src);
    if slides.is_empty() {
        eprintln!("poorerpoint: no slides in {path}");
        std::process::exit(1);
    }

    // restore terminal on panic
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        hook(info);
    }));

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let res = app::run(&mut terminal, slides);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}