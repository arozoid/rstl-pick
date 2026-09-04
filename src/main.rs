mod app;
mod events;
mod ui;

use std::{io, process::Command};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, terminal,
};
use events::AppEvent;
use ratatui::{backend::CrosstermBackend, Terminal};

type T = Terminal<CrosstermBackend<io::Stdout>>;

fn main() -> io::Result<()> {
    ignore_signals();
    let mut stdout = io::stdout();
    enter_ui(&mut stdout)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run(&mut terminal);

    // Always restore the terminal, even on error.
    leave_ui(&mut io::stdout())?;
    result
}

/// Discard common terminal signals so stray keystrokes can't kill the TUI.
///
/// Raw mode already disables ISIG (so Ctrl+C / Ctrl+\ / Ctrl+Z arrive as key
/// events rather than signals, and are filtered in the event loop). Setting the
/// signals to SIG_IGN here is defence in depth for any moment raw mode is not
/// active.
#[cfg(unix)]
fn ignore_signals() {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
    }
}

/// Revert to normal signal handling inside a spawned child (only), so a child
/// app gets real Ctrl+C behaviour. The parent keeps the signals ignored (set in
/// `ignore_signals`) so it can never be killed by them and the TUI always
/// returns.
#[cfg(unix)]
fn reset_signals_in_child() -> io::Result<()> {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_DFL);
        libc::signal(libc::SIGQUIT, libc::SIG_DFL);
        libc::signal(libc::SIGTSTP, libc::SIG_DFL);
    }
    Ok(())
}

#[cfg(not(unix))]
fn ignore_signals() {}

#[cfg(not(unix))]
fn reset_signals_in_child() -> io::Result<()> {
    Ok(())
}

/// Enter the alternate screen and raw mode.
fn enter_ui(stdout: &mut io::Stdout) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

/// Leave the alternate screen and raw mode so a real program can take over.
fn leave_ui(stdout: &mut io::Stdout) -> io::Result<()> {
    execute!(stdout, terminal::LeaveAlternateScreen, DisableMouseCapture)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

/// Run a foreground command outside the TUI, then return to it.
///
/// After the program exits, the terminal is cleared and `Terminal::clear`
/// resets ratatui's internal back buffer so the next `draw()` repaints the
/// entire frame, guaranteeing the TUI stays fully rendered.
fn run_external(terminal: &mut T, command: &str) -> io::Result<()> {
    leave_ui(&mut io::stdout())?;
    // The parent keeps the signals ignored (set in `ignore_signals`), so a
    // Ctrl+C inside the child only kills the child — never this process — and
    // the TUI reliably comes back. Normal signal handling is restored inside
    // the child only, so Ctrl+C actually works in the running program.
    let _ = unsafe {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .pre_exec(reset_signals_in_child)
    }
    .status();
    enter_ui(&mut io::stdout())?;
    terminal.clear()?;
    Ok(())
}

fn run(terminal: &mut T) -> io::Result<()> {
    let mut app = App::new();

    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        match events::next_event()? {
            AppEvent::Quit => app.quit = true,
            AppEvent::NavigatePrev => app.previous(),
            AppEvent::NavigateNext => app.next(),
            AppEvent::Type(c) => app.type_char(c),
            AppEvent::Backspace => app.backspace(),
            AppEvent::Select => handle_selection(terminal, &app)?,
            AppEvent::Redraw => {}
        }
    }

    Ok(())
}

/// Dispatch the chosen entry to its underlying program.
fn handle_selection(terminal: &mut T, app: &App) -> io::Result<()> {
    if let Some(item) = app.current_selection() {
        run_external(terminal, item.run())?;
    }
    Ok(())
}