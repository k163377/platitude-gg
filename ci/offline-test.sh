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

# Offscreen smoke: open a locally generated repository, stream it, quit.
if [ -n "${PG_SMOKE_BIN:-}" ]; then
    echo "== offline smoke: ${PG_SMOKE_BIN}"
    smoke_repo="$(mktemp -d)/repo"
    mkdir -p "${smoke_repo}"
    git -C "${smoke_repo}" init -q -b main
    git -C "${smoke_repo}" config user.name smoke
    git -C "${smoke_repo}" config user.email smoke@example.com
    git -C "${smoke_repo}" config commit.gpgsign false
    echo hello > "${smoke_repo}/a.txt"
    git -C "${smoke_repo}" add .
    git -C "${smoke_repo}" commit -qm "smoke commit"

    QT_QPA_PLATFORM=offscreen \
    PG_AUTO_OPEN="${smoke_repo}" \
    PG_AUTO_QUIT_MS=4000 \
    PG_LOG=info \
        "${PG_SMOKE_BIN}"
fi

exit "${status}"
