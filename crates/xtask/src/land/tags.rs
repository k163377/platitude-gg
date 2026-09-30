//! The pair of tags a landing leaves when it moves a version the shipped
//! build is made from ([`super::versions`]): the last main on the old
//! versions, and the main the update and its follow-up work arrived at.
//! History is linear, so nothing else says where a landing's commits
//! begin and end (反映前テストの機械化.md §版を動かした land のタグ).

use std::process::Command;

use super::versions::{self, Move, Versions};
use crate::subprocess::{git_query, run_captured};

/// Tags a landing that took main from `before` to `after` in `commits`
/// commits, when it moved the build's versions, and pushes the pair where
/// main goes. Says what it did. Answers whether there was anything to tag.
///
/// Main has moved by now, so nothing here can fail the landing: a tag git
/// refused is said in git's words, and a push that did not happen with
/// the command that finishes it.
pub(super) fn bracket(here: &str, before: &str, after: &str, commits: u32) -> bool {
    let moves = versions::moved(&Versions::at(here, before), &Versions::at(here, after));
    if moves.is_empty() {
        return false;
    }
    match tag_pair(here, before, after, commits, &moves) {
        Ok(pair) => push(here, &pair),
        Err(why) => println!("note: {why}"),
    }
    true
}

fn tag_pair(
    here: &str,
    before: &str,
    after: &str,
    commits: u32,
    moves: &[Move],
) -> Result<[String; 2], String> {
    let date = git_query(here, &["log", "-1", "--format=%cd", "--date=short", after])
        .filter(|date| !date.is_empty())
        .ok_or("git could not date main's new tip, so the version tags were not made")?;
    let [old, new] = pair(&date, |name| {
        git_query(
            here,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("refs/tags/{name}"),
            ],
        )
        .is_some()
    });
    let body: Vec<String> = moves.iter().map(Move::to_string).collect();
    let body = body.join("\n");
    create(
        here,
        &old,
        &format!(
            "Last main on the old versions: the update starts after this and is done at {new}\n\n\
             {body}\n"
        ),
        before,
    )?;
    create(
        here,
        &new,
        &format!(
            "Update done: {commits} commit(s) of the update and its follow-up since {old}\n\n{body}\n"
        ),
        after,
    )?;
    println!("tagged the version move: {old} = {before}, {new} = {after}");
    Ok([old, new])
}

/// `deps/<date>/before` and `…/after`; a second update landed the same
/// day is `deps/<date>-2`, and so on.
fn pair(date: &str, taken: impl Fn(&str) -> bool) -> [String; 2] {
    let mut n = 1;
    loop {
        let stem = if n == 1 {
            format!("deps/{date}")
        } else {
            format!("deps/{date}-{n}")
        };
        let names = [format!("{stem}/before"), format!("{stem}/after")];
        if !names.iter().any(|name| taken(name)) {
            return names;
        }
        n += 1;
    }
}

fn create(here: &str, name: &str, message: &str, at: &str) -> Result<(), String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(here)
        .args(["tag", "-a", name, "-m", message, at]);
    let output = run_captured(&mut command)?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git could not make the tag {name}:\n{}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// Both or neither (`--atomic`), to main's remote. Never waits on a
/// prompt, and gives up on a transfer that stalls (git sets no speed
/// floor of its own): the landing still holds the landings' queue, and a
/// push that wants a password or a network is the user's to finish.
fn push(here: &str, pair: &[String; 2]) {
    let refs: Vec<String> = pair
        .iter()
        .map(|name| format!("refs/tags/{name}"))
        .collect();
    let by_hand = |remote: &str| format!("git push --atomic {remote} {}", refs.join(" "));
    let Some(remote) = git_query(here, &["config", "--get", "branch.main.remote"])
        .filter(|remote| !remote.is_empty())
    else {
        println!(
            "note: main tracks no remote, so the version tags stand here only — push them with \
             `{}`",
            by_hand("<remote>")
        );
        return;
    };
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(here)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .args([
            "-c",
            "http.lowSpeedLimit=1000",
            "-c",
            "http.lowSpeedTime=30",
        ])
        .args(["push", "--atomic", "--quiet", &remote])
        .args(&refs);
    let refused = match run_captured(&mut command) {
        Ok(output) if output.status.success() => {
            println!("pushed {} and {} to {remote}", pair[0], pair[1]);
            return;
        }
        Ok(output) => String::from_utf8_lossy(&output.stderr).trim().to_string(),
        Err(why) => why,
    };
    println!(
        "note: the version tags were not pushed to {remote}, so they stand here only — push them \
         with `{}`:\n{refused}",
        by_hand(&remote)
    );
}

#[cfg(test)]
mod tests {
    use super::pair;

    #[test]
    fn the_pair_is_named_by_the_day_and_a_second_update_counts_on() {
        assert_eq!(
            pair("2026-09-30", |_| false),
            ["deps/2026-09-30/before", "deps/2026-09-30/after"]
        );
        let taken = ["deps/2026-09-30/before", "deps/2026-09-30-2/after"];
        assert_eq!(
            pair("2026-09-30", |name| taken.contains(&name)),
            ["deps/2026-09-30-3/before", "deps/2026-09-30-3/after"]
        );
    }
}
