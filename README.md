# termcanvas

A family of terminal apps built on shared library crates: a pixel-canvas
core, a terminal render/game loop, and the apps that consume them.

## Crates

All crates are direct siblings under `crates/` — an app is simply a crate
whose product is a binary.

| Crate | Purpose |
|---|---|
| `termcanvas` | Canvas over terminal regions: unicode half-block pixels, double-buffered diff rendering, `image`-crate integration |
| `termcanvas-loop` | Terminal lifecycle + render/game loop: raw mode, alternate screen, event dispatch, tick pacing, cleanup on every exit path |
| `termviewer` | CLI picture viewer — writes one self-contained frame to stdout |
| `termsnake` | Classic snake game (library + binary) |

## What termcanvas renders

Pixels are drawn with the unicode UPPER HALF BLOCK character (`▀`) and ANSI
24-bit foreground/background colors, so each character cell carries two
vertically-stacked pixels: a `W × H` terminal region becomes a `W × 2H`
pixel canvas. Rendering is double-buffered — only cells that changed since
the last frame are written.

**Terminal requirements:** a modern terminal emulator with unicode support,
24-bit (truecolor) color, and — for the interactive apps — raw-mode
capability (kitty, iTerm2, Alacritty, WezTerm, Windows Terminal, and recent
GNOME Terminal / macOS Terminal.app all work).

## Usage

```bash
cargo run -p termviewer -- <image_file>          # view a picture
cargo run -p termsnake                            # play snake

# Piping a frame to a file:
termviewer -w 80 -h 40 pic.png > image.txt
```

`termviewer` piped output renders as an image only in a text editor that
supports ANSI escape sequences (truecolor SGR, plus the `▀` character as a
full-width glyph). A plain editor shows the raw escape codes.

Build and test from the workspace root:

```bash
cargo test        # unit tests + doctests (headless)
cargo clippy
cargo doc --no-deps   # API docs; the doctests are the examples
```

## Status

See [ROADMAP.md](ROADMAP.md) for the long-term shape — video player,
screensaver, and an arcade shell hosting games are planned.
