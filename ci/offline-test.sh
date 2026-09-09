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
# quit clock. PGG_SMOKE_BIN must be a build carrying the harness feature —
# without it the app reads none of the knobs below and the window stands
# until the timeout kills it.
if [ -n "${PGG_SMOKE_BIN:-}" ]; then
    echo "== offline smoke: ${PGG_SMOKE_BIN}"
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

    # Whatever automation the caller carries is cleared by rule, not by a
    # hand-kept list: everything under `PGG_` except the three names that
    # say nothing about who is driving (xtask `app_env::clear_automation`,
    # core `settings::AUTOMATION_PREFIX` / `settings::NOT_AUTOMATION` —
    # change those and change this). A list would let a knob added to the
    # app later compose two automation protocols in one run.
    smoke_env=(env)
    while IFS= read -r name; do
        case "${name}" in
            PGG_CONFIG_DIR | PGG_LOG | PGG_ALLOW_GUI) ;;
            PGG_*) smoke_env+=(-u "${name}") ;;
        esac
    done < <(compgen -e)

    timeout 50s "${smoke_env[@]}" \
        QT_QPA_PLATFORM=offscreen \
        PGG_CONFIG_DIR="${smoke_config}" \
        PGG_AUTO_OPEN="${smoke_repo}" \
        PGG_AUTO_ACT=band \
        PGG_AUTO_WATCHDOG_MS=30000 \
        PGG_LOG=info \
        "${PGG_SMOKE_BIN}"
fi

exit "${status}"
