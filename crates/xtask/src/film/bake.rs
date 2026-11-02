//! `film bake`: a scene's sides into one APNG each, on one clock — the
//! setup cut, each side's time counted from its first frame after it, and
//! every side as long as the longest — so a before and an after set side
//! by side on the board play in step.

use std::path::{Path, PathBuf};

use super::changes::{changed, same_size};
use super::shoot::{CAPTIONS, SCENE_COPY};
use super::{SIDES, Side, scene};
use crate::png::Image;

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
        let picture_height = picture_height(&dir.join(side.name))?;
        let (frames, ms) = bake(rows, picture_height, end, &out)?;
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

/// The height of the picture of the window atop a side's frames: the band
/// under it is the component's own — the side's tag, the caption by its
/// number in frames.tsv, and the frame's number, which every frame changes.
/// The component's `lens` stands the tiles in one row, as tall as the
/// tallest view, with the band right under it.
pub(super) fn picture_height(side: &Path) -> Result<u32, String> {
    let path = side.join(SCENE_COPY);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let scene = scene::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(scene.views.iter().map(|view| view[3]).max().unwrap_or(0) * scene.scale)
}

/// The side as an APNG at `out`; says how many frames and milliseconds.
fn bake(rows: &[Row], picture_height: u32, end: u64, out: &Path) -> Result<(usize, u64), String> {
    let times: Vec<u64> = rows.iter().map(|row| row.ms).collect();
    let files: Vec<Vec<u8>> = rows
        .iter()
        .map(|row| std::fs::read(&row.file).map_err(|e| format!("{}: {e}", row.file.display())))
        .collect::<Result<_, String>>()?;
    // A frame grabbed while nothing moved stands as the one kept before it,
    // whose number the band keeps showing.
    let mut picked: Vec<(usize, u64)> = Vec::new();
    let mut kept: Option<(Image, usize)> = None;
    for (index, ms) in timeline(&times, end, STEP_MS, HOLD_MS) {
        let image = crate::png::decode(&files[index])
            .map_err(|e| format!("{}: {e}", rows[index].file.display()))?;
        let caption = rows[index].caption;
        let still = kept.as_ref().is_some_and(|(before, before_caption)| {
            same_frame((before, *before_caption), (&image, caption), picture_height)
        });
        match picked.last_mut() {
            Some((_, total)) if still => *total += ms,
            _ => {
                picked.push((index, ms));
                kept = Some((image, caption));
            }
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

/// Whether the `next` frame shows what the `before` one does: the picture
/// above the band, under the caption of the same number. The band's frame
/// number is all they may differ in.
fn same_frame(before: (&Image, usize), next: (&Image, usize), picture_height: u32) -> bool {
    before.1 == next.1
        && same_size(before.0, next.0)
        && changed(before.0, next.0, picture_height).is_none()
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
    use super::{same_frame, timeline};
    use crate::png::{decode, rgba};

    /// Four rows of picture over one of band: a frame whose number alone
    /// moved is the frame before; a new caption, a moved picture or another
    /// size is a frame of its own.
    #[test]
    fn a_frame_differing_in_the_band_alone_is_the_frame_before() {
        let marked = |at: (u32, u32)| {
            decode(&rgba(6, 5, |x, y| {
                if (x, y) == at {
                    [200, 0, 0, 255]
                } else {
                    [9, 9, 9, 255]
                }
            }))
            .expect("decodes")
        };
        let plain = decode(&rgba(6, 5, |_, _| [9, 9, 9, 255])).expect("decodes");
        let renumbered = marked((4, 4));
        let moved = marked((1, 1));
        let wider = decode(&rgba(7, 5, |_, _| [9, 9, 9, 255])).expect("decodes");
        assert!(same_frame((&plain, 1), (&renumbered, 1), 4));
        assert!(!same_frame((&plain, 1), (&renumbered, 2), 4));
        assert!(!same_frame((&plain, 1), (&moved, 1), 4));
        assert!(!same_frame((&plain, 1), (&wider, 1), 4));
    }

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
