//! `cargo xtask shots crop` — the piece of a screenshot a judgement is
//! made on, cut out and magnified without smoothing.
//!
//! A session has no window to zoom, so the board's magnifying — whole
//! ratios, nearest neighbour — has to be in the file: a smoothed
//! enlargement invents the edge it was asked about, and a shrunk one
//! drops a five-pixel bar (verify-ui SKILL.md「目視は等倍以上で」).
//!
//! `png::rgba` stores every pixel as it is, so a crop is about as many
//! bytes as it has pixels.

use std::path::{Path, PathBuf};

use super::board::shown;
use crate::png;

/// Past this many pixels, whatever reads a crop fits it to a window or a
/// pane and shrinks it — the magnifying undone.
const CEILING: u64 = 16 << 20;

/// The ratio the skill asks for before a picture is judged on.
const SCALE: u32 = 3;

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let cut = parse(args)?;
    let file = std::fs::read(&cut.source)
        .map_err(|e| format!("could not read {}: {e}", shown(&cut.source)))?;
    let image = png::decode(&file).map_err(|e| format!("{}: {e}", shown(&cut.source)))?;
    let (wide, tall) = cut.at.measured(&image, cut.scale)?;
    let out = cut.out.unwrap_or_else(|| beside(&cut.source, &cut.at));
    let (left, top, scale) = (cut.at.x, cut.at.y, cut.scale);
    // Nearest neighbour at a whole ratio: every source pixel becomes a
    // square block of itself.
    let png = png::rgba(wide, tall, |x, y| {
        image.pixel(left + x / scale, top + y / scale)
    });
    std::fs::write(&out, &png).map_err(|e| format!("could not write {}: {e}", shown(&out)))?;
    println!(
        "crop: {} ({wide}x{tall} at {scale}x from {} {})",
        shown(&out),
        shown(&cut.source),
        cut.at.named()
    );
    Ok(())
}

struct Cut {
    source: PathBuf,
    at: Rect,
    scale: u32,
    out: Option<PathBuf>,
}

struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn named(&self) -> String {
        format!("{}:{}:{}:{}", self.x, self.y, self.width, self.height)
    }

    /// The crop's size, once it is inside the picture and under `CEILING`.
    fn measured(&self, image: &png::Image, scale: u32) -> Result<(u32, u32), String> {
        let (right, bottom) = (
            u64::from(self.x) + u64::from(self.width),
            u64::from(self.y) + u64::from(self.height),
        );
        if right > u64::from(image.width) || bottom > u64::from(image.height) {
            return Err(format!(
                "shots crop: {} runs outside a picture that is {}x{}",
                self.named(),
                image.width,
                image.height
            ));
        }
        let (wide, tall) = (
            u64::from(self.width) * u64::from(scale),
            u64::from(self.height) * u64::from(scale),
        );
        // Saturating: --scale is whatever anyone types.
        if wide.saturating_mul(tall) > CEILING {
            return Err(format!(
                "shots crop: {} at {scale}x comes to {wide}x{tall}, which is bigger than \
                 anything cut out to be looked at — whatever opens it will shrink it, and \
                 that undoes the magnifying. Cut a smaller piece, or magnify it less",
                self.named()
            ));
        }
        Ok((wide as u32, tall as u32))
    }
}

fn parse(args: &[String]) -> Result<Cut, String> {
    let mut source: Option<PathBuf> = None;
    let mut at: Option<Rect> = None;
    let mut scale = SCALE;
    let mut out: Option<PathBuf> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let wants = |flag: &str, given: Option<&String>| {
            given
                .cloned()
                .ok_or_else(|| format!("shots crop: {flag} wants a value"))
        };
        match arg.as_str() {
            "--at" => at = Some(rect(&wants("--at", rest.next())?)?),
            "--scale" => scale = whole(&wants("--scale", rest.next())?)?,
            "--out" => out = Some(PathBuf::from(wants("--out", rest.next())?)),
            other if other.starts_with("--") => {
                return Err(format!("shots crop: unknown flag {other}"));
            }
            other if source.is_none() => source = Some(PathBuf::from(other)),
            other => {
                return Err(format!(
                    "shots crop: one picture at a time, and {other} is a second"
                ));
            }
        }
    }
    Ok(Cut {
        source: source.ok_or_else(|| "shots crop: no picture to cut from".to_string())?,
        // Required: a crop of everything is just the picture.
        at: at.ok_or_else(|| "shots crop: --at wants x:y:width:height in pixels".to_string())?,
        scale,
        out,
    })
}

/// `x:y:width:height`, in the pixels of the picture being cut.
fn rect(text: &str) -> Result<Rect, String> {
    let numbers = text
        .split(':')
        .map(|part| {
            part.parse::<u32>()
                .map_err(|_| format!("shots crop: --at wants whole pixels, got {part:?}"))
        })
        .collect::<Result<Vec<u32>, String>>()?;
    let [x, y, width, height] = numbers[..] else {
        return Err(format!(
            "shots crop: --at wants x:y:width:height in pixels, got {text:?}"
        ));
    };
    if width == 0 || height == 0 {
        return Err(format!(
            "shots crop: --at asks for {width}x{height} of picture"
        ));
    }
    Ok(Rect {
        x,
        y,
        width,
        height,
    })
}

/// Whole numbers only: at a fractional ratio, nearest neighbour doubles
/// some rows and not others.
fn whole(text: &str) -> Result<u32, String> {
    match text.parse::<u32>() {
        Ok(0) | Err(_) => Err(format!(
            "shots crop: --scale wants a whole number of times, 1 or more, got {text:?} — \
             a fraction would have to interpolate, and then the picture is not the pixels"
        )),
        Ok(scale) => Ok(scale),
    }
}

/// Beside the picture, named for the region: two regions of one shot
/// keep apart, and the same region cut again replaces itself.
fn beside(source: &Path, at: &Rect) -> PathBuf {
    let stem = source.file_stem().map_or_else(
        || "shot".to_string(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    source.with_file_name(format!(
        "{stem}-crop-{}-{}-{}x{}.png",
        at.x, at.y, at.width, at.height
    ))
}

#[cfg(test)]
mod tests;
