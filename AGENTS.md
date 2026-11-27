# AGENTS.md

Guidance for agentic coding assistants (AI helpers, automated tools) working in this
repository. Read this before making changes.

---

## Project summary

`termcanvas` is a family of terminal apps built on shared Rust library crates:
a pixel canvas over terminal regions, a terminal lifecycle/render loop, and the
apps that consume them — a CLI picture viewer and a snake game.

The question it answers: can a small set of library crates make terminal apps
look graphical — real pixels, real colors — while keeping terminal ergonomics?
Output must pipe cleanly to a file, raw-mode/alternate-screen lifecycle is
handled exactly once, and apps are shaped so a future arcade shell can host
them without a rewrite.

The key idea is the character cell. The Unicode UPPER HALF BLOCK (`▀`) packs
two vertically stacked pixels into one cell, so a `W × H` terminal region is a
`W × 2H` pixel canvas. Rendering is double-buffered and diffed, so output
volume follows what changed, not screen size. All global terminal state belongs
to `termcanvas-loop`; an app draws only into the `CharRect` the loop gives it.

**Status:** milestones 1–4 are done and merged into `dev` — workspace
conversion, `termviewer`, `termcanvas-loop` v0, and `termsnake`. Next is
ROADMAP milestone 5, the video player, scoped to animated GIF/APNG because the
container has no `ffmpeg`/libav. Rendering behavior still needs a human
terminal run to verify; `cargo test` covers buffers, diffing, and cell encoding
only.

Key design points, in the order they matter:

1. **Double-buffered diff rendering.** Two in-memory pixel buffers (current and
   previous); `TerminalCanvas::render` writes only the character cells that
   changed since the last render, keeping output minimal and flicker-free.
   `render_unclipped` skips the terminal-size query (so piped output works),
   and `render_flow` writes a full frame with no cursor addressing.
2. **`image`-crate integration.** `TerminalCanvas` implements
   `image::GenericImage` / `GenericImageView`, so the whole `image` ecosystem
   (blitting, resizing, compositing) can paint directly onto the canvas.
3. **The loop owns the terminal.** `termcanvas-loop` exposes `App`, `LoopCtx`,
   `Flow`, and `run`: raw mode, alternate screen, cursor hiding, event
   dispatch, tick pacing, and restoration on every exit path. Apps implement
   `App` and never touch global terminal state.

---

## Modes of work

The modes and their rules come from the global instructions
(`~/.pi/agent/AGENTS.md`) and the skills: `/skill:pair`, `/skill:tdd-plan`
then `/skill:tdd-implement`, `/skill:unsupervised-goal` then
`/skill:unsupervised-run`. The default is **pair**. All three modes are
allowed in this repo; switching is jef's call.

What this repo adds:

- **Read-only checks** pair mode may run freely: `cargo check`, `cargo test`,
  `cargo clippy`, `cargo fmt --check`, `cargo doc --no-deps`, `cargo build`.
- **Unsupervised base branch:** `dev`. The end-of-run test suite is
  `cargo test` and `cargo clippy`.
- The **(a)/(b) test split** below applies in every mode and is the most
  important project-specific rule here. When unsure, check the tier before
  rewriting a test.

### Project-specific workflow deltas

- **The (a)/(b) test split.** Tests come in two classes by intent:
  - **(a) Unit tests for *internal* interfaces.** Inline in Rust
    (`#[cfg(test)] mod test { ... }` at the bottom of the source file). They pin
    a single module's internals. **These MAY be updated or changed during a
    refactor** — they are part of the implementation, not a contract.
  - **(b) Tests for the *public* API.** These are the doctests on public items
    in every crate and any integration tests under `tests/` (when added). They pin
    the public contract. **These MUST NOT be updated or changed during a
    refactor** — a refactor moves internals around while keeping these green. If
    a refactor genuinely needs to change one of these, that is a signal the
    public contract is changing and must be called out explicitly, not silently
    edited.

  The point: a refactor is *allowed* to rewrite every unit test and *forbidden*
  from rewriting the public-API tests. The doctests are the safety net — they
  also render into the published docs, so they are both contract and
  documentation at once.

- **Rendering is not unit-testable end to end.** The real render output goes to a
  terminal; `cargo test` can only cover buffer arithmetic, diffing, and cell
  encoding. Anything touching actual terminal behavior must be verified through
  an app run (`cargo run -p termsnake`, `cargo run -p termviewer -- <image>`)
  — say so when a change needs that kind of verification, rather than claiming
  tests cover it.

---

## Planning and progress files

| File | Horizon | Granularity | Churn |
|------|---------|-------------|-------|
| `todo.md` | current session | commit-sized actionable items (1–4) | every planning session |
| `backlog.md` | persistent | raw ideas, mid-session discoveries | triaged in planning mode; created when first needed |
| `changelog.md` | persistent | dated chunk summaries | one entry per chunk closed (`close-chunk`) |

`ROADMAP.md` holds the long-term shape (library crates and planned apps,
milestone order); update it when milestones land or the plan changes.

`todo.md` is session-scoped: rewritten at each planning session, cleared at the
end of a work chunk.

---

## Repository Layout

```
repo root/
├── Cargo.toml               # virtual workspace root (members = ["crates/*"],
│                            #   resolver 3); no root package
├── Cargo.lock
├── LICENSE                  # MIT
├── README.md                # project overview + usage
├── ROADMAP.md               # crate and milestone plan
├── changelog.md             # dated, chunk-level record of what landed
├── AGENTS.md                # this file
├── crates/
│   ├── termcanvas/          # the canvas library: TerminalCanvas, CharRect,
│   │   ├── src/             #   pixel buffers, diff renderer, unit tests
│   │   └── assets/          #   example media (colorful_finches.jpeg)
│   ├── termcanvas-loop/     # terminal lifecycle + render/game loop
│   ├── termviewer/          # CLI picture viewer (bin only)
│   └── termsnake/           # snake game (lib + bin)
├── notes/                   # working scratch: brainstorms, plans, drafts,
│                            #   unsupervised goals and run summaries
├── worktree/                # git-ignored; worktrees for unattended runs
└── todo.md                  # git-ignored; session-scoped plan
```

---

## Build, Test, and Tooling Commands

Tools: **`cargo`** only for building — no Python, no maturin. Stable Rust,
edition 2024, no `rust-toolchain.toml`. (`uv` appears only in
`uv run ~/tools/spend.py`, for spend accounting.)

```bash
cargo check                       # fast type-check, no linking
cargo test                        # unit tests + doctests (no terminal needed)
cargo clippy                      # lint
cargo fmt                         # format before committing (rustfmt defaults)

cargo doc --no-deps               # build the API docs (doctests are the examples)

cargo run -p termsnake            # interactive; needs a real terminal
cargo run -p termviewer -- <image_path>   # one frame; pipes cleanly
```

### Key gotchas

- **Unit tests run headless.** `cargo test` covers buffers, diffing, and cell
  encoding only. It cannot catch a broken escape-sequence stream — that needs an
  app run in a real terminal (`cargo run -p termsnake`, or
  `cargo run -p termviewer -- <image>`).
- **`image` decoders are per-crate.** `termcanvas` and `termcanvas-loop` keep
  `image` at default-features = false. Only `termviewer` enables `jpeg`/`png`,
  because it is the component that reads image files. Don't add decoders to the
  library crates.
- **The app crates are the executable documentation.** A new public-API feature
  should come with a doctest, or (for terminal-shaped behavior doctests cannot
  check) an example in the app crates.

---

## Commit Message Style

Two parts:

1. **Subject line** — a short, general description that reads well from
   `git log --oneline`. Imperative mood preferred. No required prefix.
2. **Body** — a longer message detailing the changes: what was added/changed and
   *why*, notable design decisions, and (for tests) what the new tests cover.
   Wrap at a readable width.

Lasting design decisions go in the crate docs (if they change the public API or
usage) or the commit body (if they're local to the task).

---

## Code Style

### Rust

- Edition **2024**. Run `cargo fmt` before committing (rustfmt defaults; no
  `rustfmt.toml`).
- `///` doc-comments on all public items, with doctests where the behavior is
  demonstrable. The doc-comments are the single source of truth for
  documentation — `cargo doc` is the docs site, there is no other layer.
- PascalCase for types, snake_case for functions/methods, SCREAMING_SNAKE_CASE
  for constants. Color constants in the app crates follow the
  `COLOR_<THING>` pattern (see `crates/termsnake/src/lib.rs`).
- Section banners (`// ── Section ──...──`) are used in the app crates; keep
  them consistent when adding code there.
- Keep trait bounds minimal — only what the body uses.
- Inline `#[cfg(test)] mod test { use super::*; ... }` for unit tests; name each
  test `test_<what_is_being_tested>`.

---

## Containment

**Work inside the devcontainer.** Open this repo in a devcontainer (VS Code
"Reopen in Container", or `devcontainer up`) and run agent sessions there.
Built and verified: no `ssh` binary, `~/wiki/` reads, and git authentication
all confirmed inside the container before first use.

The properties below are to be enforced by the container, not promised by this
file — a session on the host has the personal SSH key and defeats all of it. If
a session is somehow running outside the container, say so before doing anything
else.

### What the container enforces

- **The personal SSH key is absent**, and no `ssh`, `scp`, `sftp`,
  `ssh-agent`, or `rsync` binary exists. Git has no SSH transport at all.
- **One credential exists**: the repo-scoped PAT, mounted read-only. The
  `jefscad` and `pixel-world` PATs and the lab bot credential are neither
  mounted nor reachable.
- **No `~/.aws`, `~/.azure`, `~/.config/lab-bot`, or host tool directory.**
- **`~/wiki/` is mounted read-only** — full read access to `kb/` and the
  projects pages, with no ability to write.
- **The instructions, skills and `~/tools/spend.py` are read-only**, so the
  rules this session operates under cannot be edited mid-session.
- **No mail credentials.** There is no `msmtp` and no `~/.msmtprc`. The one
  writable path outside the workspace is `~/outbox`, where `notify-jef` drops
  a message; agentd on the host validates it, rate-limits it and mails it to
  jef. The container cannot choose the recipient.
- `--cap-drop=ALL` and `--security-opt=no-new-privileges:true`.

Consequence: **the worst realistic outcome of a bad unattended run is commits
and branches inside this repository, plus one notification to jef.** Nothing else on the
machine is writable, and nothing else is reachable.

### Credentials

Agent sessions push as `jefwagner` with a fine-grained PAT scoped to this repo
only. Setup and the full permission list are in
the `pat-setup` skill (on the host); the token lives at
`~/.config/jef/termcanvas-pat` (mode 600, outside the repo), bind-mounted in at
the same relative path.

`git-askpass.sh` (tracked, repo root) reads it at call time; `devcontainer.json`
sets `GIT_ASKPASS`, `GIT_TERMINAL_PROMPT=0`, and `GIT_SSH_COMMAND=/bin/false`.
The remote is already HTTPS (`https://github.com/jefwagner/termcanvas.git`), so
the token is genuinely used.

If a git command prompts interactively, something is misconfigured: stop and
say so rather than working around it.

### Branches

- **Git-flow: `dev` is active, `main` holds releases.** Both reject
  force-pushes (GitHub branch protection) — that binds jef too. Solo dev: the
  only branches beyond these are the `agent/*` work branches below.
- Work goes on `agent/<short-desc>` branches. Never commit to `main` or `dev`
  directly from an agent session.
- Unattended work always runs in a worktree (the `unsupervised-run` skill).

### Residual risk — the honest remainder

- **Spend.** `spend.py` reports and exits non-zero; nothing interrupts. The cap
  is enforced by treating that exit as a stop signal.
- **Damage within this repo.** `reset --hard`, `rm`, and a force-push to a
  non-protected branch are all still possible here. Hence worktrees and branch
  discipline.
- **The agent's judgement.** A container cannot tell a good idea from a
  plausible wrong one. The (a)/(b) test contract is a prose contract: nothing
  mechanically stops an agent from rewriting a doctest to make a refactor pass.
  The rule in "Project-specific workflow deltas" is the only thing holding that
  line, so treat it as non-negotiable.

## Wiki

`~/wiki/` is **mounted read-only**. Read it freely: `~/wiki/kb/` for the
distilled fundamentals (terminal escape sequences, color models,
character-cell rendering), and other projects' pages when a decision has
cross-project implications.

**Never write to `~/wiki/`, and don't write wiki proposals.** The wiki is
becoming a project catalog that the orchestrator maintains. Things worth
keeping go in `notes/` (the `close-chunk` skill); unsupervised runs put them
in the summary's "For the catalog" section.

## Things to Avoid

- Do not run cargo with `+nightly` — `termcanvas` is stable-Rust (edition 2024).
- Do not rewrite doctests (public-API tests) during a refactor. If a refactor
  seems to require it, that is a contract change: call it out explicitly. Inline
  `mod test` unit tests are fair game to rewrite.
- Do not add `unwrap()` in library code; use `expect("reason")` or return
  `Option`/`Result`.
- Do not enable `image` format decoders in `termcanvas` or `termcanvas-loop`;
  only `termviewer` (the file-reading app) enables `jpeg`/`png`.
- Do not claim a rendering change is verified by `cargo test` alone — say which
  app run (`cargo run -p termsnake`, `cargo run -p termviewer -- <image>`) is
  needed to see it.
- Do not write to `~/wiki/`.
