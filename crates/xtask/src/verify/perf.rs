//! The offscreen contract check for the real-window performance driver.
use std::process::Command;

pub(super) fn configure(
    cmd: &mut Command,
    verb: &str,
    arg: &str,
    directory: &std::path::Path,
) -> Result<(), String> {
    if [
        "perf",
        "details-failure",
        "rebase-plan",
        "rebase-plan-run",
        "rebase-edit-stop",
        "wip",
    ]
    .contains(&verb)
    {
        // A missing global identity opens a modal over the graph on Linux.
        // Only this child's git sees the fixture; never write the user's config.
        let identity = directory.join("identity.gitconfig");
        std::fs::write(
            &identity,
            "[user]\nname = Performance Fixture\nemail = perf@example.invalid\n",
        )
        .map_err(|e| e.to_string())?;
        cmd.env("GIT_CONFIG_GLOBAL", identity)
            .env("GIT_CONFIG_NOSYSTEM", "1");
    }
    if verb != "perf" {
        return Ok(());
    }
    if !["none", "details", "diff", "scroll", "scroll-none"].contains(&arg) {
        return Err("verify-ui perf takes none, details, diff, scroll, or scroll-none".into());
    }
    cmd.env("PG_AUTO_PERF", "1")
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
