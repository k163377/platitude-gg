//! Where the board lives, what a run writes into it, and how the runs
//! come back out.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Run, Shot, sweep};
use crate::seats::worktree_root;

/// Field separator inside a run file. Tabs and newlines are stripped
/// from every value written, so the format has no escape rules to get
/// wrong — xtask carries no JSON library (CLAUDE.md 技術スタック).
const SEP: char = '\t';

/// The board directory: `.shots/` beside the primary checkout's `.git`.
///
/// `--git-common-dir` answers the *shared* git directory, so all six
/// seats and the primary checkout resolve to one board — which is what
/// puts a seat's pictures next to its neighbours' instead of stranding
/// each seat with a board only it can see. `--path-format=absolute`
/// keeps the answer from being relative to whichever directory asked
/// (measured: the primary checkout answers a bare `.git` without it).
pub(super) fn board_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    let cwd = cwd.to_string_lossy().replace('\\', "/");
    let common = crate::git_query(
        &cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .ok_or("not in a git repository: the board lives beside the primary checkout")?;
    let common = Path::new(&common);
    let root = common
        .parent()
        .ok_or_else(|| format!("{} has no parent to hold the board", common.display()))?;
    Ok(root.join(".shots"))
}

/// A board path as it is written out. `git_query` answers with forward
/// slashes and `Path::join` adds the platform's, so a path built from
/// both reads half one way and half the other; one convention is the
/// whole fix, and forward slashes are the one the rest of xtask already
/// speaks.
pub(crate) fn shown(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// The tree this process is running in: a roster letter, or `main`.
///
/// Taken from the working directory rather than asked for, because the
/// one thing the board must never do is attribute a picture to the wrong
/// seat — and a caller that has to remember to say which seat it is will
/// eventually forget (CLAUDE.md ビルド・テスト: seats are the unit of work).
pub(super) fn seat_here() -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return "?".to_string();
    };
    let cwd = cwd.to_string_lossy().replace('\\', "/");
    let Some(root) = worktree_root(&cwd) else {
        return "main".to_string();
    };
    // Whatever the directory is called, roster letter or not. A tree
    // outside the roster is worth naming truthfully rather than filing
    // under one of the six it is not.
    root.rsplit('/').next().unwrap_or("?").to_string()
}

/// The session this process was run by, empty when nothing said.
///
/// Claude Code puts the session id in the environment of everything it
/// runs and in the payload of every hook, so a run taken by a session
/// and the SessionEnd that ends it agree on one string without either
/// being told (CLAUDE.md ビルド・テスト: the seat is a session's, for as
/// long as the session lasts). Nothing here depends on it being there —
/// a run taken by hand in a terminal carries no session, and the sweeps
/// that go by session leave it alone.
pub(super) fn session_here() -> String {
    std::env::var("CLAUDE_CODE_SESSION_ID")
        .map(|id| one_line(&id))
        .unwrap_or_default()
}

/// Puts one run on the board and rebuilds the page, answering where the
/// page is. An empty label is refused here rather than at the command
/// line, so the rule holds for `verify-ui`'s own calls too.
pub(crate) fn record(label: &str, verb: &str, pngs: &[PathBuf]) -> Result<PathBuf, String> {
    let captions: Vec<String> = pngs.iter().map(|_| String::new()).collect();
    record_with(label, verb, pngs, &captions, false)
}

/// Two pictures of the same thing under one name, read abreast: the one
/// before the change on the left, the one after it on the right.
///
/// Shown one at a time they are not a comparison at all — the reader
/// holds the first in their head while looking at the second
/// (2026-08-22 ユーザー指示). One view, one magnifier, and the difference
/// is on the screen instead of in the memory.
pub(crate) fn record_pair(
    label: &str,
    verb: &str,
    before: &Path,
    after: &Path,
) -> Result<PathBuf, String> {
    record_with(
        label,
        verb,
        &[before.to_path_buf(), after.to_path_buf()],
        &["before".to_string(), "after".to_string()],
        true,
    )
}

fn record_with(
    label: &str,
    verb: &str,
    pngs: &[PathBuf],
    captions: &[String],
    side_by_side: bool,
) -> Result<PathBuf, String> {
    let label = one_line(label);
    if label.is_empty() {
        return Err("a run needs --label \"<what these pictures show>\"".to_string());
    }
    if pngs.is_empty() {
        return Err("a run needs at least one png".to_string());
    }
    let board = board_dir()?;
    let img = board.join("img");
    let runs = board.join("runs");
    for dir in [&img, &runs] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    }
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("the clock is before the epoch: {e}"))?
        .as_millis();
    let seat = seat_here();
    let stem = format!("{at}-{seat}-{}", slug(&label));
    let mut run = Run {
        label,
        verb: one_line(verb),
        seat,
        session: session_here(),
        at,
        side_by_side,
        shots: Vec::new(),
    };
    for (n, png) in pngs.iter().enumerate() {
        let from = png
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "shot.png".to_string());
        let (width, height) = png_size(png)?;
        let name = format!("{stem}-{n}-{from}");
        std::fs::copy(png, img.join(&name))
            .map_err(|e| format!("could not copy {}: {e}", png.display()))?;
        run.shots.push(Shot {
            file: format!("img/{name}"),
            from,
            caption: captions.get(n).cloned().unwrap_or_default(),
            width,
            height,
        });
    }
    write_run(&runs, &stem, &run)?;
    // The new run first, then what it replaces: interrupted between the
    // two the board holds one picture too many, where the other order
    // could leave it holding none.
    sweep::supersede(&board, &run, &stem)?;
    sweep::rebuild(&board)
}

/// Every picture in a directory, onto the board under one name.
///
/// How a run that happened somewhere else reaches the board: a container
/// leaves its pictures in a bridged host directory, and the seat they
/// belong to is the one out here, not the working directory they were
/// taken in.
pub(crate) fn record_dir(dir: &Path, label: &str, verb: &str) -> Result<PathBuf, String> {
    let mut shots: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("could not read {}: {e}", dir.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "png"))
        .collect();
    shots.sort();
    record(label, verb, &shots)
}

/// Every run on the board, newest first. A run file that cannot be read
/// or parsed is skipped rather than fatal: one bad file must not cost
/// the board every other picture on it.
pub(super) fn load_runs(runs: &Path) -> Vec<Run> {
    let Ok(entries) = std::fs::read_dir(runs) else {
        return Vec::new();
    };
    let mut out: Vec<Run> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "tsv"))
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|text| parse_run(&text))
        .collect();
    out.sort_by_key(|run| std::cmp::Reverse(run.at));
    out
}

/// Writes the run beside its neighbours under a name of its own.
///
/// Seats add to the board concurrently, so there is no shared file to
/// read-modify-write: each run is its own file, written to a temporary
/// name and renamed into place, which keeps a reader from ever seeing
/// half of one.
fn write_run(runs: &Path, stem: &str, run: &Run) -> Result<(), String> {
    let mut text = String::new();
    for (key, value) in [
        ("label", run.label.as_str()),
        ("verb", run.verb.as_str()),
        ("seat", run.seat.as_str()),
        ("session", run.session.as_str()),
    ] {
        text.push_str(&format!("{key}{SEP}{value}\n"));
    }
    text.push_str(&format!("at{SEP}{}\n", run.at));
    if run.side_by_side {
        text.push_str(&format!("abreast{SEP}1\n"));
    }
    // The caption goes last, so a run written before the board had one
    // parses here with every field it does carry still in its place.
    for shot in &run.shots {
        text.push_str(&format!(
            "shot{SEP}{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}\n",
            shot.file, shot.from, shot.width, shot.height, shot.caption
        ));
    }
    let final_path = runs.join(format!("{stem}.tsv"));
    let staging = runs.join(format!("{stem}.tsv.part"));
    std::fs::write(&staging, text)
        .map_err(|e| format!("could not write {}: {e}", staging.display()))?;
    std::fs::rename(&staging, &final_path)
        .map_err(|e| format!("could not place {}: {e}", final_path.display()))
}

/// The inverse of `write_run`. None when the file is missing what a run
/// is: a name, a time, and a picture.
pub(super) fn parse_run(text: &str) -> Option<Run> {
    let mut run = Run {
        label: String::new(),
        verb: String::new(),
        seat: String::new(),
        session: String::new(),
        at: 0,
        side_by_side: false,
        shots: Vec::new(),
    };
    for line in text.lines() {
        let mut parts = line.split(SEP);
        match parts.next() {
            Some("label") => run.label = parts.next().unwrap_or_default().to_string(),
            Some("verb") => run.verb = parts.next().unwrap_or_default().to_string(),
            Some("seat") => run.seat = parts.next().unwrap_or_default().to_string(),
            // Runs written before the board knew about sessions carry no
            // such line, and read back as belonging to no session — which
            // is what they are: nothing that ends can claim them.
            Some("session") => run.session = parts.next().unwrap_or_default().to_string(),
            Some("at") => run.at = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            Some("abreast") => run.side_by_side = parts.next() == Some("1"),
            Some("shot") => {
                let file = parts.next()?.to_string();
                let from = parts.next()?.to_string();
                let width = parts.next()?.parse().ok()?;
                let height = parts.next()?.parse().ok()?;
                // Absent in every run written before captions existed,
                // and empty is exactly what those runs mean.
                let caption = parts.next().unwrap_or_default().to_string();
                run.shots.push(Shot {
                    file,
                    from,
                    caption,
                    width,
                    height,
                });
            }
            _ => {}
        }
    }
    (!run.label.is_empty() && run.at > 0 && !run.shots.is_empty()).then_some(run)
}

/// A PNG's pixel dimensions, straight out of the IHDR that every PNG
/// opens with. Reading 24 bytes is the whole job — decoding one would
/// mean a dependency, and the board only needs to know how big to say
/// the picture is.
fn png_size(path: &Path) -> Result<(u32, u32), String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let header: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < 24 || bytes[..8] != header {
        return Err(format!("{} is not a png", path.display()));
    }
    let word = |at: usize| -> u32 {
        u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    Ok((word(16), word(20)))
}

/// A label reduced to something safe in a file name. Labels are written
/// in Japanese as often as not, and non-ASCII in a name is a portability
/// problem nobody needs — the timestamp and seat already make the name
/// unique, so this only has to be a hint.
fn slug(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "shot".to_string()
    } else {
        out
    }
}

/// One line of text: the run format separates on tabs and newlines, so
/// values may hold neither.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{Run, Shot, one_line, parse_run, slug};

    #[test]
    fn a_run_survives_the_round_trip() {
        let text = "label\tthe chip's badge\nverb\trow-card\nseat\ta\nsession\ts-1\n\
                    at\t1700000000000\nshot\timg/x.png\tapp.png\t1440\t900\n";
        let run = parse_run(text).expect("a whole run parses");
        assert_eq!(run.label, "the chip's badge");
        assert_eq!(run.seat, "a");
        assert_eq!(run.session, "s-1");
        assert_eq!(run.at, 1_700_000_000_000);
        assert_eq!(run.shots.len(), 1);
        assert_eq!(run.shots[0].width, 1440);
    }

    /// A before/after has to come back off the board as one thing: the
    /// two are read abreast, and each half keeps the word over it.
    #[test]
    fn a_pair_is_still_a_pair_after_a_rebuild() {
        let text = "label\tthe stopped landing\tseat\ta\nseat\ta\nat\t1700000000000\n\
                    abreast\t1\n\
                    shot\timg/x.png\tapp.png\t1440\t900\tbefore\n\
                    shot\timg/y.png\tapp.png\t1440\t900\tafter\n";
        let run = parse_run(text).expect("a whole run parses");
        assert!(run.side_by_side);
        assert_eq!(run.shots.len(), 2);
        assert_eq!(run.shots[0].caption, "before");
        assert_eq!(run.shots[1].caption, "after");
    }

    /// Every other run is read one picture at a time, and says so by
    /// carrying no such line — including the ones written before the
    /// board could put two pictures side by side.
    #[test]
    fn a_run_from_before_pairs_is_read_one_at_a_time() {
        let text = "label\tthe chip's badge\nseat\ta\nat\t1700000000000\n\
                    shot\timg/x.png\tapp.png\t1440\t900\n";
        let run = parse_run(text).expect("a whole run parses");
        assert!(!run.side_by_side);
        assert!(run.shots[0].caption.is_empty());
    }

    /// A run written before the board carried sessions is still a run —
    /// one that belongs to no session, which is the reading that keeps a
    /// session's sweep off it.
    #[test]
    fn a_run_from_before_sessions_belongs_to_none() {
        let text = "label\tthe chip's badge\nseat\ta\nat\t1700000000000\n\
                    shot\timg/x.png\tapp.png\t1440\t900\n";
        let run = parse_run(text).expect("a whole run parses");
        assert!(run.session.is_empty());
    }

    /// A file missing what a run *is* is skipped, not guessed at.
    #[test]
    fn a_run_without_a_picture_is_not_a_run() {
        assert!(parse_run("label\tnamed\nat\t1\n").is_none());
        assert!(parse_run("at\t1\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
        assert!(parse_run("label\tnamed\nshot\timg/x.png\tapp.png\t1\t1\n").is_none());
    }

    /// Japanese labels are the common case, and none of it may reach a
    /// file name.
    #[test]
    fn a_slug_keeps_to_ascii() {
        assert_eq!(slug("チップの余白"), "shot");
        assert_eq!(slug("chip padding: top-left"), "chip-padding-top-left");
        assert!(slug(&"x".repeat(200)).len() <= 40);
    }

    #[test]
    fn a_value_never_carries_a_separator() {
        assert_eq!(one_line("two\tlines\nhere"), "two lines here");
    }

    /// The board reads newest first, so a run's own time is what orders
    /// it — not the order the directory happened to hand them back.
    #[test]
    fn runs_carry_their_own_order() {
        let run = |at| Run {
            label: "x".to_string(),
            verb: String::new(),
            seat: "a".to_string(),
            session: String::new(),
            at,
            side_by_side: false,
            shots: vec![Shot {
                file: "img/x.png".to_string(),
                from: "app.png".to_string(),
                caption: String::new(),
                width: 1,
                height: 1,
            }],
        };
        let mut runs = [run(1), run(3), run(2)];
        runs.sort_by_key(|run| std::cmp::Reverse(run.at));
        assert_eq!(runs.iter().map(|r| r.at).collect::<Vec<_>>(), vec![3, 2, 1]);
    }
}
