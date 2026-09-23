# The Linux half of a Windows workstation: `cargo xtask linux <cargo args>`
# builds this image and runs the workspace inside it.
#
# ubuntu:24.04 earns the base twice over. It is what CI's ubuntu-latest
# resolves to, so a failure here is reproducible there and the other way
# round. And its git is 2.43 — the oldest version git最低バージョン整合.md promises to
# support, which no other machine in this project runs. Green here is
# the floor.
#
# The core stage stops at git and Rust. platitude-core is Qt-free by rule,
# so the tests that most want a second operating system — paths, a
# case-sensitive filesystem, file modes, git itself — need none of it, and
# leaving Qt out is the difference between a two-minute image and a
# twenty-minute one. Work that reaches platitude-app builds the app stage
# below; `cargo xtask linux` picks between the two.
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

# The registry is a volume every container on the machine mounts
# (`linux::REGISTRY_MOUNT`), and cargo guards it with two lock files in
# CARGO_HOME itself, which would be each container's own. Linked into the
# registry, the locks are the volume's too: two containers after the same new
# crate take turns instead of both unpacking it, the second failing on the
# `.cargo-ok` the first created (cargo's `unpack_package` makes it
# `create_new`). The directory stands in the image as well, so the links
# resolve in a container that mounts nothing there.
RUN mkdir -p /usr/local/cargo/registry \
    && ln -s registry/.package-cache /usr/local/cargo/.package-cache \
    && ln -s registry/.package-cache-mutate /usr/local/cargo/.package-cache-mutate

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

# The version comes from the caller: ci.yml pins it (calling itself the
# toolchain lockfile) and `cargo xtask linux` passes it in. A second
# copy here is a second place to forget.
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
# found"). libgl1-mesa-dri is llvmpipe: no GPU reaches a container, so it
# is what the offscreen platform renders through.
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

# gnupg is for the `E` signature verdict (--preset errsig): a commit
# signed by a key no keyring holds is the one state git can only report
# through a gpg binary — absent one, it calls the same commit unsigned
# (measured: N). Verifying is the whole ask; the demo embeds the signed
# object, so nothing here ever signs. A RUN of its own, below the Qt and
# font layers, so adding it never fetched them again.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        gnupg \
    && rm -rf /var/lib/apt/lists/*

# qtbridge finds Qt through QMAKE, as CI hands it over; the built binary
# finds its libraries through LD_LIBRARY_PATH. Qt switches a "C" locale to
# C.UTF-8 on its own and says so on every run — naming it here says it once.
# git is unaffected either way: core pins LC_ALL=C per command.
ENV PATH=/opt/qt/${QT_VERSION}/gcc_64/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    LD_LIBRARY_PATH=/opt/qt/${QT_VERSION}/gcc_64/lib \
    QMAKE=/opt/qt/${QT_VERSION}/gcc_64/bin/qmake \
    LANG=C.UTF-8

# A stock Ubuntu with Qt staged beside the binary the way a distribution
# would ship it, and *nothing else installed*. The two stages above prove
# the code builds; these last two are the only place that can say the built
# thing runs somewhere it was not built.
#
# What a distribution ships is the libraries, the platform plugins and the
# QML modules. The modules matter most — they are opened at run time, so
# `ldd` never names them, and a set chosen from the linker's answer starts
# and then dies loading QML.
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

# The same machine, with exactly what a package would have to declare. The
# list is derived: `cargo xtask linux bare --discover` walks it out in the
# stage above by starting the binary, reading the one library the loader
# names, installing only that, and going round again. What that returned,
# in the order it returned it:
#
#     git libglib2.0-0t64 libegl1 libfontconfig1 libxkbcommon0 libgl1
#     libdbus-1-3
#
# git leads because the product is a GUI over it, and it settles krb5 on the
# way: Qt6Network wants libgssapi_krb5 and git's own curl brings it, so the
# list does not name it. Everything else the build stage installs turned out
# to be somebody else's dependency — llvmpipe included, which an offscreen
# run never asked for.
#
# That makes the list a claim and this stage the test of it: link one more
# library and `cargo xtask linux bare` stops with its name. It has already
# caught two. libGL and libdbus were both missing from the list written by
# hand, because that first pass added a package per round whatever the error
# said, and a library that is needed but not the *current* blocker hides
# behind the one that is.
#
# fonts-noto-cjk is the one entry no loader can ask for: デザイン規約 names
# Noto Sans CJK JP for Ubuntu, and a stock Ubuntu resolves both it and
# `sans:lang=ja` to DejaVu Sans, which has no kanji in it.
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
