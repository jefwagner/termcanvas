# Roadmap

termcanvas is a family of terminal apps built on shared library crates:
a pixel-canvas core, a terminal render/game loop, and the apps that
consume them.

## Crates

| Crate | Purpose | Status |
|---|---|---|
| `termcanvas` | Canvas over terminal regions: half-block pixels, double-buffered diff rendering, `image` integration | exists (0.1.0) |
| `termcanvas-loop` | Terminal lifecycle + render/game loop: raw mode, event handling, frame pacing, redraw discipline | planned — distill from `examples/snake.rs` |
| `apps/termviewer` | CLI picture viewer | planned |
| `apps/termvideo` | Video player (alternate screen) | planned |
| `apps/termsaver` | Full-terminal screensaver | planned |
| `apps/termarcade` | Full-terminal game menu shell | planned |
| games | snake, tetris, breakout, ... | open question: standalone bins or crates consumed by the arcade |

## Milestones (in order)

1. **Workspace conversion.** Restructure as a cargo workspace with
   `termcanvas` as the core crate. No behavior change; examples still
   run; tests still green.
2. **Picture CLI.** First real app binary: promote
   `examples/picture_viewer.rs` with argument handling. Proves the
   app-crate pattern; needs no loop crate.
3. **`termcanvas-loop` v0.** Distill the loop (terminal setup, input,
   pacing) out of `examples/snake.rs`; snake becomes its first consumer.
   Extract from working code, don't design from imagination.
4. **Snake game TUI app** First consumer of loop. Promote `examples/snake.rs` 
   as full in-terminal TUI, alternate screen, resize handling, input handling.
5. **Video player TUI app.** Stretches the loop: fps pacing, alternate-screen
   lifecycle, large-frame performance.
6. **Screensaver.** Full-terminal, idle-driven variant of the loop.
7. **Arcade + games.** Menu shell hosting games; revisit the
   games-as-crates question here.

## Open questions

- Do games ship as separate binaries or crates consumed by the arcade?
  (Decide when two games exist.)
- How much game scaffolding (input mapping, scoring, grid model) belongs
  in `termcanvas-loop` vs. per-app?
- Does the loop crate own the alternate-screen/raw-mode lifecycle, or do
  apps?

## Notes

- `README.md` is referenced in `Cargo.toml` (`readme = "README.md"`) but
  doesn't exist yet — needed before any `cargo publish`.
