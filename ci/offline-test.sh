#!/usr/bin/env bash
# Runs prebuilt test executables and an offscreen app smoke test inside the
# caller's `unshare -n` (empty network namespace; 実装計画 §11.3).
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

# Offscreen smoke on a local repository: the run ends on causal completion,
# and `timeout` is only the kill guard. PGG_SMOKE_BIN must carry the harness
# feature, or the app ignores the knobs below and stands until the timeout.
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

    # Clear every `PGG_` variable except those that say nothing about who
    # drives — core `settings::NOT_AUTOMATION` / xtask `app_env`, kept in
    # step by hand. By prefix, not a list, so a knob added later is cleared.
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
