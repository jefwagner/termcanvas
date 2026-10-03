// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! termsnake binary — a thin wrapper around the [`SnakeApp`] library.
//!
//! All game logic lives in the library target so the future arcade shell can
//! embed the game; the binary only wires the app to the loop.

use std::io::{Result, stdout};
use termcanvas_loop::run;
use termsnake::SnakeApp;

fn main() -> Result<()> {
    run(&mut stdout(), &mut SnakeApp::new())
}
