# Unsupervised run summary — 2026-10-03

Goal: ROADMAP milestones 1–4 (workspace conversion, termviewer,
termcanvas-loop v0, termsnake), per the jef-approved `goal.md`
(recorded as `notes/goal-2026-10-03.md`). Branch
`agent/milestones-1-4`, worktree `worktree/agent-2026-10-03`, created
inside the container off `dev`.

## What was done

Eight commits, one per item:

1. `76e0c3d` — ROADMAP carry-in + approved goal.md record (initial commit).
2. `1634623` — Milestone 1: cargo workspace, all crates as siblings under
   `crates/` (virtual root, resolver 3). No behavior change.
3. `4d8e1f3` — termcanvas: new public `TerminalCanvas::render_unclipped`
   (diff render with no terminal-size query or clipping) — required for
   piped termviewer output, since `render` fails when it can't query the
   terminal size.
4. `fdbde92` — Milestone 2: `crates/termviewer` (clap CLI: `-w/--width`,
   `-h/--height`, `-c/--crop X,Y,W,H`; `-h` belongs to height per the
   agreed shape, so help is long-only). CLI utility, not TUI: one clean
   frame, no alternate screen, no key-wait. Piped runs require both size
   flags (`IsTerminal`). Sizing math is pure + unit-tested in `sizing.rs`.
5. `7a24044` — termcanvas bug fix: `resize()` copied the previous buffer
   from column 0 for every column, corrupting the diff baseline after a
   resize (found while building the loop's resize path).
6. `60fb11b` — Milestone 3: `crates/termcanvas-loop` v0 — `App` trait
   (init/handle_event/tick/draw + tick_interval/layout), `LoopCtx`,
   RAII terminal guard (raw mode + alternate screen + cursor restore on
   every exit path incl. panic), Ctrl-C as loop-level safety exit,
   resize re-layout, poll-with-deadline pacing.
7. `31bef0d` — Milestone 4: `crates/termsnake` lib + bin — `Game` pure
   model, `SnakeApp` implements `App`; binary is a thin wrapper. Shaped
   for arcade embedding per goal.md rules. `examples/` fully removed.
8. `1f0557d` — Root `README.md` (Cargo.toml referenced it; now exists)
   and ROADMAP status updates.

## What was learned

- `TerminalCanvas::render` hard-depends on a queryable terminal, which
  blocks any non-tty sink; the unclipped variant now covers that, and
  the split (clip vs no-clip) is a clean public seam.
- The termcanvas `resize()` prev-buffer bug only shows under resize —
  exactly the path the new loop exercises. Fixed + unit-tested.
- Exact `assert_eq!` on `Instant`-based timeouts is flaky (the
  `elapsed()` query itself takes ~100ns); timing assertions must be
  bounded, not exact. Also: lower bounds on sleep-based assertions
  overshoot under scheduler jitter.
- clap v4: assigning `-h` to a real flag requires
  `disable_help_flag = true` + a long-only help arg (done — help is
  `--help`).
- A native-size image with odd pixel height must still get an even
  canvas (one half-row of canvas black at the bottom) — pinned by test.

## Current state

- Branch pushed: `agent/milestones-1-4` (not merged into dev/main).
- `cargo test`: all green — 9 inline unit tests + 9 doctests across
  termcanvas (1 unit + 8 doc), termcanvas-loop (1 unit + 1 doc),
  termviewer (8 unit), termsnake (7 unit). `cargo clippy` and
  `cargo fmt --check` clean; `cargo doc --no-deps` builds.
- Spend: **$0.23** of the $15 cap (`uv run ~/tools/spend.py --cap 15`).
- wiki-worthy learnings: none proposed during the run (per rules); the
  nearest candidates would be "Instant-based timeout equality is
  flaky" and "render vs render-unclipped as a tty-boundary seam" —
  noting only, not proposing.

## Needs a real-terminal check (cannot be covered by cargo test)

- `cargo run -p termsnake` — alternate screen lifecycle, arrow input,
  tick pacing, Ctrl-C exit, resize re-layout, game-over screen.
- `cargo run -p termviewer -- crates/termcanvas/assets/colorful_finches.jpeg`
  — tty path fit/scale and the reserved last line.
- Piped termviewer output viewed in an ANSI-capable editor
  (`termviewer pic.png -w 80 -h 40 > /tmp/img.txt`).
- Terminal restore after a panic (hard to trigger; guard logic is
  code-reviewed only).

## Next steps

- jef reviews/merges the branch.
- Interactive verification list above.
- Milestone 5 (video player) will stretch the loop's fps pacing;
  the `-a/--aspect` termviewer question is deferred as agreed.
- If a doctest contract needs touching during future refactors, it's a
  public-contract change and needs explicit sign-off.

— agent run, contained per AGENTS.md (devcontainer, repo-scoped PAT)
