// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! Pixel-graphics canvas for text-based terminals.
//!
//! `termcanvas` exposes a canvas-like API for drawing colored pixels directly
//! inside a running terminal emulator. The canvas is backed by two in-memory
//! pixel buffers (current and previous). When you call
//! [`TerminalCanvas::render`] only the character cells that changed since the
//! last render are written to the terminal stream, keeping output minimal and
//! flicker-free.
//!
//! Pixels are encoded using the Unicode UPPER HALF BLOCK character (`▀`)
//! combined with ANSI 24-bit foreground and background colors. This lets each
//! character cell carry two vertically-stacked pixels, so a terminal window
//! that is `W` columns wide and `H` rows tall gives you a pixel canvas of
//! `W × 2H` pixels.
//!
//! [`TerminalCanvas`] implements [`image::GenericImage`] and
//! [`image::GenericImageView`], so any drawing function from the `image` crate
//! that accepts a `GenericImage` — including image blitting, resizing, and
//! compositing — can paint directly onto the canvas.
//!
//! # Terminal requirements
//!
//! `termcanvas` requires a modern terminal emulator with:
//!
//! - **Unicode support** — the `▀` (U+2580) half-block character must render
//!   as a single, full-width glyph.
//! - **24-bit (truecolor) ANSI color** — both foreground and background colors
//!   are set with `ESC[38;2;r;g;bm` / `ESC[48;2;r;g;bm` sequences. Terminals
//!   that only support 8 or 256 colors will produce incorrect output.
//! - **Raw mode** — the application is responsible for enabling and disabling
//!   raw mode via [`crossterm::terminal::enable_raw_mode`] /
//!   [`crossterm::terminal::disable_raw_mode`] around the drawing loop.
//!
//! Most modern terminal emulators (kitty, iTerm2, Alacritty, WezTerm, Windows
//! Terminal, and recent versions of GNOME Terminal and macOS Terminal.app)
//! satisfy all three requirements.
//!
//! # Example
//!
//! The following program fills the canvas with a dark background, draws a
//! colored rectangle, places a single bright pixel, renders everything to
//! stdout, then waits for a keypress before restoring the terminal.
//!
//! ```no_run
//! use crossterm::{
//!     QueueableCommand, cursor,
//!     event::{self, Event, KeyEventKind},
//!     style::ResetColor,
//!     terminal,
//! };
//! use image::{GenericImage, Rgba};
//! use termcanvas::{CharRect, TerminalCanvas};
//! use std::io::{Write, stdout};
//!
//! fn main() -> std::io::Result<()> {
//!     let mut out = stdout();
//!     terminal::enable_raw_mode()?;
//!     out.queue(terminal::Clear(terminal::ClearType::All))?;
//!     out.queue(cursor::Hide)?;
//!
//!     // Create a canvas covering the full terminal, leaving one row at the
//!     // top for a status line and one at the bottom for a prompt.
//!     let (cols, rows) = terminal::size()?;
//!     let rect = CharRect { left: 0, top: 1, width: cols, height: rows - 2 };
//!     let mut canvas = TerminalCanvas::new(&rect);
//!
//!     // Fill the whole canvas with a dark background color.
//!     canvas.fill(&Rgba([20, 20, 20, 255]));
//!
//!     // Draw a solid blue rectangle by setting individual pixels.
//!     let blue = Rgba([30, 100, 220, 255]);
//!     for y in 10..40 {
//!         for x in 10..60 {
//!             canvas.put_pixel(x, y, blue);
//!         }
//!     }
//!
//!     // Place a single bright-white pixel in the center of the rectangle.
//!     canvas.put_pixel(34, 24, Rgba([255, 255, 255, 255]));
//!
//!     // Write only the changed cells to the terminal.
//!     canvas.render(&mut out)?;
//!
//!     // Status line above the canvas.
//!     out.queue(cursor::MoveTo(0, 0))?;
//!     out.queue(ResetColor)?;
//!     write!(out, "Press any key to quit")?;
//!     out.flush()?;
//!
//!     // Wait for a keypress.
//!     loop {
//!         if let Ok(Event::Key(ke)) = event::read()
//!             && ke.kind == KeyEventKind::Press
//!         {
//!             break;
//!         }
//!     }
//!
//!     // Restore the terminal.
//!     out.queue(terminal::Clear(terminal::ClearType::All))?;
//!     out.queue(cursor::MoveTo(0, rows - 1))?;
//!     out.queue(ResetColor)?;
//!     out.queue(cursor::Show)?;
//!     out.flush()?;
//!     terminal::disable_raw_mode()?;
//!     Ok(())
//! }
//! ```

use crossterm::{
    QueueableCommand, cursor,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, Pixel, Rgba};
use std::io::{self, Write};

/// The half-block box-element character
const VERT_HALF_BLOCK: &str = "▀";

#[derive(Debug, Copy, Clone)]
struct VertPair {
    top: Rgba<u8>,
    bot: Rgba<u8>,
}

/// A rectangle in terminal character-cell coordinates.
///
/// All coordinates are measured in character cells, where `(0, 0)` is the
/// top-left corner of the terminal. Because each character cell encodes two
/// vertical pixels via the `▀` half-block character, the pixel height of a
/// `CharRect` is always `2 * height`.
///
/// # Examples
///
/// ```
/// use termcanvas::CharRect;
///
/// let rect = CharRect { left: 0, top: 2, width: 80, height: 20 };
/// assert_eq!(rect.width, 80);
/// // Pixel height is twice the character height
/// assert_eq!(rect.height * 2, 40);
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CharRect {
    /// Column of the left edge (0-indexed from the left of the terminal).
    pub left: u16,
    /// Row of the top edge (0-indexed from the top of the terminal).
    pub top: u16,
    /// Width in character columns.
    pub width: u16,
    /// Height in character rows (pixel height = `2 * height`).
    pub height: u16,
}

/// A pixel canvas occupying a rectangular region of the terminal.
///
/// `TerminalCanvas` maintains two internal pixel buffers — a *current* buffer
/// that you draw into and a *previous* buffer that records what was last
/// rendered. On each call to [`render`](TerminalCanvas::render) only the
/// character cells whose top/bottom pixel pair has changed are written to the
/// terminal stream, minimising flicker and I/O overhead.
///
/// Pixel coordinates use the top-left origin convention: `(0, 0)` is the
/// top-left pixel of the canvas. Because each terminal character cell encodes
/// two vertical pixels via the `▀` half-block character, the pixel height of
/// the canvas is always `2 * rect.height`.
///
/// `TerminalCanvas` implements [`image::GenericImageView`] and
/// [`image::GenericImage`], so any drawing function from the `image` crate
/// that accepts a `GenericImage` can paint directly onto the canvas.
#[derive(Debug, Clone)]
pub struct TerminalCanvas {
    /// The rectagle inside the terminal that holds the pixel-canvas
    rect: CharRect,
    /// The buffer of pixels to be rendered to the canvas
    cur_buf: ImageBuffer<Rgba<u8>, Vec<u8>>,
    /// A previous buffer of pixesl rendered to the canvas,
    /// only used internally for the diff
    prev_buf: ImageBuffer<Rgba<u8>, Vec<u8>>,
    /// A list of changes between current buffer and previous buffer
    diff: Vec<(u16, u16, VertPair)>,
}

impl TerminalCanvas {
    /// Create a new `TerminalCanvas` covering the given character-cell rectangle.
    ///
    /// The current pixel buffer is initialised to fully opaque black
    /// (`Rgba([0, 0, 0, 255])`). The previous buffer is initialised to fully
    /// transparent black (`Rgba([0, 0, 0, 0])`), which guarantees that the
    /// first call to [`render`](TerminalCanvas::render) will write every cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use image::GenericImageView;
    ///
    /// let rect = CharRect { left: 0, top: 0, width: 40, height: 10 };
    /// let canvas = TerminalCanvas::new(&rect);
    ///
    /// // Pixel dimensions are width × (2 * height)
    /// assert_eq!(canvas.dimensions(), (40, 20));
    /// ```
    pub fn new(rect: &CharRect) -> Self {
        let pix_cols = rect.width as u32;
        let pix_rows = 2 * rect.height as u32;
        let cur_buf = ImageBuffer::from_pixel(pix_cols, pix_rows, Rgba([0, 0, 0, 255]));
        let prev_buf = ImageBuffer::from_pixel(pix_cols, pix_rows, Rgba([0, 0, 0, 0]));
        let num_chars = (rect.width * rect.height) as usize;
        let diff = Vec::with_capacity(num_chars);
        Self {
            rect: *rect,
            cur_buf,
            prev_buf,
            diff,
        }
    }

    /// Resize the canvas to a new character-cell rectangle.
    ///
    /// Pixel data that fits within the new dimensions is preserved. Any pixels
    /// that fall outside the new bounds are discarded, and newly added pixels
    /// are initialised to fully opaque black (`Rgba([0, 0, 0, 255])`).
    ///
    /// Because the previous buffer is also resized, the next
    /// [`render`](TerminalCanvas::render) call will only redraw cells that
    /// genuinely changed relative to the last render before the resize.
    ///
    /// # Examples
    ///
    /// ```
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use image::GenericImageView;
    ///
    /// let rect = CharRect { left: 0, top: 0, width: 20, height: 5 };
    /// let mut canvas = TerminalCanvas::new(&rect);
    /// assert_eq!(canvas.dimensions(), (20, 10));
    ///
    /// // Grow the canvas
    /// let bigger = CharRect { left: 0, top: 0, width: 40, height: 10 };
    /// canvas.resize(&bigger);
    /// assert_eq!(canvas.dimensions(), (40, 20));
    ///
    /// // Shrink the canvas
    /// let smaller = CharRect { left: 0, top: 0, width: 10, height: 3 };
    /// canvas.resize(&smaller);
    /// assert_eq!(canvas.dimensions(), (10, 6));
    /// ```
    pub fn resize(&mut self, new_rect: &CharRect) {
        let new_pix_cols = new_rect.width as u32;
        let new_pix_rows = 2 * new_rect.height as u32;
        let mut new_cur_buf =
            ImageBuffer::from_pixel(new_pix_cols, new_pix_rows, Rgba([0, 0, 0, 255]));
        let mut new_prev_buf =
            ImageBuffer::from_pixel(new_pix_cols, new_pix_rows, Rgba([0, 0, 0, 0]));
        for y in 0..new_pix_rows {
            if y < 2 * self.rect.height as u32 {
                for x in 0..new_pix_cols {
                    if x < self.rect.width as u32 {
                        new_cur_buf.put_pixel(x, y, *self.cur_buf.get_pixel(x, y));
                        new_prev_buf.put_pixel(x, y, *self.prev_buf.get_pixel(x, y));
                    }
                }
            }
        }
        self.rect = *new_rect;
        self.cur_buf = new_cur_buf;
        self.prev_buf = new_prev_buf;
    }

    /// Fill every pixel in the canvas with a single solid color.
    ///
    /// This is typically called at the start of each frame to clear the canvas
    /// before drawing new content. Any subsequent call to
    /// [`render`](TerminalCanvas::render) will only write the cells that differ
    /// from the previous rendered frame.
    ///
    /// # Examples
    ///
    /// ```
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use image::{GenericImageView, Rgba};
    ///
    /// let rect = CharRect { left: 0, top: 0, width: 10, height: 5 };
    /// let mut canvas = TerminalCanvas::new(&rect);
    ///
    /// let red = Rgba([255, 0, 0, 255]);
    /// canvas.fill(&red);
    ///
    /// // Every pixel is now red
    /// assert_eq!(canvas.get_pixel(0, 0), red);
    /// assert_eq!(canvas.get_pixel(9, 9), red);
    /// ```
    pub fn fill(&mut self, fill_color: &Rgba<u8>) {
        for pixel in self.cur_buf.pixels_mut() {
            *pixel = *fill_color;
        }
    }

    /// Blit an image onto the canvas, scaled to fit and centred.
    ///
    /// The image is downscaled (or upscaled) with a Lanczos3 filter to fit
    /// within the canvas pixel dimensions while preserving the aspect ratio.
    /// Any letterbox or pillarbox area is filled with opaque black. The
    /// resulting pixels are written into the current buffer; call
    /// [`render`](TerminalCanvas::render) afterwards to push the changes to
    /// the terminal.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use std::io::stdout;
    ///
    /// let rect = CharRect { left: 0, top: 0, width: 80, height: 24 };
    /// let mut canvas = TerminalCanvas::new(&rect);
    ///
    /// let img = image::open("assets/colorful_finches.jpeg")
    ///     .expect("image file not found");
    /// canvas.show_image(&img);
    ///
    /// // canvas is now ready to render — call canvas.render(&mut stdout())
    /// ```
    pub fn show_image(&mut self, img: &DynamicImage) {
        let (width, height) = self.dimensions();
        let img = img.resize(width, height, image::imageops::FilterType::Lanczos3);
        let (img_width, img_height) = img.dimensions();
        let x_offset = (width as i32 - img_width as i32) / 2;
        let y_offset = (height as i32 - img_height as i32) / 2;
        self.fill(&Rgba([0, 0, 0, 255]));
        self.copy_from(&img, x_offset as u32, y_offset as u32)
            .expect("Failed to copy image to canvas");
    }

    /// Calculate the difference as a set of characters that need to be updated
    fn calc_diff(&mut self) {
        self.diff.clear();
        let pix_cols = self.rect.width as u32;
        let pix_rows = 2 * self.rect.height as u32;
        for y in (0..pix_rows).step_by(2) {
            let char_y = self.rect.top + y as u16 / 2;
            for x in 0..pix_cols {
                let char_x = self.rect.left + x as u16;
                let cur_top = self.cur_buf.get_pixel(x, y);
                let cur_bot = self.cur_buf.get_pixel(x, y + 1);
                let prev_top = self.prev_buf.get_pixel(x, y);
                let prev_bot = self.prev_buf.get_pixel(x, y + 1);
                if *cur_top != *prev_top || *cur_bot != *prev_bot {
                    self.diff.push((
                        char_x,
                        char_y,
                        VertPair {
                            top: *cur_top,
                            bot: *cur_bot,
                        },
                    ));
                }
            }
        }
    }

    /// Write all changed pixels to the terminal stream.
    ///
    /// Computes a diff between the current and previous pixel buffers, then
    /// emits only the character cells whose top/bottom pixel pair has changed.
    /// Each changed cell is rendered as a `▀` half-block character with the
    /// top pixel color as the ANSI foreground and the bottom pixel color as
    /// the ANSI background. After flushing, the previous buffer is updated to
    /// match the current buffer so that subsequent calls only redraw cells
    /// that change again.
    ///
    /// Returns immediately with `Ok(())` if nothing has changed since the last
    /// render.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any underlying write to `stream` fails, or if
    /// [`crossterm::terminal::size`] cannot query the terminal dimensions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use image::Rgba;
    /// use std::io::stdout;
    ///
    /// let rect = CharRect { left: 0, top: 2, width: 80, height: 22 };
    /// let mut canvas = TerminalCanvas::new(&rect);
    ///
    /// canvas.fill(&Rgba([0, 128, 255, 255]));
    /// canvas.render(&mut stdout()).expect("render failed");
    /// ```
    pub fn render<T>(&mut self, stream: &mut T) -> Result<(), io::Error>
    where
        T: Write + QueueableCommand,
    {
        let clip = terminal::size().ok();
        self.emit_diff(stream, clip)
    }

    /// Write all changed pixels to any output stream, without clipping to the
    /// terminal size.
    ///
    /// Behaves like [`render`](TerminalCanvas::render) except that changed
    /// cells are never clipped against the terminal dimensions and no
    /// terminal-size query is made. This is what makes the canvas usable when
    /// the output stream is not a terminal at all — for example when
    /// redirected to a file — since [`render`](TerminalCanvas::render) fails
    /// if it cannot query the terminal size.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any underlying write to `stream` fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use termcanvas::{CharRect, TerminalCanvas};
    /// use image::Rgba;
    /// use std::io::stdout;
    ///
    /// let rect = CharRect { left: 0, top: 0, width: 80, height: 10 };
    /// let mut canvas = TerminalCanvas::new(&rect);
    ///
    /// canvas.fill(&Rgba([0, 128, 255, 255]));
    ///
    /// // Works even when stdout is redirected to a file:
    /// canvas.render_unclipped(&mut stdout()).expect("render failed");
    /// ```
    pub fn render_unclipped<T>(&mut self, stream: &mut T) -> Result<(), io::Error>
    where
        T: Write + QueueableCommand,
    {
        self.emit_diff(stream, None)
    }

    /// Compute the diff and emit it as escape sequences to `stream`.
    ///
    /// When `clip` is `Some((width, height))` cells beyond those terminal
    /// dimensions are skipped; `None` emits every changed cell.
    fn emit_diff<T>(&mut self, stream: &mut T, clip: Option<(u16, u16)>) -> Result<(), io::Error>
    where
        T: Write + QueueableCommand,
    {
        self.calc_diff();
        if self.diff.is_empty() {
            return Ok(());
        }
        // queue up all the pixel changes
        for (x, y, term_char) in &self.diff {
            if clip.is_none_or(|(w, h)| x < &w && y < &h) {
                // move the cursore to character positoin
                stream.queue(cursor::MoveTo(*x, *y))?;
                let Rgba([r, g, b, _a]) = term_char.top;
                let fg = Color::Rgb { r, g, b };
                let Rgba([r, g, b, _a]) = term_char.bot;
                let bg = Color::Rgb { r, g, b };
                stream.queue(SetForegroundColor(fg))?;
                stream.queue(SetBackgroundColor(bg))?;
                write!(stream, "{VERT_HALF_BLOCK}")?;
            }
        }
        // reset color and move cursor to bottom right corner
        stream.queue(ResetColor)?;
        let x_right = self.rect.left + self.rect.width - 1;
        let y_bot = self.rect.top + self.rect.height - 1;
        stream.queue(cursor::MoveTo(x_right, y_bot))?;
        // copy over current buffer to previous buffer
        // _should_ only error when sizes don't match - which they always should
        self.prev_buf
            .copy_from(&self.cur_buf, 0, 0)
            .expect("Failed to copy current buffer to previous buffer");
        // flush the stream to render all changes
        stream.flush()
    }
}

/// `GenericImageView` and `GenericImage` are implemented so that any drawing
/// function from the `image` crate that accepts a `GenericImage` — such as
/// `image::imageops::draw_text`, overlays, or blits — can target a
/// `TerminalCanvas` directly.
///
/// **Bounds handling:** `get_pixel` and `get_pixel_mut` panic on out-of-bounds
/// coordinates (matching the contract of the `image` crate's own
/// `ImageBuffer`). `put_pixel` and `blend_pixel` silently ignore out-of-bounds
/// writes, which is safer for drawing near the edges of the canvas where the
/// terminal size may be smaller than expected.
impl GenericImageView for TerminalCanvas {
    type Pixel = Rgba<u8>;

    fn dimensions(&self) -> (u32, u32) {
        let width = self.rect.width as u32;
        let height = 2 * self.rect.height as u32;
        (width, height)
    }

    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        let (width, height) = self.dimensions();
        if x < width && y < height {
            *self.cur_buf.get_pixel(x, y)
        } else {
            panic!("Pixel coordinates out of bounds!");
        }
    }
}

impl GenericImage for TerminalCanvas {
    fn get_pixel_mut(&mut self, x: u32, y: u32) -> &mut Self::Pixel {
        let (width, height) = self.dimensions();
        if x < width && y < height {
            self.cur_buf.get_pixel_mut(x, y)
        } else {
            panic!("Pixel coordinates out of bounds!");
        }
    }

    fn put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        let (width, height) = self.dimensions();
        if x < width && y < height {
            self.cur_buf.put_pixel(x, y, pixel);
        }
    }

    fn blend_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        let (width, height) = self.dimensions();
        if x < width && y < height {
            let mut base = *self.cur_buf.get_pixel(x, y);
            base.blend(&pixel);
            self.cur_buf.put_pixel(x, y, base);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_resize_preserves_previous_buffer_contents() {
        let rect = CharRect {
            left: 0,
            top: 0,
            width: 4,
            height: 2,
        };
        let mut canvas = TerminalCanvas::new(&rect);

        // Draw distinct colors and render once (unclipped works headless) to
        // populate the previous buffer.
        canvas.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        canvas.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        canvas.put_pixel(2, 0, Rgba([0, 0, 255, 255]));
        canvas
            .render_unclipped(&mut Vec::new())
            .expect("render_unclipped into a Vec should never fail");

        // Grow the canvas; the previous buffer must keep each pixel's own
        // old value (not column 0's), or the next diff is wrong.
        canvas.resize(&CharRect {
            left: 0,
            top: 0,
            width: 6,
            height: 3,
        });
        assert_eq!(canvas.prev_buf.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(canvas.prev_buf.get_pixel(1, 0), &Rgba([0, 255, 0, 255]));
        assert_eq!(canvas.prev_buf.get_pixel(2, 0), &Rgba([0, 0, 255, 255]));
        // Undrawn pixels mirror the canvas init color after a render
        assert_eq!(canvas.prev_buf.get_pixel(1, 1), &Rgba([0, 0, 0, 255]));
    }
}
