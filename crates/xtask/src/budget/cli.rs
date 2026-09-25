//! `cargo xtask budget` — what the machine is doing, said to a person,
//! and the hold the suite takes from a process of its own.

use std::cell::Cell;
use std::path::PathBuf;
use std::process::Command;

use super::ledger::{Pool, QUIET_CEILING};
use super::queue::Rank;
use super::unit::{Ask, HELD, LIGHT};
use crate::wait::{Budget, LOOK_AGAIN, Wait};

pub fn run(args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|first| first == "hold") {
        return hold(&args[1..]);
    }
    let dir = match args.len() {
        0 => crate::tree::workspace_root(),
        2 if args[0] == "--dir" => PathBuf::from(&args[1]),
        _ => {
            return Err(format!(
                "unknown option {:?} (budget takes --dir <tree>, or `hold`)",
                args.join(" ")
            ));
        }
    };
    let pool = Pool::of(&dir, crate::budget::default_jobs())?;
    print!("{}", pool.standing()?);
    Ok(())
}

/// `cargo xtask budget hold …` — one unit's ticket held by a real
/// process, so `tests/gate/budget.rs` can watch priority, exclusion and a
/// holder's death. Writes its pid to `--queued` on joining the queue and
/// to `--say` on admission, and holds until `--until` exists: the test
/// drives every edge, with no clock.
fn hold(args: &[String]) -> Result<(), String> {
    let mut dir = crate::tree::workspace_root();
    let (mut weight, mut jobs) = (LIGHT, crate::budget::default_jobs());
    let mut rank = Rank::Normal;
    let (mut seat, mut what) = ("held".to_string(), "hold".to_string());
    let (mut say, mut until, mut turn) = (None, None, false);
    let mut queued: Option<PathBuf> = None;
    let mut child_until: Option<PathBuf> = None;
    let mut at = 0;
    while let Some(arg) = args.get(at) {
        let next = |at: &mut usize| -> Result<String, String> {
            *at += 1;
            args.get(*at)
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match arg.as_str() {
            "--dir" => dir = PathBuf::from(next(&mut at)?),
            "--weight" => {
                weight = next(&mut at)?
                    .parse()
                    .map_err(|_| "--weight takes a count")?;
            }
            "--jobs" => jobs = next(&mut at)?.parse().map_err(|_| "--jobs takes a count")?,
            "--seat" => seat = next(&mut at)?,
            "--what" => what = next(&mut at)?,
            "--say" => say = Some(PathBuf::from(next(&mut at)?)),
            "--queued" => queued = Some(PathBuf::from(next(&mut at)?)),
            "--until" => until = Some(PathBuf::from(next(&mut at)?)),
            "--child-until" => child_until = Some(PathBuf::from(next(&mut at)?)),
            "--landing" => rank = Rank::Landing,
            "--launch" => rank = Rank::Launch,
            "--turn" => turn = true,
            other => return Err(format!("unknown option {other:?}")),
        }
        at += 1;
    }
    let until = until.ok_or("budget hold needs --until <file>: the word to let go")?;
    let pool = Pool::of(&dir, jobs)?;
    // The closure cannot return an error, so a failed write is kept and
    // reported after the wait.
    let unwritten: Cell<Option<String>> = Cell::new(None);
    let arrived = || {
        if let Some(path) = &queued
            && let Err(error) = std::fs::write(path, format!("{}\n", std::process::id()))
        {
            unwritten.set(Some(format!("could not write {}: {error}", path.display())));
        }
    };
    let ask = Ask {
        weight,
        rank,
        seat: &seat,
        what: &what,
    };
    let held = if turn {
        pool.turn_arriving(&seat, &what, &arrived)?
    } else {
        pool.admit_arriving(&ask, &arrived)?
    };
    if let Some(error) = unwritten.take() {
        return Err(error);
    }
    // Stands in for the cargo or container a step starts: the ledger is
    // told its pid, so a test can kill this holder and watch the room
    // stay held until the child goes.
    let mut child = match &child_until {
        Some(word) => {
            let me = std::env::current_exe().map_err(|e| e.to_string())?;
            let mut command = Command::new(&me);
            command
                .args(["budget", "hold", "--dir"])
                .arg(&dir)
                .arg("--until")
                .arg(word)
                .env(HELD, "1")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let child = command.spawn().map_err(|e| e.to_string())?;
            held.started(child.id(), &me.display().to_string());
            Some(child)
        }
        None => None,
    };
    if let Some(say) = say {
        std::fs::write(&say, format!("{}\n", std::process::id()))
            .map_err(|e| format!("could not write {}: {e}", say.display()))?;
    }
    println!("held {what} after {}ms", held.waited.as_millis());
    // A hold whose word never comes (the suite gone with its directory)
    // gives the room back after the queue's silence ceiling.
    let mut wait = Wait::new(
        format!("the word at {}", until.display()),
        Budget::whole(QUIET_CEILING),
        LOOK_AGAIN,
    );
    while !until.exists() {
        wait.look_again("the word to let go")
            .map_err(|expired| expired.to_string())?;
    }
    if let Some(child) = &mut child {
        child.wait().map_err(|e| e.to_string())?;
    }
    drop(held);
    Ok(())
}
