// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! termviewer — display an image file in the terminal as half-block pixels.
//!
//! A CLI utility, not a TUI app: it writes exactly one self-contained frame
//! and exits. It works both written to a terminal and piped to a file
//! (`termviewer pic.png > image.txt`), where the file can be viewed as an
//! image by any editor that renders ANSI escape sequences.

mod sizing;

use clap::Parser;
use crossterm::terminal;
use image::GenericImage;
use image::imageops::FilterType;
use std::io::{IsTerminal, Write, stdout};
use std::path::PathBuf;
use termcanvas::{CharRect, TerminalCanvas};

const AFTER_HELP: &str = "\
Examples:
  termviewer pic.png
  termviewer -w 100 -h 60 -c 0,0,200,100 pic.jpg
  termviewer -w 80 -h 40 pic.png > image.txt

Piped output renders as an image only in a text editor that supports
ANSI escape sequences; a plain editor shows the raw escape codes.";

/// Display an image file in the terminal as unicode half-block pixels.
#[derive(Debug, Parser)]
#[command(
    name = "termviewer",
    version,
    about = "Display an image in the terminal as half-block pixels",
    after_help = AFTER_HELP,
    // -h belongs to --height, so help is long-only
    disable_help_flag = true
)]
struct Cli {
    /// The image file to display
    pic_file: PathBuf,
    /// Target width in pixels (1 px = 1 character column)
    #[arg(short, long, value_name = "PX")]
    width: Option<u32>,
    /// Target height in pixels (1 px = half a character row; odd values are
    /// bumped up to the next even pixel)
    #[arg(short = 'h', long, value_name = "PX")]
    height: Option<u32>,
    /// Crop window X,Y,W,H in unscaled source-image pixels, clamped to the
    /// image bounds. Scaling happens after cropping, never before.
    #[arg(short, long, value_name = "X,Y,W,H")]
    crop: Option<String>,
    /// Print help
    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("termviewer: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    // Pipeline order is fixed: load → crop → scale-to-box.
    let img = image::open(&cli.pic_file)
        .map_err(|e| format!("cannot open {}: {e}", cli.pic_file.display()))?
        .to_rgba8();

    let img = match &cli.crop {
        None => img,
        Some(spec) => {
            let (x, y, w, h) = parse_crop(spec)?;
            let (iw, ih) = img.dimensions();
            let (x, y, w, h) = sizing::clamp_crop((x, y, w, h), (iw, ih))
                .ok_or_else(|| format!("crop window {spec} is empty for a {iw}x{ih} image"))?;
            image::imageops::crop_imm(&img, x, y, w, h).to_image()
        }
    };

    // Target box: the terminal size (minus the reserved last line) when
    // stdout is a terminal; both size flags are required otherwise.
    let tty = std::io::stdout().is_terminal();
    let box_px = if tty {
        let (cols, rows) = terminal::size().map_err(|e| format!("{e}"))?;
        if rows < 2 {
            return Err("terminal is too small (needs at least 2 rows)".into());
        }
        (u32::from(cols).max(1), 2 * u32::from(rows - 1))
    } else {
        let w = cli
            .width
            .ok_or("stdout is not a terminal: --width is required")?;
        let h = cli
            .height
            .ok_or("stdout is not a terminal: --height is required")?;
        (w.max(1), h.max(1))
    };

    let explicit = match (cli.width, cli.height) {
        (Some(w), Some(h)) => Some((w.max(1), h.max(1))),
        _ => None,
    };
    let allow_upscale = cli.width.is_some() || cli.height.is_some();

    let (canvas_px, image_px) =
        sizing::compute_display_size(img.dimensions(), box_px, explicit, allow_upscale);
    if canvas_px.0 > u32::from(u16::MAX) {
        return Err(format!(
            "canvas width {} exceeds the maximum of {}",
            canvas_px.0,
            u16::MAX
        ));
    }

    let scaled = image::imageops::resize(&img, image_px.0, image_px.1, FilterType::Lanczos3);

    let rect = CharRect {
        left: 0,
        top: 0,
        width: canvas_px.0 as u16,
        height: (canvas_px.1 / 2) as u16,
    };
    let mut canvas = TerminalCanvas::new(&rect);
    canvas
        .copy_from(&scaled, 0, 0)
        .map_err(|e| format!("{e}"))?;

    // Output: one frame written as a linear flow of rows, starting at the
    // stream's current position — below the command on a tty, and a plain
    // linear stream in a piped file. No Clear(All), no cursor addressing,
    // no key-wait (that would hang piped runs).
    let mut out = stdout().lock();
    writeln!(out).map_err(io_err)?;
    canvas.render_flow(&mut out).map_err(io_err)?;
    Ok(())
}

fn parse_crop(spec: &str) -> Result<(u32, u32, u32, u32), String> {
    let invalid = || format!("invalid crop '{spec}', expected X,Y,W,H");
    let parts: Vec<&str> = spec.split(',').collect();
    if parts.len() != 4 {
        return Err(invalid());
    }
    let vals: Vec<u32> = parts
        .iter()
        .map(|p| p.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| invalid())?;
    Ok((vals[0], vals[1], vals[2], vals[3]))
}

fn io_err(e: std::io::Error) -> String {
    format!("io error: {e}")
}
