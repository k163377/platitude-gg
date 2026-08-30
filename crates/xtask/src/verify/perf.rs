//! The offscreen contract check for the real-window performance driver.
use std::process::Command;

pub(super) fn configure(cmd: &mut Command, verb: &str, arg: &str) -> Result<(), String> {
    if verb != "perf" {
        return Ok(());
    }
    if !["none", "details", "diff", "scroll", "scroll-none"].contains(&arg) {
        return Err("verify-ui perf takes none, details, diff, scroll, or scroll-none".into());
    }
    cmd.env("PG_AUTO_PERF", "1")
        .env("PG_PERF_TRACE_FRAMES", "1")
        .env(
            "PG_PERF_SELECTION",
            if ["none", "scroll-none"].contains(&arg) {
                "none"
            } else {
                "first"
            },
        )
        .env(
            "PG_PERF_DIFF",
            if ["diff", "scroll"].contains(&arg) {
                "1"
            } else {
                "0"
            },
        );
    if ["scroll", "scroll-none"].contains(&arg) {
        cmd.env("PG_AUTO_SCROLL", "1");
    }
    Ok(())
}
