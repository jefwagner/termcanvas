# AGENTS.md

Guidance for agentic coding assistants (AI helpers, automated tools) working in this
repository. Read this before making changes.

---

## Project Overview

`termcanvas` is an early-stage personal project: a Rust library providing a
canvas-like pixel-graphics API over terminal regions. It renders pixels using the
Unicode UPPER HALF BLOCK character (`▀`) with ANSI 24-bit foreground/background
colors, so each character cell carries two vertically-stacked pixels — a `W × H`
terminal region becomes a `W × 2H` pixel canvas.

Key design points:

1. **Double-buffered diff rendering.** Two in-memory pixel buffers (current and
   previous); `TerminalCanvas::render` writes only the character cells that
   changed since the last render, keeping output minimal and flicker-free.
2. **`image`-crate integration.** `TerminalCanvas` implements
   `image::GenericImage` / `GenericImageView`, so the whole `image` ecosystem
   (blitting, resizing, compositing) can paint directly onto the canvas. The
   `image` dependency uses default-features = false; format decoders are
   dev-only (for examples).
3. **crossterm for terminal control.** The library does the drawing; the
   application owns raw mode, cursor visibility, and event handling (see the
   crate-level docs for the required terminal setup).

The library is the deliverable; `examples/` (a snake game, a picture viewer) are
the living documentation and test harness for real terminal behavior.

---

## Modes of work

Mode switches are explicit — say "pair", "tdd", or "unsupervised" to change modes.
The default, when no mode is stated, is **pair**.

Three nested loops, each with a different unit of approval:

| Mode           | Unit of work              | Writes need approval? |
|----------------|---------------------------|-----------------------|
| `pair`         | single command/change     | yes, per command      |
| `tdd`          | task / commit-sized item  | no                    |
| `unsupervised` | feature / whole goal      | no                    |

The nesting widens the unit of work that may be done unattended: a single command
in pair, a task in tdd, a whole feature or goal in unsupervised. Stepping up is
jef's call, not mine — if a task needs many writes, say so and suggest `tdd`
rather than quietly widening the loop.

### Pair mode (default)

Pair-programming at the keyboard. Short back-and-forth, one or two ideas per
interaction — no long essays presenting a pile of options. Every idea and change
gets discussed as we go: a sentence or two saying what I'm about to do and why,
then act. No autopilot — no multi-file refactors or multi-step plans executed
without checking in between steps.

Freely, without asking:

- read files, search the repo (`rg`, `find`, `grep`), read `notes/`
- non-destructive inspection: `ls`, `cat`, `head`, `tail`, `wc`, `git status`,
  `git log`, `git diff`
- the project's read-only checks: `cargo check`, `cargo test`, `cargo clippy`,
  `cargo fmt --check`, `cargo doc --no-deps`, `cargo build`

Ask for explicit approval, per command, before:

- writing, editing, creating, moving, or deleting any file — including `notes/`,
  and anything outside this repo
- destructive or state-changing commands: `rm`, `mv`, `cp` over an existing
  file, `git commit`/`push`/`checkout`/`reset`, or anything else that changes
  state on disk
- installing packages, editing config outside the repo, sending mail

Approval is per command, not blanket — "yes, go ahead" for one command does not
authorize the next. One approval covers one file: a multi-hunk edit within a
single file is one atomic change, but two files is two approvals. Never commit
unless asked in that same turn, even if every individual write was approved.

Full rules: read `~/tools/llm-instructions/pair.md`.

### TDD mode (interactive)

Structured mode for larger chunks of work. Two sub-modes:

- **Plan** — read `~/tools/llm-instructions/planning.md` first. Produces 1–4
  one-commit-sized items in `todo.md` and triages `backlog.md`.
- **Implement** — read `~/tools/llm-instructions/implementation.md` first. TDD
  loop with hard STOP gates requiring user approval at the task level.

Writes, edits, and commits are **not** gated per command here — the task-level
STOP gates are the approval unit. Commits remain one-item-per-commit, and I still
state intent before each task, but I don't ask before every `edit`.

The **(a)/(b) test split** below applies and is the most important
project-specific rule here. When unsure, check the tier before rewriting a test.

### Unsupervised mode

For well-scoped work that jef approves up front and then leaves to run
independently. See **Containment** for what the environment does and does not buy.

1. **Goal stage (conversational)**: work with jef to draft `goal.md` at the repo
   root. It must contain: the goal, constraints, a definition of done, and a
   **spend cap in dollars** agreed during the conversation. Write `goal.md` as
   a complete prompt — an agent with no other context should be able to do the
   work from it alone.
2. **Approval gate**: do not start work until jef explicitly approves `goal.md`.
   Revise and re-submit until approved.
3. **Isolate**: create a worktree before anything else —
   `git worktree add ../termcanvas-loop -b agent/<date> main`, and work in
   `../termcanvas-loop`. Never work unattended in the main checkout. This is
   what keeps `~/projects/termcanvas` itself untouched.
4. **Independent work**: proceed without further check-ins. Track spend with:
   `uv run ~/tools/spend.py --cap <cap from goal.md>`
   Check periodically. It **reports and exits non-zero**; it does not interrupt,
   so treating that exit code as a stop signal *is* the enforcement. The cap is
   an upper bound on money, not a bound on time — `goal.md` must be small enough
   to finish.
5. **Stop and report**: stop when the goal is met, the cap is exceeded, or the
   work is blocked. Then:

   - run the full test suite (`cargo test`, `cargo clippy`) and record the result;
   - push the branch — **never merge it, never push to `main`**;
   - write a summary email to **jefwagner@gmail.com only** with what was done,
     what was learned, current state, next steps, test results, and the final
     spend from `uv run ~/tools/spend.py`;
   - send it with `msmtp -t < summary-email.txt`, subject prefixed `[termcanvas]`
     so it is filterable;
   - **also commit the same summary into the branch** as
     `notes/unsupervised-<date>.md`, because email is the notification and the
     file is the durable record;
   - note in the summary what would have been proposed to `~/wiki/` — do not
     propose it. An unmonitored agent does not touch the wiki at all.

**One email per run, to that one address.** The email is the signal that pulls
jef in to review. If `msmtp` fails, say so in the branch summary rather than
retrying in a loop — a silent email means jef does not know work is waiting.

### Project-specific workflow deltas

- **The (a)/(b) test split.** Tests come in two classes by intent:
  - **(a) Unit tests for *internal* interfaces.** Inline in Rust
    (`#[cfg(test)] mod test { ... }` at the bottom of the source file). They pin
    a single module's internals. **These MAY be updated or changed during a
    refactor** — they are part of the implementation, not a contract.
  - **(b) Tests for the *public* API.** These are the doctests on public items in
    `src/lib.rs` and any integration tests under `tests/` (when added). They pin
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
  an example run (`cargo run --example snake`) — say so when a change needs that
  kind of verification, rather than claiming tests cover it.

---

## Planning and progress files

| File | Horizon | Granularity | Churn |
|------|---------|-------------|-------|
| `todo.md` | current session | commit-sized actionable items (1–4) | every planning session |
| `backlog.md` | persistent | raw ideas, mid-session discoveries | triaged in planning mode |

(No `ROADMAP.md` yet — the crate is small enough that `todo.md` suffices. Add a
roadmap when the feature list outgrows one page.)

`todo.md` is session-scoped: rewritten at each planning session, cleared at the
end of a work chunk.

---

## Repository Layout

```
repo root/
├── Cargo.toml               # the crate: termcanvas, edition 2024
├── Cargo.lock
├── LICENSE                  # MIT
├── src/
│   └── lib.rs               # the whole library: TerminalCanvas, CharRect,
│                            #   pixel buffers, diff renderer, unit tests
├── examples/
│   ├── snake.rs             # snake game — interactive correctness harness
│   └── picture_viewer.rs    # displays an image file on the canvas
├── assets/                  # example media (colorful_finches.jpeg)
├── notes/                   # working scratch: brainstorms, plans, drafts;
│   └── wiki-proposals/      #   dated proposals for ~/wiki/ (see Wiki section)
├── README.md                # (planned — referenced by Cargo.toml, not yet written)
└── AGENTS.md                # this file
```

---

## Build, Test, and Tooling Commands

Tools: **`cargo`** only — no Python, no uv, no maturin. Stable Rust, edition 2024,
no `rust-toolchain.toml`.

```bash
cargo check                       # fast type-check, no linking
cargo test                        # unit tests + doctests (no terminal needed)
cargo clippy                      # lint
cargo fmt                         # format before committing (rustfmt defaults)

cargo doc --no-deps               # build the API docs (doctests are the examples)

cargo run --example snake         # interactive; needs a real terminal
cargo run --example picture_viewer -- <image_path>
```

### Key gotchas

- **Unit tests run headless.** `cargo test` covers buffers, diffing, and cell
  encoding only. It cannot catch a broken escape-sequence stream — that needs an
  example run in a real terminal.
- **`image` has default features off** in `[dependencies]`; the examples
  re-enable format decoders in `[dev-dependencies]`. Don't add decoders to the
  main dependency list.
- Examples are the executable documentation. A new public-API feature should
  come with either a doctest or an example that exercises it.

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
  for constants. Color constants in examples follow the
  `COLOR_<THING>` pattern (see `snake.rs`).
- Section banners (`// ── Section ──...──`) are used in examples; keep them
  consistent when adding code there.
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
- **`~/tools/llm-instructions/` and `~/tools/spend.py` are read-only**, so the
  rules this session operates under cannot be edited mid-session.
- **`~/.msmtprc` is read-only** — present only so an unsupervised run can send
  its one summary email.
- `--cap-drop=ALL` and `--security-opt=no-new-privileges:true`.

Consequence: **the worst realistic outcome of a bad unattended run is commits
and branches inside this repository, plus one email.** Nothing else on the
machine is writable, and nothing else is reachable.

### Credentials

Agent sessions push as `jefwagner` with a fine-grained PAT scoped to this repo
only. Setup and the full permission list are in
`~/tools/llm-instructions/pat-setup.md`; the token lives at
`~/.config/jef/termcanvas-pat` (mode 600, outside the repo), bind-mounted in at
the same relative path.

`git-askpass.sh` (tracked, repo root) reads it at call time; `devcontainer.json`
sets `GIT_ASKPASS`, `GIT_TERMINAL_PROMPT=0`, and `GIT_SSH_COMMAND=/bin/false`.
The remote is already HTTPS (`https://github.com/jefwagner/termcanvas.git`), so
the token is genuinely used.

If a git command prompts interactively, something is misconfigured: stop and
say so rather than working around it.

### Branches

- **The primary branch is `main`** (currently the only branch).
- Work goes on `agent/<short-desc>` branches. Never commit to `main` directly
  from an agent session.
- Prefer a worktree for anything unattended (see Unsupervised mode, step 3).

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

`~/wiki/` is **mounted read-only**. Read it freely — that is the point:

- `~/wiki/kb/` — the distilled fundamentals (terminal escape sequences, color
  models, character-cell vs. pixel rendering, whatever the kb holds on TUI
  rendering).
- `~/wiki/projects/termcanvas/` — implementation specifics for this project
  (when written).
- `~/wiki/projects/<other>/` — **when a decision has cross-project
  implications.** The sister projects share vocabulary and containment design;
  reading them is how a decision gets made consistent with the rest of the
  estate rather than merely locally plausible.

**You cannot write to `~/wiki/`, and must not try.**

### Proposing an update

If the wiki would have changed, write a **proposal** to
`notes/wiki-proposals/YYYY-MM-DD-short-topic.md` (create the directory if needed),
structured as the change rather than as a session summary:

- exact target paths under `~/wiki/`,
- for each, the full new text or a precise before/after,
- what `~/wiki/AGENTS.md` would require: new inbound links, index updates, a
  matching edit to `kb/Projects-Summary.md`, cross-links to related pages.

Commit it with the item and point at it in the end-of-chunk summary, so
promotion is mechanical for a human rather than a research project.

**Never promote a proposal yourself**, in any mode. The wiki is the one place in
this estate where an undetected error gets *stronger with use* rather than
caught: prose has no failing test.

**While unsupervised, do not propose either** — note it in the summary and stop.

## Things to Avoid

- Do not run cargo with `+nightly` — `termcanvas` is stable-Rust (edition 2024).
- Do not rewrite doctests (public-API tests) during a refactor. If a refactor
  seems to require it, that is a contract change: call it out explicitly. Inline
  `mod test` unit tests are fair game to rewrite.
- Do not add `unwrap()` in library code; use `expect("reason")` or return
  `Option`/`Result`.
- Do not enable default `image` features (format decoders) in the main
  dependency — keep decoders dev-only.
- Do not claim a rendering change is verified by `cargo test` alone — say what
  example run is needed to see it.
- Do not write to `~/wiki/` from implementation tasks — wiki updates happen via
  proposals, promoted by a human.
