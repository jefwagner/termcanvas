// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! Snake — a classic snake game rendered with TerminalCanvas.
//!
//! Usage: cargo run --example snake
//!
//! Controls:
//!   Arrow keys — change direction
//!   Ctrl-C     — quit

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::ResetColor,
    terminal,
};
use image::{GenericImage, Rgba};
use rand::Rng;
use std::collections::VecDeque;
use std::io::{Error, Write, stdout};
use std::time::{Duration, Instant};
use termcanvas::{CharRect, TerminalCanvas};

// ── Colors ────────────────────────────────────────────────────────────────────

const COLOR_BG: Rgba<u8> = Rgba([20, 20, 20, 255]);
const COLOR_SNAKE_HEAD: Rgba<u8> = Rgba([120, 220, 80, 255]);
const COLOR_SNAKE_BODY: Rgba<u8> = Rgba([60, 160, 40, 255]);
const COLOR_FOOD: Rgba<u8> = Rgba([220, 50, 50, 255]);

// ── Game constants ────────────────────────────────────────────────────────────

/// Number of header rows above the canvas (score + instructions)
const HEADER_ROWS: u16 = 2;
/// Milliseconds between game ticks
const TICK_MS: u64 = 150;

// ── Direction ─────────────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, PartialEq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    /// Returns true if the two directions are exact opposites (illegal reversal)
    fn is_opposite(self, other: Dir) -> bool {
        matches!(
            (self, other),
            (Dir::Up, Dir::Down)
                | (Dir::Down, Dir::Up)
                | (Dir::Left, Dir::Right)
                | (Dir::Right, Dir::Left)
        )
    }
}

// ── Game state ────────────────────────────────────────────────────────────────

struct Game {
    /// Width of the play field in pixels
    board_w: u16,
    /// Height of the play field in pixels
    board_h: u16,
    /// Snake body — front is head, back is tail
    snake: VecDeque<(u16, u16)>,
    /// Current movement direction
    dir: Dir,
    /// Buffered next direction (from player input)
    next_dir: Dir,
    /// Food position
    food: (u16, u16),
    /// Current score
    score: u32,
    /// Whether the game is still running
    alive: bool,
}

impl Game {
    fn new(board_w: u16, board_h: u16) -> Self {
        // Start the snake in the middle, pointing right, 3 segments long
        let mid_x = board_w / 2;
        let mid_y = board_h / 2;
        let mut snake = VecDeque::new();
        snake.push_back((mid_x, mid_y));
        snake.push_back((mid_x - 1, mid_y));
        snake.push_back((mid_x - 2, mid_y));

        let mut game = Self {
            board_w,
            board_h,
            snake,
            dir: Dir::Right,
            next_dir: Dir::Right,
            food: (0, 0),
            score: 0,
            alive: true,
        };
        game.place_food();
        game
    }

    /// Place food in a random cell not occupied by the snake
    fn place_food(&mut self) {
        let mut rng = rand::rng();
        loop {
            let fx = rng.random_range(0..self.board_w);
            let fy = rng.random_range(0..self.board_h);
            if !self.snake.contains(&(fx, fy)) {
                self.food = (fx, fy);
                return;
            }
        }
    }

    /// Advance the game by one tick; returns false if the game ended
    fn tick(&mut self) {
        // Commit buffered direction
        self.dir = self.next_dir;

        let (hx, hy) = *self.snake.front().expect("snake is never empty");

        // Compute new head position — signed arithmetic to detect wall hits
        let (nx, ny) = match self.dir {
            Dir::Up => (hx as i32, hy as i32 - 1),
            Dir::Down => (hx as i32, hy as i32 + 1),
            Dir::Left => (hx as i32 - 1, hy as i32),
            Dir::Right => (hx as i32 + 1, hy as i32),
        };

        // Wall collision
        if nx < 0 || ny < 0 || nx >= self.board_w as i32 || ny >= self.board_h as i32 {
            self.alive = false;
            return;
        }
        let (nx, ny) = (nx as u16, ny as u16);

        // Self collision (ignore tail tip which is about to vacate)
        let tail = self.snake.back().copied();
        if self.snake.contains(&(nx, ny)) && Some((nx, ny)) != tail {
            self.alive = false;
            return;
        }

        self.snake.push_front((nx, ny));

        if (nx, ny) == self.food {
            // Eat food — grow (don't remove tail) and place new food
            self.score += 1;
            self.place_food();
        } else {
            self.snake.pop_back();
        }
    }

    /// Buffer a direction change (ignores illegal reversals)
    fn steer(&mut self, new_dir: Dir) {
        if !self.dir.is_opposite(new_dir) {
            self.next_dir = new_dir;
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Draw the current game state onto the canvas
fn draw(canvas: &mut TerminalCanvas, game: &Game) {
    canvas.fill(&COLOR_BG);

    // Food
    let (fx, fy) = game.food;
    canvas.put_pixel(fx as u32, fy as u32, COLOR_FOOD);

    // Snake body (draw body first, then head on top)
    for (i, &(x, y)) in game.snake.iter().enumerate() {
        let color = if i == 0 {
            COLOR_SNAKE_HEAD
        } else {
            COLOR_SNAKE_BODY
        };
        canvas.put_pixel(x as u32, y as u32, color);
    }
}

/// Write the header lines (score + instructions) above the canvas
fn draw_header<T: Write + QueueableCommand>(s: &mut T, score: u32, cols: u16) -> Result<(), Error> {
    // Row 0 — score
    s.queue(cursor::MoveTo(0, 0))?;
    s.queue(ResetColor)?;
    let score_str = format!("Score: {score}");
    // Pad to full width so previous content is overwritten
    write!(s, "{score_str:<width$}", width = cols as usize)?;

    // Row 1 — instructions
    s.queue(cursor::MoveTo(0, 1))?;
    let inst = "Arrow keys: move   Ctrl-C: quit";
    write!(s, "{inst:<width$}", width = cols as usize)?;

    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn run<T: Write + QueueableCommand>(s: &mut T) -> Result<(), Error> {
    let (cols, rows) = terminal::size()?;

    // Canvas occupies everything below the header, one char row = two pixel rows
    let canvas_char_rows = rows - HEADER_ROWS;
    let rect = CharRect {
        left: 0,
        top: HEADER_ROWS,
        width: cols,
        height: canvas_char_rows,
    };

    // Game board in pixels matches the canvas pixel dimensions
    let board_w = cols;
    let board_h = canvas_char_rows * 2;

    let mut canvas = TerminalCanvas::new(&rect);
    let mut game = Game::new(board_w, board_h);

    s.queue(terminal::Clear(terminal::ClearType::All))?;
    s.queue(cursor::Hide)?;

    let tick_duration = Duration::from_millis(TICK_MS);
    let mut last_tick = Instant::now();

    // ── Game loop ─────────────────────────────────────────────────────────────
    while game.alive {
        // Poll for input without blocking longer than the remaining tick time
        let elapsed = last_tick.elapsed();
        let timeout = tick_duration.saturating_sub(elapsed);

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(ke) if ke.kind == KeyEventKind::Press => {
                    match ke.code {
                        // Quit on Ctrl-C
                        KeyCode::Char('c') if ke.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Up => game.steer(Dir::Up),
                        KeyCode::Down => game.steer(Dir::Down),
                        KeyCode::Left => game.steer(Dir::Left),
                        KeyCode::Right => game.steer(Dir::Right),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Advance the game on each tick
        if last_tick.elapsed() >= tick_duration {
            game.tick();
            last_tick = Instant::now();

            draw(&mut canvas, &game);
            draw_header(s, game.score, cols)?;
            canvas.render(s)?;
        }
    }

    // ── Game over screen ──────────────────────────────────────────────────────
    let msg = format!("Game over!  Final score: {}  — press any key", game.score);
    let x = cols.saturating_sub(msg.len() as u16) / 2;
    let y = rows / 2;
    s.queue(cursor::MoveTo(x, y))?;
    s.queue(ResetColor)?;
    write!(s, "{msg}")?;
    s.flush()?;

    // Wait for a keypress before exiting
    loop {
        if let Ok(Event::Key(ke)) = event::read()
            && ke.kind == KeyEventKind::Press
        {
            break;
        }
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let mut stdout = stdout();
    terminal::enable_raw_mode()?;

    let res = run(&mut stdout);

    // Always restore the terminal, even on error
    let (_, rows) = terminal::size()?;
    stdout.queue(terminal::Clear(terminal::ClearType::All))?;
    stdout.queue(cursor::MoveTo(0, rows - 1))?;
    stdout.queue(ResetColor)?;
    stdout.queue(cursor::Show)?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;

    if let Err(e) = res {
        eprintln!("\nError: {e:?}");
        return Err(e);
    }

    Ok(())
}
