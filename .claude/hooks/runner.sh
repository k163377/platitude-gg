#!/bin/sh
# Git supplies sh also where Claude falls back to PowerShell. A cloud
# session leaves the hook runner unbuilt and the checks to the desk.
[ "$CLAUDE_CODE_REMOTE" = true ] && exit 0
# A shell alias starts at the repository root; Cargo must see the
# directory the original hook was called from, including in a subfolder.
cd "${GIT_PREFIX:-.}" || exit
exec cargo run --quiet -p xtask --profile hooks -- hook "$@"
