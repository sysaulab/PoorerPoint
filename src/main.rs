mod app;
mod parse;
mod render;

use std::io::{self, stdout};
use std::path::PathBuf;

use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

#[derive(Parser, Debug)]
#[command(name = "poorerpoint", version, about = "A terminal presentation engine. Markdown in, ASCII out.")]
struct Cli {
    /// Path to the markdown deck
    deck: PathBuf,

    /// Title animation duration in milliseconds (0 = instant)
    #[arg(short = 't', long, value_name = "MS", default_value_t = 500)]
    title_animation: u64,

    /// Body block stagger in milliseconds (0 = all blocks at once)
    #[arg(short = 'b', long, value_name = "MS", default_value_t = 250)]
    body_animation: u64,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let src = std::fs::read_to_string(&cli.deck)?;
    let slides = parse::parse(&src);
    if slides.is_empty() {
        eprintln!("poorerpoint: no slides in {}", cli.deck.display());
        std::process::exit(1);
    }

    let timings = app::Timings {
        title_ms: cli.title_animation,
        content_stagger_ms: cli.body_animation,
        ..Default::default()
    };

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

    let res = app::run(&mut terminal, slides, timings);

    // Ignore cleanup errors individually: a failure in `disable_raw_mode`
    // must not skip `LeaveAlternateScreen` (which would leave the terminal
    // stuck in the alternate screen with the cursor hidden).
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    res
}