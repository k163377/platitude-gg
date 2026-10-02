# `cargo xtask linux <cargo args>` builds this image and runs the workspace
# inside it — with WSL's own engine (wslc) on Windows, docker elsewhere.
#
# ubuntu:24.04: what CI's ubuntu-latest resolves to, and its git is the
# oldest git最低バージョン整合.md supports — no other machine here runs it.
#
# The core stage stops at git and Rust (platitude-core is Qt-free): without
# Qt the image builds an order of magnitude faster. `cargo xtask linux`
# picks core or app.
FROM ubuntu:24.04 AS core

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# build-essential: cc, rustc's linker. openssh-client: ssh-keygen, which
# git's ssh signing runs (the signing tests); CI's runners already have it.
# tini: the gate's long-lived container starts behind it, to reap what a
# verb leaves orphaned — docker's `--init` is tini, and wslc has no such
# flag (`linux::engine::INIT`).
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        openssh-client \
        tini \
    && rm -rf /var/lib/apt/lists/*

# rust-toolchain.toml stays the one toolchain pin: rustup installs what it names.
RUN curl --proto '=https' --tlsv1.2 --silent --show-error --fail \
        https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain none
COPY rust-toolchain.toml /tmp/pin/
RUN cd /tmp/pin && rustup toolchain install && rm -rf /tmp/pin

# The registry is a volume every container mounts (`linux::REGISTRY_MOUNT`),
# but cargo's two lock files sit in CARGO_HOME, per container. Linked into
# the registry they are shared, so two containers unpacking the same crate
# take turns instead of the second failing on the first's `.cargo-ok`. The
# directory exists in the image so the links resolve without the mount.
RUN mkdir -p /usr/local/cargo/registry \
    && ln -s registry/.package-cache /usr/local/cargo/.package-cache \
    && ln -s registry/.package-cache-mutate /usr/local/cargo/.package-cache-mutate

# /work is a bind mount owned by the host's user, and git refuses a repository
# it takes for someone else's. The integration tests never read this (they
# set GIT_CONFIG_NOSYSTEM).
RUN git config --system --add safe.directory /work

WORKDIR /work

# Everything platitude-app needs: Qt, a software GL stack for the offscreen
# platform, and the fonts the design tokens name for Ubuntu.
FROM core AS app

# From the caller: ci.yml pins it and `cargo xtask linux` passes it in —
# no second copy here.
ARG QT_VERSION

# The two expensive downloads go first, so editing the library list below
# never fetches Qt or the fonts again.
#
# fonts-noto-cjk / -extra carry Noto Sans CJK JP and Noto Sans Mono CJK JP,
# the Ubuntu links of both font chains in デザイン規約; fonts-dejavu-core is
# the last link of both.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        fonts-dejavu-core \
        fonts-noto-cjk \
        fonts-noto-cjk-extra \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*

# A venv: 24.04's system python is externally managed and refuses pip.
# --base pins the primary server, as CI does.
RUN python3 -m venv /opt/aqt \
    && /opt/aqt/bin/pip install --no-cache-dir aqtinstall \
    && /opt/aqt/bin/aqt install-qt --base https://download.qt.io/ \
        linux desktop "${QT_VERSION}" linux_gcc_64 -O /opt/qt \
    && rm -rf /opt/aqt

# libgl1-mesa-dev and libxkbcommon-dev are CI's two installs. The rest a
# GitHub runner already has, and Qt's libraries want them with or without a
# display (without libglib2.0 even `qmake -query` dies). libgl1-mesa-dri is
# llvmpipe: no GPU reaches a container, so the offscreen platform renders on it.
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

# gnupg for the `E` signature verdict (--preset errsig): without a gpg
# binary git reports a commit signed by an unknown key as unsigned (N).
# Verify only — the demo embeds the signed object.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        gnupg \
    && rm -rf /var/lib/apt/lists/*

# QMAKE for qtbridge, as in CI. LANG: Qt switches a "C" locale to C.UTF-8
# itself and says so on every run; git is unaffected (core pins LC_ALL=C).
ENV PATH=/opt/qt/${QT_VERSION}/gcc_64/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    LD_LIBRARY_PATH=/opt/qt/${QT_VERSION}/gcc_64/lib \
    QMAKE=/opt/qt/${QT_VERSION}/gcc_64/bin/qmake \
    LANG=C.UTF-8

# A stock Ubuntu with Qt staged the way a distribution would ship it and
# nothing else installed: the only place that says the built binary runs
# where it was not built. The QML modules must come too — opened at run
# time, `ldd` never names them.
FROM ubuntu:24.04 AS bare

ARG QT_VERSION
ENV DEBIAN_FRONTEND=noninteractive

COPY --from=app /opt/qt/${QT_VERSION}/gcc_64/lib /opt/pgg-qt/lib
COPY --from=app /opt/qt/${QT_VERSION}/gcc_64/plugins /opt/pgg-qt/plugins
COPY --from=app /opt/qt/${QT_VERSION}/gcc_64/qml /opt/pgg-qt/qml

ENV LD_LIBRARY_PATH=/opt/pgg-qt/lib \
    QT_PLUGIN_PATH=/opt/pgg-qt/plugins \
    QML_IMPORT_PATH=/opt/pgg-qt/qml \
    QML2_IMPORT_PATH=/opt/pgg-qt/qml \
    LANG=C.UTF-8

# The same machine with exactly what a package would have to declare; link
# one more library and `cargo xtask linux bare` stops with its name. The
# list comes from `cargo xtask linux bare --discover` (start, install the
# one library the loader names, repeat), not by hand: a needed library
# hides behind the current blocker. No krb5: Qt6Network's libgssapi_krb5
# comes with git's curl.
#
# fonts-noto-cjk no loader asks for: a stock Ubuntu resolves Noto Sans CJK
# JP and `sans:lang=ja` to DejaVu Sans, which has no kanji.
FROM bare AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        fonts-dejavu-core \
        fonts-noto-cjk \
        git \
        libdbus-1-3 \
        libegl1 \
        libfontconfig1 \
        libgl1 \
        libglib2.0-0t64 \
        libxkbcommon0 \
    && rm -rf /var/lib/apt/lists/*
