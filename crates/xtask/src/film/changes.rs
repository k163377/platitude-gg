//! `film changes`: when a side's picture changed and where — what a film
//! shows at which moment, read without watching it.

use std::path::{Path, PathBuf};

use crate::png::Image;

pub(super) fn run(tree: &Path, args: &[String]) -> Result<(), String> {
    let mut scene_file = None;
    let mut side = super::side("now")?;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--side" => side = super::side(it.next().ok_or("--side needs a side")?)?,
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other if scene_file.is_none() => scene_file = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let scene_file = scene_file.ok_or("film changes needs the scene file")?;
    let dir = super::scene_dir(tree, &scene_file)?;
    let captions = super::bake::captions(&dir)?;
    let shots = dir.join(side.name);
    // The band's number changes every frame: only the picture above it is
    // compared, and a caption is told by its number.
    let picture_height = super::bake::picture_height(&shots)?;
    let mut before: Option<(Image, usize)> = None;
    for row in super::bake::all_rows(&shots)? {
        let bytes = std::fs::read(&row.file).map_err(|e| format!("{}: {e}", row.file.display()))?;
        let image =
            crate::png::decode(&bytes).map_err(|e| format!("{}: {e}", row.file.display()))?;
        let name = row.file.file_name().unwrap_or_default().to_string_lossy();
        let caption = captions.get(row.caption).map_or("", String::as_str);
        let said = match &before {
            Some((last, _)) if !same_size(last, &image) => {
                Some(format!("size {}x{}", image.width, image.height))
            }
            Some((last, last_caption)) => match changed(last, &image, picture_height) {
                Some([x0, y0, x1, y1]) => Some(format!("changed {x0},{y0} to {x1},{y1}")),
                None if *last_caption != row.caption => Some("the caption only".to_string()),
                None => None,
            },
            None => Some(format!("first, {}x{}", image.width, image.height)),
        };
        if let Some(said) = said {
            println!(
                "{:>6} ms  {name}  [{}] {caption}  {said}",
                row.ms, row.caption
            );
        }
        before = Some((image, row.caption));
    }
    Ok(())
}

pub(super) fn same_size(a: &Image, b: &Image) -> bool {
    (a.width, a.height) == (b.width, b.height)
}

/// The box around every pixel that differs in the top `height` rows,
/// inclusive; None when the two are the same picture there. Both are one
/// size.
pub(super) fn changed(a: &Image, b: &Image, height: u32) -> Option<[u32; 4]> {
    let mut found: Option<[u32; 4]> = None;
    for y in 0..a.height.min(height) {
        let (left, right) = (a.row(y), b.row(y));
        if left == right {
            continue;
        }
        let differs = |x: &u32| {
            let at = *x as usize * 4;
            left[at..at + 4] != right[at..at + 4]
        };
        let first = (0..a.width).find(differs)?;
        let last = (0..a.width).rev().find(differs)?;
        found = Some(match found {
            Some([x0, y0, x1, _]) => [x0.min(first), y0, x1.max(last), y],
            None => [first, y, last, y],
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::changed;
    use crate::png::{decode, rgba};

    #[test]
    fn the_box_holds_every_pixel_that_moved() {
        let plain = decode(&rgba(6, 5, |_, _| [9, 9, 9, 255])).expect("decodes");
        let marked = decode(&rgba(6, 5, |x, y| {
            if (x, y) == (1, 1) || (x, y) == (4, 3) {
                [200, 0, 0, 255]
            } else {
                [9, 9, 9, 255]
            }
        }))
        .expect("decodes");
        assert_eq!(changed(&plain, &marked, 5), Some([1, 1, 4, 3]));
        assert_eq!(changed(&plain, &plain, 5), None);
    }

    /// The rows past the picture are the band, whose number changes every
    /// frame: a difference there is none.
    #[test]
    fn the_rows_past_the_picture_are_not_compared() {
        let plain = decode(&rgba(6, 5, |_, _| [9, 9, 9, 255])).expect("decodes");
        let numbered = decode(&rgba(6, 5, |x, y| {
            if (x, y) == (4, 3) {
                [200, 0, 0, 255]
            } else {
                [9, 9, 9, 255]
            }
        }))
        .expect("decodes");
        assert_eq!(changed(&plain, &numbered, 3), None);
        assert_eq!(changed(&plain, &numbered, 4), Some([4, 3, 4, 3]));
    }
}
