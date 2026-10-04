//! Where the board lives, what a run writes into it, and how the runs
//! come back out.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Run, Shot, sweep};
use crate::seats::{SEATS, worktree_root};

/// Tabs and newlines are stripped from every value written (`one_line`),
/// so the format has no escape rules — xtask has no JSON library.
const SEP: char = '\t';

/// `.shots/` beside the primary checkout's `.git`: `--git-common-dir`
/// resolves every seat to one board. Without `--path-format=absolute`
/// the primary checkout answers a bare `.git`.
pub(super) fn board_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    board_dir_of(&cwd.to_string_lossy().replace('\\', "/"))
}

/// The board of the repository `dir` is in.
fn board_dir_of(dir: &str) -> Result<PathBuf, String> {
    let common = crate::subprocess::git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .ok_or("not in a git repository: the board lives beside the primary checkout")?;
    let common = Path::new(&common);
    let root = common
        .parent()
        .ok_or_else(|| format!("{} has no parent to hold the board", common.display()))?;
    Ok(root.join(".shots"))
}

/// A board path as written out, in forward slashes: `git_query` answers
/// with them and `Path::join` adds the platform's, so a built path would
/// read half one way and half the other.
pub(crate) fn shown(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// The tree this process is running in: a roster letter, or `main`.
/// Read from the working directory rather than passed in: a caller that
/// has to say which seat it is will eventually forget.
pub(super) fn seat_here() -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return "?".to_string();
    };
    let cwd = cwd.to_string_lossy().replace('\\', "/");
    let Some(root) = worktree_root(&cwd) else {
        return "main".to_string();
    };
    // A tree outside the roster is named truthfully too.
    root.rsplit('/').next().unwrap_or("?").to_string()
}

/// Why a picture taken here may not go on the board; None from a roster
/// seat. A run leaves the board when its seat's work does (`sweep`),
/// which only happens to a roster letter — a run from anywhere else would
/// stand for good.
fn not_a_seat(seat: &str) -> Option<String> {
    if SEATS.contains(&seat) {
        return None;
    }
    let here = match seat {
        "main" => "this is the primary checkout".to_string(),
        "?" => "this working directory cannot be read".to_string(),
        tree => format!("this is the worktree '{tree}', which the roster does not name"),
    };
    Some(format!(
        "the board takes pictures from a seat a-f, and {here}. Nothing would ever take \
         such a run off the board again — a run leaves when its seat's work does — so \
         the picture is taken in a seat: `cargo xtask seat` hands one over, and the \
         build the picture comes from belongs there too (CLAUDE.md ビルド・テスト)."
    ))
}

/// Puts one run on the board and rebuilds the page, answering where the
/// page is. An empty label is refused here, so the rule holds for
/// `verify-ui`'s own calls too.
pub(crate) fn record(label: &str, verb: &str, pngs: &[PathBuf]) -> Result<PathBuf, String> {
    record_with(label, verb, pngs, &[], false)
}

/// Before on the left, after on the right, read abreast under one name
/// (`Run::side_by_side`).
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

/// [`record_pair`] for more than two: a part's states in a row, each
/// under its caption.
pub(crate) fn record_abreast(
    label: &str,
    verb: &str,
    parts: &[(String, PathBuf)],
) -> Result<PathBuf, String> {
    let pngs: Vec<PathBuf> = parts.iter().map(|(_, png)| png.clone()).collect();
    let captions: Vec<String> = parts.iter().map(|(cap, _)| cap.clone()).collect();
    record_with(label, verb, &pngs, &captions, true)
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
        return Err(NEEDS_A_NAME.to_string());
    }
    if pngs.is_empty() {
        return Err("a run needs at least one png".to_string());
    }
    let seat = seat_here();
    if let Some(refusal) = not_a_seat(&seat) {
        return Err(refusal);
    }
    crate::seats::may_picture(&seat)?;
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
    let stem = format!("{at}-{seat}-{}", slug(&label));
    let mut run = Run {
        label,
        verb: one_line(verb),
        seat,
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
    // The new run first, then what it replaces: interrupted between, the
    // board holds one picture too many rather than none.
    sweep::supersede(&board, &run, &stem)?;
    sweep::rebuild(&board)
}

/// Every picture in a directory, onto the board under one name — how a
/// container's run reaches it: the pictures land in a bridged host
/// directory, and the seat they belong to is the one out here.
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

/// Every run on the board, oldest first — the order they went up is the
/// order they are read in
/// (verify-ui SKILL.md「board は上から下へ、載せた順に読む」).
/// A run file that cannot be read or parsed is skipped, leaving the rest
/// standing.
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
    out.sort_by_key(|run| run.at);
    out
}

/// The names of `seat`'s runs put up after `since` (milliseconds since
/// the epoch), oldest first, on the board of the repository `dir` is in:
/// what a landing would take off the board before anybody was asked
/// about it (`land`). A board that cannot be read holds none.
pub(crate) fn put_up_since(dir: &str, seat: &str, since: u128) -> Vec<String> {
    let Ok(board) = board_dir_of(dir) else {
        return Vec::new();
    };
    load_runs(&board.join("runs"))
        .into_iter()
        .filter(|run| run.seat == seat && run.at > since)
        .map(|run| run.label)
        .collect()
}

/// Seats add to the board concurrently, so each run is its own file,
/// written under a temporary name and renamed into place so a reader
/// gets a whole one.
fn write_run(runs: &Path, stem: &str, run: &Run) -> Result<(), String> {
    let mut text = String::new();
    for (key, value) in [
        ("label", run.label.as_str()),
        ("verb", run.verb.as_str()),
        ("seat", run.seat.as_str()),
    ] {
        text.push_str(&format!("{key}{SEP}{value}\n"));
    }
    text.push_str(&format!("at{SEP}{}\n", run.at));
    if run.side_by_side {
        text.push_str(&format!("abreast{SEP}1\n"));
    }
    // The caption goes last, so a run file without one still parses.
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

/// The inverse of `write_run`; None without a name, a time and a picture.
pub(super) fn parse_run(text: &str) -> Option<Run> {
    let mut run = Run {
        label: String::new(),
        verb: String::new(),
        seat: String::new(),
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
            Some("at") => run.at = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            Some("abreast") => run.side_by_side = parts.next() == Some("1"),
            Some("shot") => {
                let file = parts.next()?.to_string();
                let from = parts.next()?.to_string();
                let width = parts.next()?.parse().ok()?;
                let height = parts.next()?.parse().ok()?;
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

/// Out of the IHDR every PNG opens with — `png::decode` would inflate
/// every pixel to find out.
fn png_size(path: &Path) -> Result<(u32, u32), String> {
    use std::io::Read;
    let mut bytes = [0_u8; 24];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let header: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes[..8] != header {
        return Err(format!("{} is not a png", path.display()));
    }
    let word = |at: usize| -> u32 {
        u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    Ok((word(16), word(20)))
}

const NEEDS_A_NAME: &str =
    "a run needs --label \"<what to look at in these pictures, written in Japanese>\"";

/// A typed name, checked for the board: it holds one Japanese character
/// at least (verify-ui SKILL.md「ラベルは日本語で書く」). `verify-ui`'s
/// fallback name (its verb and argument) does not come through here.
pub(crate) fn written_label(text: &str) -> Result<String, String> {
    if one_line(text).is_empty() {
        return Err(NEEDS_A_NAME.to_string());
    }
    if !text.chars().any(reads_as_japanese) {
        return Err(format!(
            "--label {text:?}: a run's name is written in Japanese — it is read off the \
             board by the person the pictures were taken for, and it is the first thing \
             they read there (verify-ui SKILL.md「ラベルは日本語で書く」). Identifiers \
             inside the name keep their own spelling."
        ));
    }
    Ok(text.to_string())
}

/// Hiragana, katakana, or a kanji.
fn reads_as_japanese(ch: char) -> bool {
    matches!(ch, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')
}

/// ASCII only for a file name — the timestamp and seat already make the
/// name unique, so this is only a hint.
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

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;
