//! Pictures in every bucket the diff pane previews from, and one too big
//! for anything but a file to carry.

use super::repo::DemoRepo;

const TEAL: [u8; 4] = [0x2a, 0x9d, 0x8f, 0xff];
const GOLD: [u8; 4] = [0xe9, 0xc4, 0x6a, 0xff];
const NAVY: [u8; 4] = [0x26, 0x46, 0x53, 0xff];
const CORAL: [u8; 4] = [0xe7, 0x6f, 0x51, 0xff];

/// A square of one colour with a diagonal band of another: hard edges an
/// upscale either keeps or blurs.
fn badge(size: u32, face: [u8; 4], band: [u8; 4]) -> Vec<u8> {
    let width = size / 6;
    crate::png::rgba(size, size, move |x, y| {
        let along = x.abs_diff(y);
        if along < width { band } else { face }
    })
}

/// A photograph's shape without a photograph: two gradients and a grid,
/// which no zoom step fits and every frame has to shrink.
fn scene(width: u32, height: u32) -> Vec<u8> {
    crate::png::rgba(width, height, move |x, y| {
        let r = (x * 255 / width.max(1)) as u8;
        let g = (y * 255 / height.max(1)) as u8;
        let grid = x % 40 == 0 || y % 40 == 0;
        if grid {
            [0xf4, 0xf1, 0xde, 0xff]
        } else {
            [r, g, 0x80, 0xff]
        }
    })
}

/// The big one, in three versions so both a commit and the tree can
/// change it; `turn` tells them apart at a glance.
fn weave(size: u32, turn: u32) -> Vec<u8> {
    let plain = [NAVY, TEAL, GOLD][turn as usize % 3];
    crate::png::rgba(size, size, move |x, y| {
        let warp = (x / 25 + y / 25) % 2 == 0;
        if !warp {
            return plain;
        }
        let channels = [
            (x * 255 / size) as u8,
            (y * 255 / size) as u8,
            ((x ^ y) & 0xff) as u8,
        ];
        let at = |i: u32| channels[((i + turn) % 3) as usize];
        [at(0), at(1), at(2), 0xff]
    })
}

const MARK_V1: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"120\" viewBox=\"0 0 120 120\">\n\
  <rect x=\"10\" y=\"10\" width=\"100\" height=\"100\" rx=\"16\" fill=\"#2a9d8f\"/>\n\
  <circle cx=\"60\" cy=\"60\" r=\"28\" fill=\"#e9c46a\"/>\n\
</svg>\n";

const MARK_V2: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"120\" viewBox=\"0 0 120 120\">\n\
  <rect x=\"10\" y=\"10\" width=\"100\" height=\"100\" rx=\"16\" fill=\"#e9c46a\"/>\n\
  <circle cx=\"60\" cy=\"60\" r=\"28\" fill=\"#2a9d8f\"/>\n\
  <rect x=\"52\" y=\"20\" width=\"16\" height=\"80\" fill=\"#264653\"/>\n\
</svg>\n";

/// A picture in every bucket: a logo changed in the tree, a vector mark
/// changed beside it (rows and pictures at once), an icon staged and
/// never committed (no HEAD side), an old mark deleted from the index
/// (no index side), and a photo nobody has added.
pub(super) fn pictures(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA picture in every bucket the diff pane previews from.\n",
        "docs: start the readme",
    )?;
    repo.commit_bytes("art/logo.png", &badge(96, TEAL, GOLD), "feat: add the logo")?;
    repo.commit_bytes(
        "art/old.png",
        &badge(48, GOLD, CORAL),
        "feat: add the old mark",
    )?;
    repo.commit("art/mark.svg", MARK_V1, "feat: add the vector mark")?;
    repo.commit(
        "notes.txt",
        "The pictures are in art/.\n",
        "docs: say where the pictures are",
    )?;

    repo.write_bytes("art/logo.png", &badge(96, GOLD, TEAL))?;
    repo.write("art/mark.svg", MARK_V2)?;
    // The icon is the zoom steps' target.
    repo.write_bytes("art/icon.png", &badge(16, NAVY, GOLD))?;
    repo.git(&["add", "--", "art/icon.png"])?;
    repo.git(&["rm", "--quiet", "--", "art/old.png"])?;
    repo.write_bytes("art/photo.png", &scene(640, 400))?;
    Ok(())
}

/// Side length of the big picture: each diff side is past what a data URL
/// could carry (16 MiB).
const BIG_SIDE: u32 = 3000;

/// One picture too big for anything but a file, changed by the newest
/// commit (what `perf --file` opens) and again in the tree, so both diffs
/// have two sides to decode.
pub(super) fn bigpicture(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nOne big picture, changed.\n",
        "docs: start the readme",
    )?;
    repo.commit_bytes("big.png", &weave(BIG_SIDE, 0), "feat: add the big picture")?;
    repo.commit_bytes("big.png", &weave(BIG_SIDE, 1), "feat: turn the weave")?;
    repo.write_bytes("big.png", &weave(BIG_SIDE, 2))?;
    Ok(())
}
