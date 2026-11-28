# Unsupervised run summary — 2026-11-27

Goal: documentation refresh and chunk close-out, per the jef-approved
`goal.md` (recorded as `notes/goal-2026-11-27.md`). Branch `agent/docs-refresh`,
worktree `worktree/agent-2026-11-27`, created inside the container off `dev`.
Docs only — no Rust source, test, doctest, or Cargo file was touched.

## What was done

1. `4d993e3` — Rewrote `AGENTS.md` for the repo that actually exists. The
   overview is now the four-part project summary (what it is, the question it
   answers, the key idea, current status), the layout tree is generated from
   `ls`/`git ls-files`, run commands point at `-p termsnake` /
   `-p termviewer`, and the doc/test/split wording no longer says
   "`src/lib.rs`" or "examples". The decoder rule is now stated per crate:
   `termcanvas` and `termcanvas-loop` keep `image` decoders off, only
   `termviewer` enables `jpeg`/`png` because it is the component that reads
   files.
2. `d4f0920` — Replaced `notes/goal-2026-11-27.md` (a byte-identical copy of
   the 2026-10-03 goal carrying that run's approval signature) with this run's
   approved goal, and updated README's Status section: milestones 1–4 merged,
   milestone 5 next and scoped to animated GIF/APNG.
3. Third commit — seeded `changelog.md` with a 2026-11-27 entry and a
   2026-10-03 entry for milestones 1–4, cleared `todo.md` to a stub, and added
   this summary.

## What was learned

- `AGENTS.md` drifted for close to two months after the workspace conversion,
  and the container-migration commit touched the file without noticing. The
  parts a run most depends on — layout and run commands — are exactly the parts
  a refactor invalidates silently. Checking them against `ls` and `--help`
  before writing anything is cheap and caught two more errors (the
  `render_flow` description and the "cargo only, no uv" tool line).
- The "record the approved goal in `notes/`" convention created a real hazard:
  a date-renamed copy of an old goal carried the old approval signature, so the
  record of *this* run's approval was a lie. The fix here was to overwrite it;
  a future run should verify `notes/goal-<today>.md` mentions today's approval
  before trusting it.
- `todo.md` is git-ignored (`/todo.md`), so "clear `todo.md`" cannot be a
  commit, and a linked worktree starts with no `todo.md` at all. The stub was
  written into the worktree; the main checkout's session file was deliberately
  left alone (see below).
- Worktree builds reuse the main checkout's target dir via
  `CARGO_TARGET_DIR=/workspace/target`, avoiding a cold rebuild. Harmless
  (git-ignored), and worth doing in every unattended run.

## Current state

- All checks green from the workspace root: `cargo test` (8 suites, 28 tests:
  18 unit + 10 doctests), `cargo clippy`, `cargo fmt --check`,
  `cargo doc --no-deps`.
- `AGENTS.md` now describes the real tree; a grep for `examples/`,
  `cargo run --example`, and `src/lib.rs` finds only the threaded
  `crates/termsnake/src/lib.rs` path.
- Branch `agent/docs-refresh` pushed; not merged into `dev` or `main`.
- Spend: **$0.03** of the $5 cap.

## Deliberately left as-is

- `/workspace/todo.md` (main checkout) still holds its placeholder stub — it
  is git-ignored and session-scoped, and unattended work stays in the worktree.
  The cleared stub lives in the worktree; jef's next planning session rewrites
  the main-checkout file anyway.
- `/workspace/goal.md` (the approved root file) is untracked in the main
  checkout. The committed record is `notes/goal-2026-11-27.md`.
- `notes/goal-2026-10-03.md` and `notes/unsupervised-2026-10-03.md` untouched.

## Next steps

- jef reviews and merges `agent/docs-refresh`.
- Milestone 5 still needs a goal-stage decision: animated GIF/APNG only, and
  whether `termcanvas-loop` should consult `tick_interval` per tick (per-frame
  GIF delays) or termvideo should use a fixed nominal fps.
- Human terminal verification from the 2026-10-03 run is still outstanding:
  `cargo run -p termsnake`, `cargo run -p termviewer -- <image>`, piped
  termviewer output in an ANSI-capable editor, and terminal restore after a
  panic.

## For the catalog

- termcanvas: 4-crate workspace; milestones 1–4 merged (canvas, loop, viewer,
  snake); next milestone is the video player, constrained to animated images by
  the absence of ffmpeg/libav in the container.
- Process: dated goal records plus approval signatures need a freshness check;
  stale copies are indistinguishable from current ones without reading them.
- Process: a git-ignored `todo.md` means close-out steps that touch it cannot
  be committed — worktree close-out only clears the worktree copy.

— agent run, contained per AGENTS.md (devcontainer, repo-scoped PAT)
