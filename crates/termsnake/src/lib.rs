// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! termsnake — classic snake, rendered with `termcanvas` on the
//! `termcanvas-loop` lifecycle.
//!
//! All game logic lives in this library target so a future host (the planned
//! arcade shell) can embed the game as a sub-app: the binary is a thin
//! wrapper that hands a [`SnakeApp`] to `termcanvas_loop::run`. The
//! arcade-readiness rules the library obeys:
//!
//! - `SnakeApp` owns no global terminal state — the loop does.
//! - It draws into the loop-provided [`CharRect`], sized at runtime.
//! - All input flows through the loop's event dispatch.
//! - No logic that only makes sense in a standalone binary lives here.

use crossterm::{
    QueueableCommand, cursor,
    event::{Event, KeyCode, KeyEventKind},
};
use image::{GenericImage, Rgba};
use rand::Rng;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::Duration;
use termcanvas::{CharRect, TerminalCanvas};
use termcanvas_loop::{App, Flow, LoopCtx};

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

    fn step(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Dir::Up => (x, y - 1),
            Dir::Down => (x, y + 1),
            Dir::Left => (x - 1, y),
            Dir::Right => (x + 1, y),
        }
    }
}

// ── Game state ────────────────────────────────────────────────────────────────

/// The pure game model: a snake on a `board_w × board_h` pixel board.
///
/// Deliberately free of terminal concerns so it can be unit-tested headless
/// and reused by any host.
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

    /// Advance the game by one tick; sets `alive = false` on game end
    fn tick(&mut self) {
        // Commit buffered direction
        self.dir = self.next_dir;

        let (hx, hy) = *self.snake.front().expect("snake is never empty");

        // Compute new head position — signed arithmetic to detect wall hits
        let (nx, ny) = self.dir.step(i32::from(hx), i32::from(hy));

        // Wall collision
        if nx < 0 || ny < 0 || nx >= i32::from(self.board_w) || ny >= i32::from(self.board_h) {
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

// ── The app: game + loop integration ─────────────────────────────────────────

/// The snake game as a `termcanvas-loop` application.
///
/// Standalone, `run(&mut stdout(), &mut SnakeApp::new())` plays the game.
/// Embedded (future arcade), the host drives the same `App` trait — the
/// game neither knows nor cares which.
pub struct SnakeApp {
    /// The game model; created in `init` once the loop supplies the layout.
    game: Option<Game>,
}

impl SnakeApp {
    /// Create an app with no game yet; the game is born in `init`, when the
    /// loop has supplied the terminal layout.
    pub fn new() -> Self {
        Self { game: None }
    }

    /// The game, once initialized.
    fn game(&mut self) -> &mut Game {
        self.game.as_mut().expect("init called before the loop ran")
    }
}

impl App for SnakeApp {
    fn tick_interval(&self) -> Duration {
        Duration::from_millis(TICK_MS)
    }

    fn layout(&mut self, cols: u16, rows: u16) -> CharRect {
        CharRect {
            left: 0,
            top: HEADER_ROWS,
            width: cols,
            height: rows.saturating_sub(HEADER_ROWS),
        }
    }

    fn init(&mut self, ctx: &LoopCtx) -> io::Result<()> {
        // The game board in pixels matches the canvas pixel dimensions
        let rect = ctx.canvas_rect;
        self.game = Some(Game::new(rect.width, 2 * rect.height));
        Ok(())
    }

    fn handle_event(&mut self, ev: Event, _ctx: &LoopCtx) -> io::Result<Flow> {
        let Event::Key(ke) = ev else {
            return Ok(Flow::Continue);
        };
        if ke.kind != KeyEventKind::Press {
            return Ok(Flow::Continue);
        }
        match ke.code {
            KeyCode::Up => self.game().steer(Dir::Up),
            KeyCode::Down => self.game().steer(Dir::Down),
            KeyCode::Left => self.game().steer(Dir::Left),
            KeyCode::Right => self.game().steer(Dir::Right),
            // After death, any key quits
            _ if !self.game.as_ref().is_none_or(|g| g.alive) => return Ok(Flow::Exit),
            _ => {}
        }
        Ok(Flow::Continue)
    }

    fn tick(&mut self, _dt: Duration, _ctx: &LoopCtx) -> io::Result<Flow> {
        if self.game.as_ref().is_none_or(|g| g.alive) {
            self.game().tick();
        }
        // Stay in the loop while dead so draw() can show the game-over
        // screen; a key press (any key) exits via handle_event.
        Ok(Flow::Continue)
    }

    fn draw<T: Write + QueueableCommand>(
        &mut self,
        out: &mut T,
        canvas: &mut TerminalCanvas,
        ctx: &LoopCtx,
    ) -> io::Result<()> {
        let (cols, rows) = ctx.terminal;
        let Some(game) = self.game.as_ref() else {
            return Ok(());
        };

        if game.alive {
            draw_game(canvas, game);
        } else {
            draw_game(canvas, game);
            draw_game_over(out, game.score, cols, rows)?;
        }

        // Header above the canvas: score + instructions, padded to the full
        // width so previous content is overwritten.
        out.queue(cursor::MoveTo(0, 0))?;
        write!(
            out,
            "{:<width$}",
            format!("Score: {}", game.score),
            width = cols as usize
        )?;
        out.queue(cursor::MoveTo(0, 1))?;
        write!(
            out,
            "{:<width$}",
            "Arrow keys: move   Ctrl-C: quit",
            width = cols as usize
        )?;
        Ok(())
    }
}

/// Draw the current game state onto the canvas
fn draw_game(canvas: &mut TerminalCanvas, game: &Game) {
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

/// Center the game-over message on the terminal
fn draw_game_over<T: Write + QueueableCommand>(
    s: &mut T,
    score: u32,
    cols: u16,
    rows: u16,
) -> io::Result<()> {
    let msg = format!("Game over!  Final score: {score}  — press any key");
    let x = cols.saturating_sub(msg.len() as u16) / 2;
    let y = rows / 2;
    s.queue(cursor::MoveTo(x, y))?;
    write!(s, "{msg}")
}

impl Default for SnakeApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_snake_moves_on_tick() {
        let mut game = Game::new(20, 10);
        let head = *game.snake.front().expect("snake is never empty");
        game.tick();
        // Started pointing right
        assert_eq!(
            *game.snake.front().expect("snake is never empty"),
            (head.0 + 1, head.1)
        );
        assert_eq!(game.snake.len(), 3);
        assert!(game.alive);
    }

    #[test]
    fn test_steer_ignores_reversal() {
        let mut game = Game::new(20, 10);
        game.steer(Dir::Left);
        game.tick();
        // Still moving right — the reversal was ignored
        assert_eq!(game.dir, Dir::Right);
    }

    #[test]
    fn test_steer_accepts_perpendicular() {
        let mut game = Game::new(20, 10);
        let head = *game.snake.front().expect("snake is never empty");
        game.steer(Dir::Up);
        game.tick();
        assert_eq!(
            *game.snake.front().expect("snake is never empty"),
            (head.0, head.1 - 1)
        );
    }

    #[test]
    fn test_wall_collision_ends_game() {
        let mut game = Game::new(20, 10);
        // Head at the right edge, food far away
        game.snake = VecDeque::from([(19, 5), (18, 5), (17, 5)]);
        game.food = (1, 1);
        game.tick();
        assert!(!game.alive);
    }

    #[test]
    fn test_self_collision_ends_game() {
        let mut game = Game::new(20, 10);
        // Head at (5,5), body touching (6,5); moving right runs into it and
        // the tail does not vacate in time
        game.snake = VecDeque::from([(5, 5), (6, 5), (5, 6)]);
        game.food = (1, 1);
        game.tick();
        assert!(!game.alive);
    }

    #[test]
    fn test_eating_food_grows_snake() {
        let mut game = Game::new(20, 10);
        // Food directly ahead of the head
        let head = *game.snake.front().expect("snake is never empty");
        game.food = (head.0 + 1, head.1);
        game.tick();
        assert_eq!(game.score, 1);
        assert_eq!(game.snake.len(), 4);
        assert!(game.alive);
    }

    #[test]
    fn test_app_lays_out_below_header() {
        let mut app = SnakeApp::new();
        let rect = app.layout(80, 24);
        assert_eq!(
            rect,
            CharRect {
                left: 0,
                top: HEADER_ROWS,
                width: 80,
                height: 22
            }
        );
    }
}
