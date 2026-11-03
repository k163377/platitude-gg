//! `cargo xtask shots diff` — where two pictures of one place differ, drawn
//! over the second.
//!
//! A one- or two-pixel move does not show when the two pictures stand side
//! by side: the reader compares the picture in front of them with the one
//! they remember (verify-ui SKILL.md「前後の画素の差は」). So the
//! difference is a picture of its own: the after, dimmed to grey; every
//! changed pixel that came out no darker (ink that arrived, on a dark
//! theme) in green, every one that came out darker (ink that left) in red;
//! and a yellow frame round each place where pixels changed, so a single
//! pixel is found at any magnification.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::board::shown;
use crate::png;

/// Changed pixels this close (in either direction) belong to one place:
/// the letters of one word, the two edges of one moved line.
const GATHER: u32 = 4;

/// The frame stands this far outside the place it frames, so it never
/// covers a changed pixel of its own place.
const FRAME_AIR: u32 = 2;

/// Places named in the summary line; the picture shows them all.
const NAMED: usize = 12;

// Not the UI's tokens: a page for a person reading a difference, coloured
// to stand out on the dimmed picture.
const LIGHTER: [u8; 4] = [64, 224, 96, 255];
const DARKER: [u8; 4] = [240, 64, 64, 255];
const FRAME: [u8; 4] = [250, 214, 40, 255];

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let asked = parse(args)?;
    let before = read(&asked.before)?;
    let after = read(&asked.after)?;
    let diff = compare(&before, &after)?;
    let out = asked.out.unwrap_or_else(|| beside(&asked.after));
    std::fs::write(&out, picture(&after, &diff))
        .map_err(|e| format!("could not write {}: {e}", shown(&out)))?;
    let named: Vec<String> = diff.places.iter().take(NAMED).map(named).collect();
    let more = diff.places.len().saturating_sub(NAMED);
    println!(
        "diff: {} ({} pixel(s) changed in {} place(s){}{})",
        shown(&out),
        diff.changed,
        diff.places.len(),
        if named.is_empty() {
            String::new()
        } else {
            format!(": {}", named.join("; "))
        },
        if more > 0 {
            format!("; {more} more")
        } else {
            String::new()
        }
    );
    Ok(())
}

struct Asked {
    before: PathBuf,
    after: PathBuf,
    out: Option<PathBuf>,
}

fn parse(args: &[String]) -> Result<Asked, String> {
    let mut pictures = Vec::new();
    let mut out = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--out" => {
                out = Some(PathBuf::from(
                    rest.next().ok_or("shots diff: --out needs a path")?,
                ));
            }
            other if other.starts_with("--") => {
                return Err(format!("shots diff: unknown flag {other}"));
            }
            other => pictures.push(PathBuf::from(other)),
        }
    }
    match <[PathBuf; 2]>::try_from(pictures) {
        Ok([before, after]) => Ok(Asked { before, after, out }),
        Err(_) => Err("shots diff: give the before and the after, in that order".to_string()),
    }
}

fn read(path: &Path) -> Result<png::Image, String> {
    let file = std::fs::read(path).map_err(|e| format!("could not read {}: {e}", shown(path)))?;
    png::decode(&file).map_err(|e| format!("{}: {e}", shown(path)))
}

/// What one pixel did between the two pictures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Change {
    Same,
    /// Changed, and no darker: a change of hue at one lightness counts
    /// here, so every changed pixel is drawn in one of the two colours.
    Lighter,
    Darker,
}

pub(super) struct Diff {
    width: u32,
    pub(super) changes: Vec<Change>,
    pub(super) changed: usize,
    /// Each place's changed pixels' bounds: left, top, right, bottom, all
    /// inclusive.
    pub(super) places: Vec<[u32; 4]>,
}

impl Diff {
    pub(super) fn at(&self, x: u32, y: u32) -> Change {
        self.changes[y as usize * self.width as usize + x as usize]
    }
}

/// Rec. 601 luma, times a thousand: which way a pixel went.
fn luma(pixel: [u8; 4]) -> u32 {
    299 * u32::from(pixel[0]) + 587 * u32::from(pixel[1]) + 114 * u32::from(pixel[2])
}

pub(super) fn compare(before: &png::Image, after: &png::Image) -> Result<Diff, String> {
    if (before.width, before.height) != (after.width, after.height) {
        return Err(format!(
            "shots diff: the before is {}x{} and the after is {}x{} — a difference is \
             read off one place at one size, so cut both to the same region first \
             (`shots crop --scale 1`)",
            before.width, before.height, after.width, after.height
        ));
    }
    let (width, height) = (after.width, after.height);
    let mut changes = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let (was, is) = (before.pixel(x, y), after.pixel(x, y));
            changes.push(if was == is {
                Change::Same
            } else if luma(is) >= luma(was) {
                Change::Lighter
            } else {
                Change::Darker
            });
        }
    }
    let changed = changes.iter().filter(|c| **c != Change::Same).count();
    let mut diff = Diff {
        width,
        changes,
        changed,
        places: Vec::new(),
    };
    diff.places = places(&diff, height);
    Ok(diff)
}

/// The places changed pixels gather into: a pixel joins every changed
/// pixel within `GATHER` of it, and each gathering is framed by the bounds
/// of its own changed pixels.
fn places(diff: &Diff, height: u32) -> Vec<[u32; 4]> {
    let width = diff.width;
    let index = |x: u32, y: u32| y as usize * width as usize + x as usize;
    let mut seen = vec![false; diff.changes.len()];
    let mut found = Vec::new();
    for start_y in 0..height {
        for start_x in 0..width {
            if seen[index(start_x, start_y)] || diff.at(start_x, start_y) == Change::Same {
                continue;
            }
            seen[index(start_x, start_y)] = true;
            let mut bounds = [start_x, start_y, start_x, start_y];
            let mut queue = VecDeque::from([(start_x, start_y)]);
            while let Some((x, y)) = queue.pop_front() {
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x),
                    bounds[3].max(y),
                ];
                for near_y in y.saturating_sub(GATHER)..=(y + GATHER).min(height - 1) {
                    for near_x in x.saturating_sub(GATHER)..=(x + GATHER).min(width - 1) {
                        let at = index(near_x, near_y);
                        if !seen[at] && diff.at(near_x, near_y) != Change::Same {
                            seen[at] = true;
                            queue.push_back((near_x, near_y));
                        }
                    }
                }
            }
            found.push(bounds);
        }
    }
    found
}

/// The after, dimmed to grey, with the changed pixels in their colours and
/// each place framed.
pub(super) fn picture(after: &png::Image, diff: &Diff) -> Vec<u8> {
    let (width, height) = (after.width, after.height);
    let frames = framed(&diff.places, width, height);
    png::rgba(width, height, |x, y| {
        match diff.at(x, y) {
            Change::Lighter => return LIGHTER,
            Change::Darker => return DARKER,
            Change::Same => {}
        }
        if frames[y as usize * width as usize + x as usize] {
            return FRAME;
        }
        // Two fifths of the after's own lightness: the picture stays
        // readable under the colours, and nothing in it competes with them.
        let grey = (luma(after.pixel(x, y)) * 2 / 5 / 1000) as u8;
        [grey, grey, grey, 255]
    })
}

/// Which pixels a frame passes over: one pixel wide, `FRAME_AIR` outside
/// each place, held inside the picture.
fn framed(places: &[[u32; 4]], width: u32, height: u32) -> Vec<bool> {
    let mut on = vec![false; width as usize * height as usize];
    let mut set = |x: u32, y: u32| on[y as usize * width as usize + x as usize] = true;
    for [left, top, right, bottom] in places {
        let (x0, y0) = (
            left.saturating_sub(FRAME_AIR),
            top.saturating_sub(FRAME_AIR),
        );
        let (x1, y1) = (
            (right + FRAME_AIR).min(width - 1),
            (bottom + FRAME_AIR).min(height - 1),
        );
        for x in x0..=x1 {
            set(x, y0);
            set(x, y1);
        }
        for y in y0..=y1 {
            set(x0, y);
            set(x1, y);
        }
    }
    on
}

fn named(place: &[u32; 4]) -> String {
    let [left, top, right, bottom] = *place;
    format!("{left},{top} {}x{}", right - left + 1, bottom - top + 1)
}

/// Beside the after: the difference belongs to the picture it is drawn
/// over, and a second diff of the same pair replaces the first.
fn beside(after: &Path) -> PathBuf {
    let stem = after.file_stem().map_or_else(
        || "shot".to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    after.with_file_name(format!("{stem}-diff.png"))
}

#[cfg(test)]
mod tests;
