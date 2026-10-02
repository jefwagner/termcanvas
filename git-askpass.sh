#!/usr/bin/env bash
# git-askpass.sh — supply the repo-scoped fine-grained PAT to git.
#
# The token itself never lives in this repo. It is read, at call time, from:
#     ~/.config/jef/termcanvas-pat      (mode 600)
#
# Usage (export both):
#     export GIT_ASKPASS="$PWD/git-askpass.sh"
#     export GIT_TERMINAL_PROMPT=0
#
# GIT_TERMINAL_PROMPT=0 matters as much as GIT_ASKPASS: without it a failed
# lookup falls back to an interactive prompt, which in an unattended session
# hangs until the run is killed.

set -euo pipefail

PAT_FILE="${TERM_CANVAS_PAT:-$HOME/.config/jef/termcanvas-pat}"

if [[ ! -r "$PAT_FILE" ]]; then
    echo "git-askpass: cannot read PAT at $PAT_FILE" >&2
    echo "  create it with: gh auth token setup  (see ~/tools/llm-instructions/pat-setup.md)" >&2
    exit 1
fi

# git calls askpass with a prompt describing what it wants; the username is
# always the token owner, and only the password is the secret.
case "${1:-}" in
    *sername*) printf '%s\n' "jefwagner" ;;
    *)         tr -d '\r\n' < "$PAT_FILE"; printf '\n' ;;
esac