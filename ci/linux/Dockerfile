# The Linux half of a Windows workstation: `cargo xtask linux <cargo args>`
# builds this image and runs the workspace inside it.
#
# ubuntu:24.04 earns the base twice over. It is what CI's ubuntu-latest
# resolves to, so a failure here is reproducible there and the other way
# round. And its git is 2.43 — the oldest version 要望.md promises to
# support, which no other machine in this project runs. Green here is the
# floor, not just "some Linux".
#
# The core stage carries no Qt on purpose. platitude-core is Qt-free by rule,
# so the tests that most want a second operating system — paths, a
# case-sensitive filesystem, file modes, git itself — need none of it, and
# leaving it out is the difference between a two-minute image and a
# twenty-minute one. The Qt-carrying stage lands with the app work.
FROM ubuntu:24.04 AS core

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# build-essential is here for cc, which rustc shells out to as the linker.
# openssh-client is here for ssh-keygen: the signing tests make a throwaway
# key and let git verify against it, and git's ssh signing is ssh-keygen.
# CI's runners carry it already, so only a minimal image has to ask.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        openssh-client \
    && rm -rf /var/lib/apt/lists/*

# rust-toolchain.toml stays the one place the toolchain is pinned: rustup
# reads it and installs what it names, so the image cannot drift from a plain
# `cargo build` on the host.
RUN curl --proto '=https' --tlsv1.2 --silent --show-error --fail \
        https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain none
COPY rust-toolchain.toml /tmp/pin/
RUN cd /tmp/pin && rustup toolchain install && rm -rf /tmp/pin

# /work arrives as a bind mount owned by whoever owns it on the host, and git
# refuses to touch a repository it believes belongs to someone else. The
# integration tests are indifferent either way — they set GIT_CONFIG_NOSYSTEM
# and build their repositories under /tmp.
RUN git config --system --add safe.directory /work

WORKDIR /work
