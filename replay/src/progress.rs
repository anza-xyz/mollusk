//! Pretty progress output for the (slow) ledger-loading path.
//!
//! Building a context loads a snapshot and replays the ledger, which can take
//! minutes with no feedback. These helpers narrate each phase on stderr, timing
//! it, so the final report stays alone on stdout. Styling is applied only when
//! stderr is a terminal and `NO_COLOR` is unset.

use std::{
    io::{IsTerminal, Write},
    sync::OnceLock,
    time::{Duration, Instant},
};

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
    })
}

fn paint(code: &str, text: &str) -> String {
    if color_enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

fn dim(text: &str) -> String {
    paint("2", text)
}

fn bold(text: &str) -> String {
    paint("1", text)
}

fn cyan(text: &str) -> String {
    paint("36", text)
}

/// Print a section header, e.g. the title above a run of steps.
pub(crate) fn header(title: &str) {
    eprintln!("{}", bold(title));
}

/// Format a duration compactly: `842ms`, `3.9s`, or `2m04s`.
fn format_elapsed(elapsed: Duration) -> String {
    let secs = elapsed.as_secs_f64();
    if secs < 1.0 {
        format!("{}ms", elapsed.as_millis())
    } else if secs < 60.0 {
        format!("{secs:.1}s")
    } else {
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        format!("{minutes}m{seconds:02}s")
    }
}

/// Begin a progress step. Prints `  ▸ <label> … ` without a trailing newline
/// and returns a handle that completes the line when the work is done.
#[must_use = "a step must be completed with `.done()` or `.done_with(..)`"]
pub(crate) fn step(label: &str) -> Step {
    eprint!("  {} {} {} ", cyan("▸"), label, dim("…"));
    let _ = std::io::stderr().flush();
    Step {
        start: Instant::now(),
    }
}

/// An in-progress step, returned by [`step`].
pub(crate) struct Step {
    start: Instant,
}

impl Step {
    /// Finish the step, appending only the elapsed time.
    pub(crate) fn done(self) {
        eprintln!("{}", dim(&format_elapsed(self.start.elapsed())));
    }

    /// Finish the step with a result message, followed by the elapsed time.
    pub(crate) fn done_with(self, message: &str) {
        eprintln!(
            "{message} {}",
            dim(&format!("({})", format_elapsed(self.start.elapsed()))),
        );
    }
}
