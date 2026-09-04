//! `cargo xtask shots <add|open|list|path>`.

use std::path::PathBuf;

use super::{board, sweep, window};

const USAGE: &str = "\
cargo xtask shots <command>

  add --label \"<what these pictures show>\" [--verb <v>] <png>...
      Put pictures on the board under a name, and rebuild the page. The
      seat is not asked for: it is read from the working directory, so a
      run always says which tree took it. --label is required — several
      pictures under no name leave nobody able to say which file was
      which change.

  add --label \"<what changed>\" --before <png> --after <png>
      The same, for two pictures of one thing: they go on the board as a
      single view holding both side by side under one magnifier, each
      with its word over it. Read one after the other they are not a
      comparison — the reader carries the first picture in their head
      while looking at the second — so a before/after goes on the board
      this way and never as two runs.

  prune [--seat <letter>]... [--label \"<label>\"]
      Take runs off the board — their pictures and all — and rebuild the
      page. Without --seat it reaches this seat's own runs and nobody
      else's: the board is shared, and the runs beside yours belong to a
      session that may be showing them right now. Name others to include
      them, or `--seat all` to sweep the board. --label narrows it to one
      name, which is how the pictures of an approach that was abandoned
      leave without taking the rest of the seat's work with them.

      Most runs need none of this: a retake replaces the picture it was
      taken to replace, and a seat's runs go when its work does — the
      branch landing on main, or the seat being handed to a fresh
      stretch of work. Nothing goes because a session ended
      (shots/sweep.rs).

  open [--again]
      Put the board in front of the reader, in one window and only one:
      the page is rewritten at a fixed path, so a window already open
      is one F5 away from the board as it stands, and this says so
      instead of opening another. The page is read top down in the
      order the runs went up, so the newest is at the bottom. That
      holds across seats and sessions — the board is shared, and six
      windows of it are six answers to \"which one is current\".
      --again is the reader's own word that they closed it, and the
      only way past: nothing here can see a closed window
      (shots/window.rs).

  list        One line per run: when, seat, count, label.
  path        Where the page is.
";

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("add") => add(&args[1..]),
        Some("prune") => prune(&args[1..]),
        Some("open") => open(&args[1..]),
        Some("list") => {
            let runs = board::load_runs(&board::board_dir()?.join("runs"));
            for run in &runs {
                println!(
                    "{}  seat {}  {} shot(s)  {}{}",
                    stamp(run.at),
                    run.seat,
                    run.shots.len(),
                    run.label,
                    if run.verb.is_empty() {
                        String::new()
                    } else {
                        format!("  [{}]", run.verb)
                    }
                );
            }
            println!("{} run(s) on the board", runs.len());
            Ok(())
        }
        Some("path") => {
            println!("{}", board::shown(&board::board_dir()?.join("index.html")));
            Ok(())
        }
        _ => {
            print!("{USAGE}");
            Err("shots: no command".to_string())
        }
    }
}

/// `shots open` — the board in front of the reader, and the whole of the
/// one-window rule is in `window::show`.
fn open(args: &[String]) -> Result<(), String> {
    let mut again = false;
    for arg in args {
        match arg.as_str() {
            "--again" => again = true,
            other => return Err(format!("shots open: unknown argument {other}")),
        }
    }
    let board = board::board_dir()?;
    let page = board.join("index.html");
    if !page.exists() {
        return Err(format!(
            "no board yet at {} — put something on it with `cargo xtask shots add`",
            board::shown(&page)
        ));
    }
    window::show(&board, &page, again)
}

fn add(args: &[String]) -> Result<(), String> {
    let mut label = String::new();
    let mut verb = String::new();
    let mut before: Option<PathBuf> = None;
    let mut after: Option<PathBuf> = None;
    let mut pngs: Vec<PathBuf> = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--label" => label = rest.next().cloned().unwrap_or_default(),
            "--verb" => verb = rest.next().cloned().unwrap_or_default(),
            "--before" => before = rest.next().map(PathBuf::from),
            "--after" => after = rest.next().map(PathBuf::from),
            // Named rather than left to the catch-all, so the hand that
            // reaches for it is told where the board is read instead of
            // "unknown option".
            "--open" => {
                return Err(
                    "shots add: --open is gone — the board is read in one window, \
                            already open or opened once with `cargo xtask shots open`"
                        .to_string(),
                );
            }
            other if other.starts_with("--") => {
                return Err(format!("shots add: unknown flag {other}"));
            }
            other => pngs.push(PathBuf::from(other)),
        }
    }
    // Half a comparison is not one, and the halves must not arrive as
    // two runs by accident: the whole point of the pair is that they are
    // looked at together.
    let (page, count) = match (before, after) {
        (Some(before), Some(after)) if pngs.is_empty() => {
            (board::record_pair(&label, &verb, &before, &after)?, 2)
        }
        (None, None) => (board::record(&label, &verb, &pngs)?, pngs.len()),
        _ => {
            return Err(
                "shots add: --before and --after go together, and take no other pictures"
                    .to_string(),
            );
        }
    };
    println!(
        "board: {} (+{count} shot(s) in seat {} under \"{}\")",
        board::shown(&page),
        board::seat_here(),
        label
    );
    // Where these pictures are read now — in place of the `--open` this
    // line replaces. That flag was the second door onto the board, and
    // between the two of them the windows piled up until nobody could
    // say which one was the board as it stood.
    if let Some(board) = page.parent() {
        println!("board: {}", window::how_to_see_it(board));
    }
    Ok(())
}

fn prune(args: &[String]) -> Result<(), String> {
    let mut seats: Vec<String> = Vec::new();
    let mut every = false;
    let mut label = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--seat" => match rest.next().map(String::as_str) {
                // The sweep is a word rather than a missing flag: a
                // prune that reached every seat because nobody typed
                // anything is the one mistake this command can make
                // that another session pays for.
                Some("all") => every = true,
                Some(seat) => seats.push(seat.to_string()),
                None => return Err("shots prune: --seat wants a letter, or `all`".to_string()),
            },
            "--label" => match rest.next() {
                Some(named) => label = Some(named.clone()),
                None => return Err("shots prune: --label wants a run's name".to_string()),
            },
            other => return Err(format!("shots prune: unknown argument {other}")),
        }
    }
    if seats.is_empty() && !every {
        seats.push(board::seat_here());
    }
    let whose = if every {
        sweep::Whose::Everything
    } else {
        sweep::Whose::Seats(seats.clone())
    };
    let (gone, page) = sweep::prune(&sweep::Scope {
        whose,
        label: label.clone(),
    })?;
    println!(
        "board: {} (-{gone} run(s) {}{})",
        board::shown(&page),
        if every {
            "from every seat".to_string()
        } else {
            format!("from seat {}", seats.join(", "))
        },
        label.map_or(String::new(), |label| format!(" named \"{label}\"")),
    );
    Ok(())
}

/// Milliseconds since the epoch as `YYYY-MM-DD HH:MMZ`.
///
/// The page has a calendar of its own and shows local time; this is for
/// the listing, where a bare epoch is a number nobody can read. UTC, and
/// the `Z` says so — the two readings of one run differ by the offset,
/// and a listing that looked local while the page was would be worse
/// than one that plainly is not. The conversion is Howard Hinnant's
/// `civil_from_days`, which is the whole job once the era arithmetic is
/// written down; xtask carries std alone (CLAUDE.md 技術スタック).
fn stamp(millis: u128) -> String {
    let secs = (millis / 1000) as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = era * 400 + yoe + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::stamp;

    /// Dates the era arithmetic gets wrong when it is written from
    /// memory: a leap day, the turn of a century that is not a leap
    /// year, and the turn of one that is.
    #[test]
    fn a_listing_reads_as_a_date() {
        assert_eq!(stamp(0), "1970-01-01 00:00Z");
        assert_eq!(stamp(1_709_164_800_000), "2024-02-29 00:00Z");
        assert_eq!(stamp(951_782_400_000), "2000-02-29 00:00Z");
        assert_eq!(stamp(4_107_542_400_000), "2100-03-01 00:00Z");
        // UTC, not the clock on the wall: this run was taken at 02:12
        // local on the 22nd, nine hours ahead.
        assert_eq!(stamp(1_787_332_329_346), "2026-08-21 17:12Z");
    }
}
