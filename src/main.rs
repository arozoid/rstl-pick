mod app;
mod config;
mod events;
mod ui;

use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use app::App;
use config::MenuEntry;
use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, terminal,
};
use events::AppEvent;
use ratatui::{backend::CrosstermBackend, Terminal};

type T = Terminal<CrosstermBackend<io::Stdout>>;

fn main() -> io::Result<()> {
    ignore_signals();
    let config_arg = match config::config_arg() {
        Ok(arg) => arg,
        Err(msg) => {
            eprintln!("rstl-pick: {msg}");
            std::process::exit(2);
        }
    };
    let path = config::config_path(config_arg);
    let entries = if path.exists() {
        let source = std::fs::read_to_string(&path).map_err(|e| {
            io::Error::new(e.kind(), format!("cannot read {}: {e}", path.display()))
        })?;
        match config::parse(&source) {
            Ok(entries) => entries,
            Err(msg) => {
                eprintln!("rstl-pick: {msg}");
                std::process::exit(2);
            }
        }
    } else {
        match config::ensure_default(&path) {
            Ok(entries) => {
                eprintln!("rstl-pick: wrote default config to {}", path.display());
                entries
            }
            Err(msg) => {
                eprintln!("rstl-pick: {msg}");
                std::process::exit(2);
            }
        }
    };

    let mut stdout = io::stdout();
    enter_ui(&mut stdout)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run(&mut terminal, entries);

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
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        EnableMouseCapture,
        // A thin bar cursor in the search box (ratatui shows it whenever the
        // frame sets a cursor position).
        SetCursorStyle::BlinkingBar
    )?;
    Ok(())
}

/// Leave the alternate screen and raw mode so a real program can take over.
fn leave_ui(stdout: &mut io::Stdout) -> io::Result<()> {
    execute!(
        stdout,
        terminal::LeaveAlternateScreen,
        DisableMouseCapture,
        // Restore the terminal's normal cursor shape after the TUI.
        SetCursorStyle::DefaultUserShape,
        Show
    )?;
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

/// Run a picker-style command outside the TUI, capture its stdout, and return
/// the result (None when nothing was produced).
///
/// Built for `VAR=$(app)`: the program renders its UI through the controlling
/// terminal and prints only the picked value to stdout, so piping stdout here
/// keeps it fully interactive while we grab the result.
fn run_capture(terminal: &mut T, command: &str) -> io::Result<Option<String>> {
    leave_ui(&mut io::stdout())?;
    let mut child = unsafe {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .pre_exec(reset_signals_in_child)
            .spawn()
    }?;

    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        // Blocks until the program closes stdout (i.e. exits).
        let _ = stdout.read_to_string(&mut out);
    }
    let _ = child.wait();

    enter_ui(&mut io::stdout())?;
    terminal.clear()?;

    let captured = out.trim();
    if captured.is_empty() {
        Ok(None)
    } else {
        Ok(Some(captured.to_string()))
    }
}

/// Put `text` on the clipboard via the first available tool. Returns Ok(false)
/// when no clipboard tool is installed.
fn set_clipboard(text: &str) -> io::Result<bool> {
    const TOOLS: [(&str, &[&str]); 3] = [
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
    ];
    for (tool, args) in TOOLS {
        let mut child = match Command::new(tool)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => continue, // tool not installed
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        match child.wait() {
            Ok(status) if status.success() => return Ok(true),
            _ => continue,
        }
    }
    Ok(false)
}

fn run(terminal: &mut T, entries: Vec<MenuEntry>) -> io::Result<()> {
    let mut app = App::new(entries);
    let shortcuts: Vec<char> = app.entries.iter().map(|e| e.key).collect();

    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        match events::next_event(&shortcuts)? {
            AppEvent::Quit => app.quit = true,
            AppEvent::NavigatePrev => {
                app.clear_notice();
                app.previous()
            }
            AppEvent::NavigateNext => {
                app.clear_notice();
                app.next()
            }
            AppEvent::Type(c) => {
                app.clear_notice();
                app.type_char(c)
            }
            AppEvent::Backspace => {
                app.clear_notice();
                app.backspace()
            }
            AppEvent::Select => {
                app.clear_notice();
                handle_selection(terminal, &mut app)?
            }
            AppEvent::Shortcut(key) => {
                app.clear_notice();
                if let Some(entry) = app.entry_by_key(key) {
                    handle_item(terminal, &mut app, &entry.command, entry.clip_stdout)?
                }
            }
            AppEvent::Redraw => {}
        }
    }

    Ok(())
}

/// Dispatch the chosen entry to its underlying program.
fn handle_selection(terminal: &mut T, app: &mut App) -> io::Result<()> {
    if let Some(entry) = app.current_selection() {
        handle_item(terminal, app, &entry.command, entry.clip_stdout)?;
    }
    Ok(())
}

/// Run a specific entry (from the highlighted row or a shortcut key).
fn handle_item(terminal: &mut T, app: &mut App, command: &str, clip: bool) -> io::Result<()> {
    if clip {
        // Closed without picking -> nothing to copy.
        if let Some(captured) = run_capture(terminal, command)? {
            match set_clipboard(&captured) {
                Ok(true) => app.flash("selection copied to clipboard"),
                _ => app.flash("no clipboard tool found (wl-copy/xclip/xsel)"),
            }
        }
    } else {
        run_external(terminal, command)?;
    }
    Ok(())
}
