//! `cargo xtask shots <add|open|list|path>`.

use std::path::PathBuf;

use super::board;

const USAGE: &str = "\
cargo xtask shots <command>

  add --label \"<what these pictures show>\" [--verb <v>] [--open] <png>...
      Put pictures on the board under a name, and rebuild the page. The
      seat is not asked for: it is read from the working directory, so a
      run always says which tree took it. --label is required — several
      pictures under no name leave nobody able to say which file was
      which change.

  open        Open the board in a window of its own.
  list        One line per run: when, seat, count, label.
  path        Where the page is.
";

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("add") => add(&args[1..]),
        Some("open") => {
            let page = board::board_dir()?.join("index.html");
            if !page.exists() {
                return Err(format!(
                    "no board yet at {} — put something on it with `cargo xtask shots add`",
                    board::shown(&page)
                ));
            }
            open(&page)
        }
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

fn add(args: &[String]) -> Result<(), String> {
    let mut label = String::new();
    let mut verb = String::new();
    let mut want_open = false;
    let mut pngs: Vec<PathBuf> = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--label" => label = rest.next().cloned().unwrap_or_default(),
            "--verb" => verb = rest.next().cloned().unwrap_or_default(),
            "--open" => want_open = true,
            other if other.starts_with("--") => {
                return Err(format!("shots add: unknown flag {other}"));
            }
            other => pngs.push(PathBuf::from(other)),
        }
    }
    let page = board::record(&label, &verb, &pngs)?;
    println!(
        "board: {} (+{} shot(s) in seat {} under \"{}\")",
        board::shown(&page),
        pngs.len(),
        board::seat_here(),
        label
    );
    if want_open {
        open(&page)?;
    }
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

/// Opens the board in a window of its own.
///
/// A browser in `--app` mode is a bare window — no tabs, no address bar —
/// which is what the board wants to be. The system image viewers are the
/// alternative and they smooth what they magnify, so they cannot answer
/// the question the board exists for. Falling back to the ordinary
/// handler is still better than nothing: a tab in the default browser
/// zooms just as exactly, it merely brings its furniture along.
fn open(page: &std::path::Path) -> Result<(), String> {
    let url = format!("file:///{}", page.display().to_string().replace('\\', "/"));
    let app = format!("--app={url}");
    for (program, args) in browsers(&app, &url) {
        if std::process::Command::new(program)
            .args(&args)
            .spawn()
            .is_ok()
        {
            println!("opened {} with {program}", board::shown(page));
            return Ok(());
        }
    }
    Err(format!(
        "found no browser to open {}: open it by hand",
        page.display()
    ))
}

/// Candidates in order of preference, per OS. Named rather than searched
/// for: xtask carries std alone, and the differences between the three
/// targets belong in code, not in parallel scripts (CLAUDE.md ビルド・テスト).
fn browsers(app: &str, url: &str) -> Vec<(&'static str, Vec<String>)> {
    if cfg!(windows) {
        vec![
            (
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                vec![app.to_string()],
            ),
            (
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                vec![app.to_string()],
            ),
            (
                "cmd",
                vec![
                    "/c".to_string(),
                    "start".to_string(),
                    String::new(),
                    url.to_string(),
                ],
            ),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            (
                "open",
                vec![
                    "-na".to_string(),
                    "Google Chrome".to_string(),
                    "--args".to_string(),
                    app.to_string(),
                ],
            ),
            ("open", vec![url.to_string()]),
        ]
    } else {
        vec![
            ("google-chrome", vec![app.to_string()]),
            ("chromium", vec![app.to_string()]),
            ("xdg-open", vec![url.to_string()]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{browsers, stamp};

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

    /// Every candidate carries the board, one way or the other: a bare
    /// window where the browser offers one, the plain URL where it does
    /// not.
    #[test]
    fn every_candidate_is_handed_the_board() {
        let candidates = browsers("--app=file:///b/index.html", "file:///b/index.html");
        assert!(!candidates.is_empty());
        for (_, args) in candidates {
            assert!(
                args.iter().any(|a| a.contains("file:///b/index.html")),
                "a candidate was handed no board: {args:?}"
            );
        }
    }
}
