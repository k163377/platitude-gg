//! `film bake`: a scene's sides into one APNG each, on one clock — the
//! setup cut, each side's time counted from its first frame after it, and
//! every side as long as the longest — so a before and an after set side
//! by side on the board play in step.

use std::path::{Path, PathBuf};

use super::shoot::{CAPTIONS, SCENE_COPY};
use super::{SIDES, Side, scene};

/// The clock the sides are put on, in milliseconds.
const STEP_MS: u64 = 40;

/// How long the last frame stands before the film starts again.
const HOLD_MS: u64 = 1600;

/// One grabbed frame, as the side's frames.tsv names it.
pub(super) struct Row {
    /// From the side's first frame past the setup.
    pub(super) ms: u64,
    pub(super) file: PathBuf,
    pub(super) caption: usize,
}

pub(super) fn run(tree: &Path, args: &[String]) -> Result<(), String> {
    let mut scene_file = None;
    let mut named: Vec<&'static Side> = Vec::new();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--sides" => {
                for name in it.next().ok_or("--sides needs a list")?.split(',') {
                    named.push(super::side(name)?);
                }
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other if scene_file.is_none() => scene_file = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let scene_file = scene_file.ok_or("film bake needs the scene file")?;
    let dir = super::scene_dir(tree, &scene_file)?;
    let captions = captions(&dir)?;
    let sides: Vec<&Side> = if named.is_empty() {
        SIDES
            .iter()
            .filter(|side| dir.join(side.name).join("frames.tsv").is_file())
            .collect()
    } else {
        named
    };
    if sides.is_empty() {
        return Err(format!("no side of {} is shot yet", dir.display()));
    }
    same_scene(&dir, &sides)?;
    let runs: Vec<(&Side, Vec<Row>)> = sides
        .iter()
        .map(|side| Ok((*side, rows(&dir.join(side.name), &captions)?)))
        .collect::<Result<_, String>>()?;
    let end = runs
        .iter()
        .filter_map(|(_, rows)| rows.last().map(|row| row.ms))
        .max()
        .unwrap_or_default();
    for (side, rows) in &runs {
        let out = dir.join(format!("{}.png", side.name));
        let (frames, ms) = bake(rows, end, &out)?;
        println!("film: {} — {frames} frames, {ms} ms", out.display());
    }
    println!("the last frame under each caption:");
    for (number, caption) in captions.iter().enumerate() {
        if scene::is_setup(caption) {
            continue;
        }
        println!("  {caption}");
        for (side, rows) in &runs {
            if let Some(row) = rows.iter().rev().find(|row| row.caption == number) {
                println!("    {}: {} ({} ms)", side.name, row.file.display(), row.ms);
            }
        }
    }
    Ok(())
}

pub(super) fn captions(dir: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(dir.join(CAPTIONS)).map_err(|e| {
        format!(
            "reading {}: {e} — shoot a side first",
            dir.join(CAPTIONS).display()
        )
    })?;
    Ok(text.lines().map(String::from).collect())
}

/// Two sides of one film were shot from one scene: a scene changed between
/// them shows that change as the difference.
fn same_scene(dir: &Path, sides: &[&Side]) -> Result<(), String> {
    let texts: Vec<(&str, String)> = sides
        .iter()
        .map(|side| {
            let path = dir.join(side.name).join(SCENE_COPY);
            std::fs::read_to_string(&path)
                .map(|text| (side.name, text))
                .map_err(|e| format!("reading {}: {e}", path.display()))
        })
        .collect::<Result<_, String>>()?;
    match texts.iter().find(|(_, text)| *text != texts[0].1) {
        Some((name, _)) => Err(format!(
            "side {name} was shot from another version of the scene than side {}: shoot them \
             again from the scene as it stands",
            texts[0].0
        )),
        None => Ok(()),
    }
}

/// A side's frames past the setup.
pub(super) fn rows(side: &Path, captions: &[String]) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = all_rows(side)?
        .into_iter()
        .filter(|row| {
            captions
                .get(row.caption)
                .is_some_and(|c| !scene::is_setup(c))
        })
        .collect();
    let start = rows
        .first()
        .ok_or_else(|| format!("{} holds no frame past the setup", side.display()))?
        .ms;
    for row in &mut rows {
        row.ms -= start;
    }
    Ok(rows)
}

/// Every row of a side's frames.tsv, in time order.
pub(super) fn all_rows(side: &Path) -> Result<Vec<Row>, String> {
    let list = side.join("frames.tsv");
    let text =
        std::fs::read_to_string(&list).map_err(|e| format!("reading {}: {e}", list.display()))?;
    let mut rows = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let cells: Vec<&str> = line.split('\t').collect();
        let (Some(name), Some(ms), Some(caption)) = (cells.first(), cells.get(1), cells.get(4))
        else {
            return Err(format!(
                "{}: a row short of its cells: {line}",
                list.display()
            ));
        };
        rows.push(Row {
            ms: ms
                .parse()
                .map_err(|_| format!("{}: {line}", list.display()))?,
            file: side.join(name),
            caption: caption
                .parse()
                .map_err(|_| format!("{}: {line}", list.display()))?,
        });
    }
    rows.sort_by_key(|row| row.ms);
    Ok(rows)
}

/// The side as an APNG at `out`; says how many frames and milliseconds.
fn bake(rows: &[Row], end: u64, out: &Path) -> Result<(usize, u64), String> {
    let times: Vec<u64> = rows.iter().map(|row| row.ms).collect();
    let files: Vec<Vec<u8>> = rows
        .iter()
        .map(|row| std::fs::read(&row.file).map_err(|e| format!("{}: {e}", row.file.display())))
        .collect::<Result<_, String>>()?;
    // A frame grabbed while nothing moved carries the one before's bytes.
    let mut picked: Vec<(usize, u64)> = Vec::new();
    for (index, ms) in timeline(&times, end, STEP_MS, HOLD_MS) {
        match picked.last_mut() {
            Some((last, total))
                if crate::png::image_data(&files[*last])?
                    == crate::png::image_data(&files[index])? =>
            {
                *total += ms;
            }
            _ => picked.push((index, ms)),
        }
    }
    let frames: Vec<crate::png::Frame> = picked
        .iter()
        .map(|(index, ms)| crate::png::Frame {
            file: &files[*index],
            ms: u32::try_from(*ms).unwrap_or(u32::MAX),
        })
        .collect();
    let film = crate::png::animate(&frames)?;
    std::fs::write(out, film).map_err(|e| format!("writing {}: {e}", out.display()))?;
    Ok((picked.len(), picked.iter().map(|(_, ms)| ms).sum()))
}

/// Which row stands at each step of the clock, and for how long: the last
/// one grabbed at or before the step, held while it stays the same. The
/// clock's last step is `end` itself, so the film's last row is on it
/// wherever it fell, and stands for `hold`. `times` is sorted and starts
/// at 0.
fn timeline(times: &[u64], end: u64, step: u64, hold: u64) -> Vec<(usize, u64)> {
    let mut ticks: Vec<u64> = (0..)
        .map(|k| k * step)
        .take_while(|tick| *tick < end)
        .collect();
    ticks.push(end);
    let mut out: Vec<(usize, u64)> = Vec::new();
    let mut at = 0usize;
    for (k, tick) in ticks.iter().enumerate() {
        while at + 1 < times.len() && times[at + 1] <= *tick {
            at += 1;
        }
        let lasts = ticks.get(k + 1).map_or(hold, |next| next - tick);
        match out.last_mut() {
            Some((index, ms)) if *index == at => *ms += lasts,
            _ => out.push((at, lasts)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::timeline;

    #[test]
    fn the_clock_holds_each_row_until_the_next_and_the_last_for_the_hold() {
        // Rows at 0, 100 and 130: the steps 0..80 stand on row 0, 120 on
        // row 1, 160 on row 2, which then holds.
        assert_eq!(
            timeline(&[0, 100, 130], 160, 40, 1600),
            [(0, 120), (1, 40), (2, 1600)]
        );
    }

    /// The longest side's last row lands between two steps of the clock:
    /// it is on the film all the same, for the hold.
    #[test]
    fn the_last_row_is_on_the_film_wherever_it_fell() {
        assert_eq!(
            timeline(&[0, 100, 170], 170, 40, 1600),
            [(0, 120), (1, 50), (2, 1600)]
        );
    }

    #[test]
    fn a_side_shorter_than_the_film_stands_on_its_last_row() {
        assert_eq!(
            timeline(&[0, 40], 200, 40, 1000),
            [(0, 40), (1, 160 + 1000)]
        );
        assert_eq!(timeline(&[0], 0, 40, 1600), [(0, 1600)]);
    }
}
