//! The offscreen contract check for the real-window performance driver.
use std::process::Command;

pub(super) fn configure(cmd: &mut Command, verb: &str, arg: &str) -> Result<(), String> {
    if verb != "perf" {
        return Ok(());
    }
    if ![
        "none",
        "details",
        "diff",
        "scroll",
        "scroll-none",
        "font-walk",
        "colour",
        "diff-scroll",
        "sequence",
    ]
    .contains(&arg)
    {
        return Err(
            "verify-ui perf takes none, details, diff, scroll, scroll-none, font-walk, colour, diff-scroll, or sequence".into(),
        );
    }
    cmd.env("PGG_AUTO_PERF", "1")
        .env("PGG_PERF_TRACE_FRAMES", "1")
        .env(
            "PGG_PERF_SELECTION",
            if ["none", "scroll-none", "font-walk"].contains(&arg) {
                "none"
            } else {
                "first"
            },
        )
        .env(
            "PGG_PERF_DIFF",
            if ["diff", "scroll", "colour", "diff-scroll", "sequence"].contains(&arg) {
                "1"
            } else {
                "0"
            },
        );
    if ["scroll", "scroll-none"].contains(&arg) {
        cmd.env("PGG_AUTO_SCROLL", "1");
    }
    if ["colour", "diff-scroll", "sequence"].contains(&arg) {
        cmd.env("PGG_PERF_COMPLETION", "coloured");
    }
    if ["diff-scroll", "sequence"].contains(&arg) {
        cmd.env("PGG_PERF_DIFF_SCROLL", "1");
    }
    if arg == "sequence" {
        let repo = cmd
            .get_envs()
            .find(|(key, _)| *key == "PGG_AUTO_OPEN")
            .and_then(|(_, value)| value)
            .ok_or("perf sequence needs a repository")?
            .to_string_lossy()
            .into_owned();
        let oids = crate::subprocess::git_query(&repo, &["rev-list", "--max-count=2", "HEAD"])
            .ok_or("perf sequence needs two commits")?;
        let cases = oids
            .lines()
            .enumerate()
            .map(|(i, oid)| format!("case{i}\t{oid}\tbench.kt\tcoloured"))
            .collect::<Vec<_>>();
        if cases.len() != 2 {
            return Err("perf sequence needs two commits".into());
        }
        cmd.env("PGG_PERF_CASES", cases.join("\n"))
            .env("PGG_PERF_CYCLES", "2");
    }
    // The calibration run's shape (`perf::fonts`): unselected, unscrolled,
    // and the font walk paid before `perf_done`. Offscreen there is
    // nothing to weigh — that platform's font database holds no fonts —
    // so what this checks is the three lines, in order.
    if arg == "font-walk" {
        cmd.env("PGG_PERF_FONT_WALK", "1");
    }
    Ok(())
}
