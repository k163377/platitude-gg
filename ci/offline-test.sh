#!/usr/bin/env bash
# Runs prebuilt test executables plus an offscreen app smoke test.
# The caller wraps this script in `unshare -n` (empty network namespace),
# proving the whole suite — including real git subprocess integration
# tests — passes with zero network access (実装計画 §11.3).
set -euo pipefail

# Loopback only; there is no route out of this namespace.
ip link set lo up 2>/dev/null || true

if [ "$#" -lt 1 ]; then
    echo "usage: offline-test.sh <test-binary>... " >&2
    exit 2
fi

status=0
for bin in "$@"; do
    echo "== offline: ${bin}"
    if ! "${bin}"; then
        status=1
    fi
done

# Offscreen smoke: open a locally generated repository, stream it, and let
# causal completion finish the run. The coreutils timeout is the parent kill
# guard for a broken automation path; app startup is not bounded by a fixed
# quit clock.
if [ -n "${PG_SMOKE_BIN:-}" ]; then
    echo "== offline smoke: ${PG_SMOKE_BIN}"
    smoke_root="$(mktemp -d)"
    smoke_repo="${smoke_root}/repo"
    smoke_config="${smoke_root}/config"
    mkdir -p "${smoke_repo}" "${smoke_config}"
    git -C "${smoke_repo}" init -q -b main
    git -C "${smoke_repo}" config user.name smoke
    git -C "${smoke_repo}" config user.email smoke@example.com
    git -C "${smoke_repo}" config commit.gpgsign false
    echo hello > "${smoke_repo}/a.txt"
    git -C "${smoke_repo}" add .
    git -C "${smoke_repo}" commit -qm "smoke commit"

    timeout 50s env \
        -u PG_AUTO_ACT \
        -u PG_AUTO_ACT_ARG \
        -u PG_AUTO_IDENTITY \
        -u PG_AUTO_IDENTITY_SAVE \
        -u PG_AUTO_OPEN \
        -u PG_AUTO_PERF \
        -u PG_AUTO_QUIT_MS \
        -u PG_AUTO_SCROLL \
        -u PG_AUTO_SELECT \
        -u PG_AUTO_WATCHDOG_MS \
        -u PG_AUTO_WIP \
        -u PG_FAKE_PR \
        -u PG_MEM_REPORT \
        -u PG_PLAIN_CHROME \
        -u PG_SCROLL_TO \
        -u PG_SHOT_DIR \
        QT_QPA_PLATFORM=offscreen \
        PG_CONFIG_DIR="${smoke_config}" \
        PG_AUTO_OPEN="${smoke_repo}" \
        PG_AUTO_ACT=band \
        PG_AUTO_WATCHDOG_MS=30000 \
        PG_LOG=info \
        "${PG_SMOKE_BIN}"
fi

exit "${status}"
