# Changelog

Chunk-level record of what landed and why. `git log` has the file-level detail.

## 2026-11-27 — Documentation refresh and chunk close-out

- Rewrote `AGENTS.md` for the post-workspace repo: the four-part project
  summary, a layout tree that matches the workspace, app-crate run commands,
  and the decoder rule stated per crate (`termviewer` only).
- Replaced `notes/goal-2026-11-27.md`, which held a copy of the October goal
  under the October approval, with this run's approved goal.
- Seeded this changelog, cleared `todo.md`, and pointed README's status at
  milestone 5.

## 2026-10-03 — Milestones 1–4: workspace, termviewer, loop, termsnake

- Converted the single-crate repo into a virtual workspace with all crates as
  siblings under `crates/`, so libraries and apps share one lockfile and
  toolchain with no behavior change.
- Promoted the picture viewer to `termviewer`: a pipeable CLI with fit-to-box
  sizing and cropping. `TerminalCanvas::render_unclipped` exists because diff
  rendering otherwise requires a queryable terminal.
- Distilled the terminal lifecycle and render loop into `termcanvas-loop` v0:
  an `App` trait, loop-owned raw mode/alternate screen restored on every exit
  path, event dispatch, tick pacing, and resize re-layout.
- Promoted snake to `termsnake` as lib + bin, so a future arcade shell can
  embed the game without the binary.
- Fixed `TerminalCanvas::resize` copying the previous buffer from column 0
  for every column, which corrupted the diff baseline after a resize.
- Wrote the root `README.md` that `termcanvas`'s `Cargo.toml` already
  referenced.
