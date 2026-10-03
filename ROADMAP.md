# Roadmap

termcanvas is a family of terminal apps built on shared library crates:
a pixel-canvas core, a terminal render/game loop, and the apps that
consume them.

## Crates

| Crate | Purpose | Status |
|---|---|---|
| `termcanvas` | Canvas over terminal regions: half-block pixels, double-buffered diff rendering, `image` integration | exists (0.1.0) |
| `termcanvas-loop` | Terminal lifecycle + render/game loop: raw mode, alternate screen, event dispatch, frame pacing, cleanup on every exit path | exists (v0) — distilled from the old `examples/snake.rs` |
| `crates/termviewer` | CLI picture viewer (one frame to stdout; pipeable) | exists (0.1.0) |
| `crates/termsnake` | Snake game, lib + bin (lib is embeddable by the arcade) | exists (0.1.0) |
| `crates/termvideo` | Video player (alternate screen) | planned |
| `crates/termsaver` | Full-terminal screensaver | planned |
| `crates/termarcade` | Full-terminal game menu shell | planned |
| games | tetris, breakout, ... | open question: standalone bins or crates consumed by the arcade (termsnake already ships lib + bin, so the arcade can consume its lib) |

## Milestones (in order)

1. **Workspace conversion.** ✅ Done — virtual workspace, all crates as
   siblings under `crates/`; no behavior change.
2. **Picture CLI.** ✅ Done — `crates/termviewer`, clap CLI, pipeable to a
   file; needs no loop crate.
3. **`termcanvas-loop` v0.** ✅ Done — distilled from the old
   `examples/snake.rs`; loop owns the terminal lifecycle, apps implement
   the `App` trait.
4. **Snake game TUI app.** ✅ Done — `crates/termsnake` (lib + bin) on the
   alternate screen via the loop; shaped for arcade embedding.
5. **Video player TUI app.**
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
  apps? **Decided:** the loop owns it (RAII restore on every exit path); apps
  draw only into the loop-provided rect.

## Notes

- `examples/` is gone — its two members were promoted into `termviewer`
  and `termsnake`. Interactive verification now happens by running the
  apps themselves.

## Layout (current)

```
Cargo.toml              # virtual workspace root
crates/termcanvas/
crates/termcanvas-loop/
crates/termviewer/
crates/termsnake/
```
