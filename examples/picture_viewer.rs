// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! Picture viewer — display an image file in the terminal.
//!
//! Usage: cargo run --example picture_viewer <image_path>

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event},
    style::ResetColor,
    terminal,
};
use image::DynamicImage;
use std::io::{Error, Write, stdout};
use termcanvas::{CharRect, TerminalCanvas};

fn show_image<T: Write + QueueableCommand>(s: &mut T, path: &str) -> Result<(), Error> {
    let pic: DynamicImage = match image::open(path) {
        Err(e) => {
            return Err(Error::other(format!("ImageError: {e:?}")));
        }
        Ok(pic) => pic,
    };

    let (cols, rows) = terminal::size()?;

    // Reserve top row for the info line and bottom row for the prompt
    let rect = CharRect {
        left: 0,
        top: 1,
        width: cols,
        height: rows - 2,
    };
    let mut canvas = TerminalCanvas::new(&rect);
    canvas.show_image(&pic);

    s.queue(terminal::Clear(terminal::ClearType::All))?;
    s.queue(cursor::Hide)?;

    // Info line
    s.queue(cursor::MoveTo(0, 0))?;
    s.queue(ResetColor)?;
    write!(
        s,
        "File: {}  |  Terminal: {}x{} chars  |  Canvas: {}x{} px",
        path,
        cols,
        rows,
        cols,
        2 * (rows - 2),
    )?;

    canvas.render(s)?;

    // Prompt on the last row
    s.queue(cursor::MoveTo(0, rows - 1))?;
    s.queue(ResetColor)?;
    write!(s, "Press any key to quit")?;

    s.flush()
}

fn main() -> Result<(), Error> {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: cargo run --example picture_viewer <image_path>");
            std::process::exit(1);
        }
    };

    let mut stdout = stdout();
    terminal::enable_raw_mode()?;

    let res = show_image(&mut stdout, &path);
    if let Err(e) = res {
        terminal::disable_raw_mode()?;
        eprintln!("\nError: {e:?}");
        return Err(e);
    }

    // Wait for any keypress
    loop {
        if let Ok(Event::Key(key_event)) = event::read()
            && key_event.kind == event::KeyEventKind::Press
        {
            break;
        }
    }

    // Restore terminal
    let (_, rows) = terminal::size()?;
    stdout.queue(terminal::Clear(terminal::ClearType::All))?;
    stdout.queue(cursor::MoveTo(0, rows - 1))?;
    stdout.queue(ResetColor)?;
    stdout.queue(cursor::Show)?;
    stdout.flush()?;
    terminal::disable_raw_mode()?;

    Ok(())
}
