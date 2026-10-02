# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/base:1-ubuntu-24.04

# The single most important line in this file is the openssh purge below.
#
# openssh-client is deliberately REMOVED. Without an ssh binary on PATH, git has
# no SSH transport available at all: it cannot fall back to ~/.ssh/id_ed25519
# even if the key were somehow present, and even if GIT_SSH_COMMAND were unset.
# This repo gets containment from having exactly one credential — the
# repo-scoped PAT mounted in by devcontainer.json — and no other transport by
# which to use any credential. That is a wall, not a speed bump.
#
# This has to be an explicit purge, not an omission. The devcontainers/base
# image ships openssh-client already, so simply not listing it in the apt
# install below leaves /usr/bin/ssh in place and the whole property silently
# does not hold. Verified by building and running the image: without this
# purge, `command -v ssh` returns /usr/bin/ssh. Do not "fix" a git problem by
# reinstalling openssh-client.

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    ca-certificates \
    curl \
    git \
    ddgr \
    msmtp \
    msmtp-mta \
    && rm -rf /var/lib/apt/lists/*

# Strip every openssh component, not just the client, so nothing can be
# re-enabled by pulling in a package that happens to depend on one. rsync goes
# too — it is not a credential transport (its remote mode needs an ssh binary,
# which is what the purge removes), so this is hygiene rather than a fix, but a
# file arguing for a minimal attack surface should not leave unexplained
# binaries lying in it.
RUN apt-get purge -y 'openssh-*' rsync && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && rm -f /usr/bin/ssh /usr/bin/scp /usr/bin/sftp /usr/bin/ssh-agent /usr/bin/ssh-add /usr/bin/rsync

# uv, copied from the official Astral distroless image (fast, no extra cruft).
# There is no Python in this image beyond what uv fetches itself, and that is
# on purpose: the unsupervised spend cap is invoked as `uv run
# ~/tools/spend.py`, and spend.py is a standalone PEP 723 script. If uv is
# missing that command fails, and an agent told to check a cap against a
# broken command is worse off than one with no cap at all — the check appears
# to have happened. UV_PYTHON_DOWNLOADS=automatic lets uv fetch an interpreter
# rather than depending on a system python.
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

ENV UV_LINK_MODE=copy \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=automatic

# Rust toolchain, installed as vscode into the same CARGO_HOME/RUSTUP_HOME
# that devcontainer.json's remoteEnv points at. Stable native only —
# termcanvas is a pure-Rust crate built with plain cargo; no cross targets,
# no cmake, no openssl (no C dependencies at all).
RUN mkdir -p /home/vscode/.rustup /home/vscode/.cargo
RUN chown -R vscode:vscode /home/vscode/.rustup /home/vscode/.cargo
USER vscode
ENV RUSTUP_HOME=/home/vscode/.rustup \
    CARGO_HOME=/home/vscode/.cargo
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable --profile default
ENV PATH=/home/vscode/.cargo/bin:$PATH

# Node.js (NodeSource LTS) + pi coding agent, installed natively in the image
# (reproducible; no host mounts for the binary).
USER root
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g @earendil-works/pi-coding-agent

# pi agent config dir. devcontainer.json mounts the repo-scoped OpenRouter
# auth.json at ~/.pi/agent/auth.json; the parent dir must exist and be
# writable by vscode so pi can store models-store.json and sessions too.
RUN mkdir -p /home/vscode/.pi/agent && chown -R vscode:vscode /home/vscode/.pi

# Create workspace mountpoint
RUN mkdir -p /workspace && chown vscode:vscode /workspace

USER vscode
WORKDIR /workspace

# msmtp is present: the unsupervised mode in AGENTS.md sends exactly one
# summary email per run, to one address, and ~/.msmtprc is mounted read-only
# for that purpose. If that ever changes, remove msmtp here and the mount —
# and re-read that section, because a summary email is a
# send-to-the-outside-world action that deserves a second thought.

CMD ["sleep", "infinity"]
