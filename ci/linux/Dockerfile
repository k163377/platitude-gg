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
# twenty-minute one. Work that reaches platitude-app builds the app stage
# below instead; `cargo xtask linux` picks between the two.
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

# Everything platitude-app needs: Qt to build and link against, a software
# GL stack for the offscreen platform to render through, and the fonts the
# design tokens name for Ubuntu — the chain is only a chain if the machine
# actually has the families in it, and no machine in this project had them
# before this image.
FROM core AS app

# No default: the version is CI's to pin (ci.yml calls itself the toolchain
# lockfile), and `cargo xtask linux` reads it from there and passes it in.
# A second copy here is a second place to forget.
ARG QT_VERSION

# The two expensive downloads go first and stay put, so editing the library
# list below never fetches Qt or the fonts again.
#
# fonts-noto-cjk / -extra are ~200MB and are the point of this stage as much
# as Qt is: they carry Noto Sans CJK JP and Noto Sans Mono CJK JP, which
# デザイン規約 names as the Ubuntu links of both font chains, and no machine
# in this project had them before. fonts-dejavu-core is the last link of
# both.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        fonts-dejavu-core \
        fonts-noto-cjk \
        fonts-noto-cjk-extra \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*

# aqtinstall in a virtual environment because 24.04 marks the system
# interpreter externally managed and refuses pip outright. --base pins the
# primary server, as CI does: a mirror redirect is a different host serving
# the same name.
RUN python3 -m venv /opt/aqt \
    && /opt/aqt/bin/pip install --no-cache-dir aqtinstall \
    && /opt/aqt/bin/aqt install-qt --base https://download.qt.io/ \
        linux desktop "${QT_VERSION}" linux_gcc_64 -O /opt/qt \
    && rm -rf /opt/aqt

# libgl1-mesa-dev and libxkbcommon-dev are the two CI installs, so the
# machines link against the same system libraries. The rest is what a
# GitHub runner happens to have already and an empty container does not:
# without libglib2.0 even `qmake -query` dies before printing a version,
# and Qt's own libraries want fontconfig, freetype, dbus and the X client
# libraries whether or not a display is ever opened (measured by walking
# ldd over the Qt libraries this app loads until nothing came back "not
# found"). libgl1-mesa-dri is llvmpipe: no GPU reaches a container, so the
# offscreen platform renders through software or not at all.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        libdbus-1-3 \
        libegl1 \
        libfontconfig1 \
        libfreetype6 \
        libgl1 \
        libgl1-mesa-dev \
        libgl1-mesa-dri \
        libglib2.0-0t64 \
        libx11-6 \
        libxext6 \
        libxkbcommon-dev \
        libxkbcommon0 \
        libxrender1 \
    && rm -rf /var/lib/apt/lists/*

# qtbridge finds Qt through QMAKE, as CI hands it over; the built binary
# finds its libraries through LD_LIBRARY_PATH. Qt switches a "C" locale to
# C.UTF-8 on its own and says so on every run — naming it here says it once
# instead. git is unaffected either way: core pins LC_ALL=C per command.
ENV PATH=/opt/qt/${QT_VERSION}/gcc_64/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    LD_LIBRARY_PATH=/opt/qt/${QT_VERSION}/gcc_64/lib \
    QMAKE=/opt/qt/${QT_VERSION}/gcc_64/bin/qmake \
    LANG=C.UTF-8
