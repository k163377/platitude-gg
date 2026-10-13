#!/bin/sh
# Git supplies sh also where Claude falls back to PowerShell. A cloud
# session leaves the hook runner unbuilt and the checks to the desk.
[ "$CLAUDE_CODE_REMOTE" = true ] && exit 0
# A shell alias starts at the repository root; Cargo must see the
# directory the original hook was called from, including in a subfolder.
cd "${GIT_PREFIX:-.}" || exit
# In a linked worktree the alias also gets GIT_DIR for that tree, and every
# `git -C <other tree>` under it would answer for this one.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES
exec cargo run --quiet -p xtask --profile hooks -- hook "$@"
