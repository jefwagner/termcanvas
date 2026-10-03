// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! Terminal lifecycle and render/game loop for `termcanvas` apps.
//!
//! This crate owns everything global and terminal-shaped so that apps don't
//! have to: raw mode, the alternate screen, cursor visibility, panic- and
//! error-path restoration, input dispatch, and tick pacing. An application
//! implements the [`App`] trait and hands itself to [`run`]; it never touches
//! global terminal state itself.
//!
//! # Ownership decisions
//!
//! - **The loop owns the terminal lifecycle.** [`run`] enables raw mode,
//!   enters the alternate screen, and hides the cursor; a drop guard restores
//!   all of it — including when the app panics or returns an error. Apps
//!   draw only into the [`CharRect`] that the loop gives them.
//! - **Ctrl-C is a loop-level safety exit.** The loop intercepts Ctrl-C
//!   itself and shuts down cleanly, even if an app never matches it. Apps
//!   see all other key events via [`App::handle_event`] and can exit on any
//!   of them by returning [`Flow::Exit`].
//! - **Input flows through the loop.** All events reach the app through
//!   [`App::handle_event`], so a future host application (e.g. an arcade
//!   shell) can dispatch, filter, or reroute events for embedded apps.
//! - **Resize re-layouts the canvas.** On a resize event the loop re-queries
//!   the layout from [`App::layout`], resizes the canvas, and reports the
//!   event to the app.
//!
//! # Example
//!
//! ```no_run
//! use crossterm::{QueueableCommand, event::{Event, KeyCode}};
//! use std::io::{Result, Write};
//! use std::time::Duration;
//! use termcanvas::{CharRect, TerminalCanvas};
//! use termcanvas_loop::{App, Flow, LoopCtx, run};
//!
//! struct Clicker;
//!
//! impl App for Clicker {
//!     fn tick_interval(&self) -> Duration { Duration::from_millis(100) }
//!
//!     fn layout(&mut self, cols: u16, rows: u16) -> CharRect {
//!         CharRect { left: 0, top: 0, width: cols, height: rows }
//!     }
//!
//!     fn handle_event(&mut self, ev: Event, _ctx: &LoopCtx) -> Result<Flow> {
//!         // Any key press quits (Ctrl-C is already handled by the loop).
//!         match ev {
//!             Event::Key(k) if k.kind == crossterm::event::KeyEventKind::Press => {
//!                 Ok(Flow::Exit)
//!             }
//!             _ => Ok(Flow::Continue),
//!         }
//!     }
//!
//!     fn tick(&mut self, _dt: Duration, _ctx: &LoopCtx) -> Result<Flow> {
//!         Ok(Flow::Continue)
//!     }
//!
//!     fn draw<T: Write + QueueableCommand>(
//!         &mut self,
//!         _out: &mut T,
//!         canvas: &mut TerminalCanvas,
//!         _ctx: &LoopCtx,
//!     ) -> Result<()> {
//!         canvas.fill(&image::Rgba([20, 20, 20, 255]));
//!         Ok(())
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     run(&mut std::io::stdout(), &mut Clicker)
//! }
//! ```

use crossterm::{
    QueueableCommand, cursor, event,
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use termcanvas::{CharRect, TerminalCanvas};

/// What the app wants the loop to do after a callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Keep running.
    Continue,
    /// Shut down cleanly; the loop restores the terminal and returns.
    Exit,
}

/// The scoped context handed to the app on every callback.
///
/// The app learns everything about the terminal through this — it never
/// queries the terminal itself.
#[derive(Debug, Copy, Clone)]
pub struct LoopCtx {
    /// Terminal size as `(columns, rows)` in character cells.
    pub terminal: (u16, u16),
    /// The character-cell rectangle the app's canvas occupies.
    pub canvas_rect: CharRect,
}

/// The application logic driving the loop.
///
/// All methods are called from the loop, in loop order: [`handle_event`]
/// (when input arrives), then [`tick`] and [`draw`] on each tick deadline.
/// [`tick_interval`] and [`layout`] are consulted once at startup and again
/// on resize.
///
/// [`handle_event`]: App::handle_event
/// [`tick`]: App::tick
/// [`draw`]: App::draw
/// [`tick_interval`]: App::tick_interval
/// [`layout`]: App::layout
pub trait App {
    /// Interval between ticks. Read once when the loop starts.
    fn tick_interval(&self) -> Duration;

    /// Lay out the canvas for a terminal of `cols` × `rows` character cells.
    ///
    /// The returned rect is where [`draw`](App::draw) will be asked to paint.
    /// Apps that need space above or below the canvas (a score header, a
    /// prompt line) return a rect that leaves those rows out — the leftover
    /// terminal rows belong to the app to decorate via `out`.
    fn layout(&mut self, cols: u16, rows: u16) -> CharRect;

    /// Called once after terminal setup, before the loop starts.
    fn init(&mut self, _ctx: &LoopCtx) -> io::Result<()> {
        Ok(())
    }

    /// Handle one input event (including resize notifications). Return
    /// [`Flow::Exit`] to stop the loop.
    fn handle_event(&mut self, ev: Event, ctx: &LoopCtx) -> io::Result<Flow>;

    /// Advance the app by `dt` — the time elapsed since the previous tick.
    /// Return [`Flow::Exit`] to stop the loop.
    fn tick(&mut self, dt: Duration, ctx: &LoopCtx) -> io::Result<Flow>;

    /// Paint the current state: game content into `canvas`, and any
    /// text chrome (headers, prompts) directly to `out`. The loop renders
    /// the canvas right after this returns.
    fn draw<T: Write + QueueableCommand>(
        &mut self,
        out: &mut T,
        canvas: &mut TerminalCanvas,
        ctx: &LoopCtx,
    ) -> io::Result<()>;
}

/// Run the app until it (or Ctrl-C) asks to stop, owning the terminal
/// throughout.
///
/// Sets up raw mode, the alternate screen, and a hidden cursor; runs the
/// event/tick/draw cycle; and restores the terminal on every exit path —
/// normal return, error, or panic unwind.
///
/// Restoration is performed on the process's real stdout, since raw mode and
/// the alternate screen are process-global tty state.
///
/// # Errors
///
/// Returns any I/O error from terminal setup, event polling, canvas
/// rendering, or the app's own callbacks.
pub fn run<T, A>(out: &mut T, app: &mut A) -> io::Result<()>
where
    T: Write + QueueableCommand,
    A: App,
{
    let _guard = TerminalGuard::enter()?;

    let (cols, rows) = terminal::size()?;
    let rect = app.layout(cols, rows);
    let mut ctx = LoopCtx {
        terminal: (cols, rows),
        canvas_rect: rect,
    };
    let mut canvas = TerminalCanvas::new(&rect);

    out.queue(terminal::Clear(terminal::ClearType::All))?;
    out.flush()?;

    app.init(&ctx)?;

    let interval = app.tick_interval();
    let mut last_tick = Instant::now();

    loop {
        // Poll with a deadline so input never delays a tick, and idle time
        // never spins the loop.
        if event::poll(next_timeout(last_tick, interval))? {
            let ev = event::read()?;

            // Ctrl-C is a loop-level safety exit: the loop handles it even
            // if the app never does.
            let ctrl_c = matches!(
                &ev,
                Event::Key(k)
                    if k.kind == KeyEventKind::Press
                        && k.code == KeyCode::Char('c')
                        && k.modifiers.contains(KeyModifiers::CONTROL)
            );
            if ctrl_c {
                return Ok(());
            }

            // Resize: re-layout the canvas, then report to the app.
            if let Event::Resize(ncols, nrows) = ev {
                ctx.terminal = (ncols, nrows);
                ctx.canvas_rect = app.layout(ncols, nrows);
                canvas.resize(&ctx.canvas_rect);
            }

            if app.handle_event(ev, &ctx)? == Flow::Exit {
                return Ok(());
            }
        }

        // Advance and draw on schedule.
        let elapsed = last_tick.elapsed();
        if elapsed >= interval {
            last_tick = Instant::now();
            if app.tick(elapsed, &ctx)? == Flow::Exit {
                return Ok(());
            }
            app.draw(out, &mut canvas, &ctx)?;
            canvas.render(out)?;
        }
    }
}

/// Time to wait in the next event poll: the remaining fraction of the tick
/// interval.
fn next_timeout(last_tick: Instant, interval: Duration) -> Duration {
    interval.saturating_sub(last_tick.elapsed())
}

/// RAII guard for the loop-owned terminal state.
///
/// `enter` enables raw mode, enters the alternate screen, and hides the
/// cursor; `drop` restores all three, so panic unwinding through `run` still
/// leaves the terminal usable.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        out.queue(terminal::EnterAlternateScreen)?;
        out.queue(cursor::Hide)?;
        out.flush()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut out = io::stdout();
        let _ = out.queue(terminal::LeaveAlternateScreen);
        let _ = out.queue(cursor::Show);
        let _ = out.flush();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_next_timeout_is_remaining_interval() {
        let interval = Duration::from_millis(100);
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));
        let timeout = next_timeout(start, interval);
        // Time has elapsed, so strictly less than the interval remains
        // (exact remaining time is scheduler-dependent, don't pin it).
        assert!(timeout < interval);
        // A fresh tick waits essentially the whole interval (exact equality
        // is impossible — the elapsed() query itself takes ~100ns)
        let fresh = next_timeout(Instant::now(), interval);
        assert!(fresh > interval - Duration::from_millis(1));
        assert!(fresh <= interval);
        // An overdue tick doesn't wait at all
        assert_eq!(next_timeout(start - interval, interval), Duration::ZERO);
    }
}
