//! The script that holds a write for its run.

/// The hook of `--preset slowhook` and `slowmerge` (`demo::remote`), and the
/// merge tool of `menu-wait conflict` (`verify::repos::held_tool`): let go by the run's driver
/// — the mark `Harness.letHookGo` leaves beside the pictures
/// (`verify::LET_GO_FILE`) — or at the trail's station, whichever comes
/// first; `by_hand_secs` seconds with no run around it. `sleep` is the blocking
/// command all three OSes agree on (`verify::repos`). The trail, its
/// station and the ceiling are the ones the run's own wedge reading, git
/// shim and watchdog use (`verify::TRAIL_FILE`, `verify::WRITES_JOINING`,
/// `verify::WATCHDOG_MS`), so a renamed file or station or a moved ceiling
/// moves the hook too. The trail's lines are `<seconds>s <station>`, each
/// ending in its newline (`platitude_app::harness::deadline`); a line the
/// writer was stopped in has none, and `read` hands it to nothing.
pub(crate) fn held_until_let_go(by_hand_secs: u64) -> String {
    /// How often the script looks for the mark and at the trail, in
    /// milliseconds.
    const LOOK_MS: u64 = 200;
    format!(
        r#"#!/bin/sh
if [ -z "$_PGG_SHOT_DIR" ]; then
  sleep {by_hand_secs}
  exit 0
fi
shots="$(printf '%s' "$_PGG_SHOT_DIR" | tr '\\' '/')"
trail="$shots/{trail}"
looks=0
while [ "$looks" -lt {looks} ]; do
  if [ -f "$shots/{let_go}" ]; then
    exit 0
  fi
  if [ -f "$trail" ]; then
    while IFS= read -r line; do
      case "$line" in
        *" {station}") exit 0 ;;
      esac
    done < "$trail"
  fi
  sleep {look}
  looks=$((looks + 1))
done
"#,
        looks = crate::verify::WATCHDOG_MS / LOOK_MS,
        trail = crate::verify::TRAIL_FILE,
        let_go = crate::verify::LET_GO_FILE,
        station = crate::verify::WRITES_JOINING,
        look = format_args!("{}.{:03}", LOOK_MS / 1000, LOOK_MS % 1000),
    )
}
