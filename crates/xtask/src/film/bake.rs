//! `film bake`: a scene's sides into one APNG each, on one clock — the
//! setup cut, each side's time counted from its first frame after it, a
//! `sync` caption's first change laid at one moment on every side, and
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
    // Slowed and synced as the sides were shot: the scene each side keeps, which `same_scene` holds to one.
    let shot_from = scene::parse(&same_scene(&dir, &sides)?)?;
    let slow = u64::from(shot_from.slow);
    let mut all: Vec<Vec<Row>> = sides
        .iter()
        .map(|side| rows(&dir.join(side.name), &captions))
        .collect::<Result<_, String>>()?;
    for &caption in &shot_from.syncs {
        let moments = sides
            .iter()
            .zip(&all)
            .map(|(side, rows)| {
                let picture_height = picture_height(&dir.join(side.name))?;
                first_change(rows, caption, picture_height, image)?.ok_or_else(|| {
                    format!(
                        "side {}: nothing on screen changed under `{}`, the caption its `sync` lays \
                         the sides on",
                        side.name, captions[caption]
                    )
                })
            })
            .collect::<Result<Vec<usize>, String>>()?;
        let delays = line_up(&mut all, &moments);
        let said: Vec<String> = sides
            .iter()
            .zip(delays)
            .map(|(side, by)| format!("{} {by} ms later", side.name))
            .collect();
        println!(
            "film: `{}` laid on its first change — {}",
            captions[caption],
            said.join(", ")
        );
    }
    let runs: Vec<(&Side, Vec<Row>)> = sides.into_iter().zip(all).collect();
    let end = runs
        .iter()
        .filter_map(|(_, rows)| rows.last().map(|row| row.ms))
        .max()
        .unwrap_or_default();
    for (side, rows) in &runs {
        let out = dir.join(format!("{}.png", side.name));
        let picture_height = picture_height(&dir.join(side.name))?;
        let (frames, ms) = bake(rows, picture_height, end, slow, &out)?;
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
/// The scene every side was shot from — one text for all, or which side
/// was shot from another.
fn same_scene(dir: &Path, sides: &[&Side]) -> Result<String, String> {
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
        None => Ok(texts[0].1.clone()),
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

/// A row's frame, decoded.
fn image(row: &Row) -> Result<Image, String> {
    let bytes = std::fs::read(&row.file).map_err(|e| format!("{}: {e}", row.file.display()))?;
    crate::png::decode(&bytes).map_err(|e| format!("{}: {e}", row.file.display()))
}

/// The row `caption`'s first change on screen is grabbed in: the first
/// under it whose picture above the band is not the row's before it. None
/// when nothing changed under it.
fn first_change(
    rows: &[Row],
    caption: usize,
    picture_height: u32,
    image: impl Fn(&Row) -> Result<Image, String>,
) -> Result<Option<usize>, String> {
    let Some(start) = rows.iter().position(|row| row.caption == caption) else {
        return Ok(None);
    };
    let from = start.max(1);
    let mut before = image(&rows[from - 1])?;
    for (index, row) in rows.iter().enumerate().skip(from) {
        if row.caption != caption {
            break;
        }
        let next = image(row)?;
        if !same_size(&before, &next) || changed(&before, &next, picture_height).is_some() {
            return Ok(Some(index));
        }
        before = next;
    }
    Ok(None)
}

/// Each side's rows from its `moment` on, laid later by what puts the
/// moment where the latest side's is; says by how much, side by side. The
/// row before it stands the longer, and it shows what was on screen all
/// that while: nothing changed until the moment.
fn line_up(sides: &mut [Vec<Row>], moments: &[usize]) -> Vec<u64> {
    let at = sides
        .iter()
        .zip(moments)
        .map(|(rows, moment)| rows[*moment].ms)
        .max()
        .unwrap_or_default();
    sides
        .iter_mut()
        .zip(moments)
        .map(|(rows, moment)| {
            let by = at - rows[*moment].ms;
            for row in &mut rows[*moment..] {
                row.ms += by;
            }
            by
        })
        .collect()
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
/// `slow` stretches the shot's clock before the film's is laid over it: a
/// film `slow` times slower picks its frames that much finer.
fn bake(
    rows: &[Row],
    picture_height: u32,
    end: u64,
    slow: u64,
    out: &Path,
) -> Result<(usize, u64), String> {
    let times: Vec<u64> = rows.iter().map(|row| row.ms * slow).collect();
    let end = end * slow;
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
    use std::path::PathBuf;

    use super::{Row, first_change, line_up, same_frame, timeline};
    use crate::png::{decode, rgba};

    fn row(ms: u64, caption: usize, file: &str) -> Row {
        Row {
            ms,
            file: PathBuf::from(file),
            caption,
        }
    }

    /// The first change under the caption is the moment, whatever changed
    /// under the caption before it or in the band alone.
    #[test]
    fn a_sync_caption_is_found_at_its_first_change_on_screen() {
        let picture = |file: &str| {
            let (dot, band) = match file {
                "moved" => ((1, 1), (5, 4)),
                "renumbered" => ((0, 0), (4, 4)),
                _ => ((0, 0), (5, 4)),
            };
            decode(&rgba(6, 5, |x, y| {
                if (x, y) == dot || (x, y) == band {
                    [200, 0, 0, 255]
                } else {
                    [9, 9, 9, 255]
                }
            }))
            .map_err(|e| e.to_string())
        };
        let image = |r: &Row| picture(r.file.to_str().unwrap_or_default());
        let rows = [
            row(0, 1, "moved"),
            row(40, 1, "still"),
            row(80, 2, "still"),
            row(120, 2, "renumbered"),
            row(160, 2, "moved"),
            row(200, 3, "moved"),
        ];
        assert_eq!(first_change(&rows, 2, 4, image).expect("reads"), Some(4));
        assert_eq!(
            first_change(&rows, 1, 4, image).expect("reads"),
            Some(1),
            "the first row past the setup is compared with the next"
        );
        assert_eq!(first_change(&rows, 3, 4, image).expect("reads"), None);
        assert_eq!(first_change(&rows, 9, 4, image).expect("reads"), None);
    }

    /// The earlier side waits on the row before its moment; what comes
    /// after keeps its own pace.
    #[test]
    fn the_sides_are_laid_on_the_latest_moment() {
        let mut sides = vec![
            vec![
                row(0, 1, ""),
                row(40, 1, ""),
                row(500, 2, ""),
                row(540, 2, ""),
            ],
            vec![row(0, 1, ""), row(40, 1, ""), row(900, 2, "")],
        ];
        assert_eq!(line_up(&mut sides, &[2, 2]), [400, 0]);
        let ms: Vec<Vec<u64>> = sides
            .iter()
            .map(|rows| rows.iter().map(|r| r.ms).collect())
            .collect();
        assert_eq!(ms, [vec![0, 40, 900, 940], vec![0, 40, 900]]);
    }

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
