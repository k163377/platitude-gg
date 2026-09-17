use std::path::PathBuf;

use super::super::write::{chunk, rgba as written, zlib_stored};
use super::*;

/// A picture of `raw`'s already-filtered scanlines, framed by hand so a
/// test can name the filter each row uses.
fn framed(width: u32, height: u32, colour: u8, raw: &[u8]) -> Vec<u8> {
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, colour, 0, 0, 0]);
    let mut out = SIGNATURE.to_vec();
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn pixels(image: &Image) -> Vec<[u8; 4]> {
    (0..image.height)
        .flat_map(|y| (0..image.width).map(move |x| (x, y)))
        .map(|(x, y)| image.pixel(x, y))
        .collect()
}

#[test]
fn a_picture_this_wrote_comes_back_pixel_for_pixel() {
    let png = written(5, 3, |x, y| [x as u8 * 10, y as u8 * 20, 7, 255 - x as u8]);
    let image = decode(&png).expect("a picture written here reads back");
    assert_eq!((image.width, image.height), (5, 3));
    for y in 0..3 {
        for x in 0..5 {
            assert_eq!(
                image.pixel(x, y),
                [x as u8 * 10, y as u8 * 20, 7, 255 - x as u8],
                "at {x},{y}"
            );
        }
    }
}

/// Every filter a scanline may name, and the neighbours each one reads:
/// the byte to its left in the row being undone, the byte above it in
/// the row already undone, and the corner between them. The bytes here
/// and the pixels they come to are both worked out from the spec rather
/// than from this file, so a filter that reads the wrong neighbour
/// cannot agree with the answer.
#[test]
fn every_filter_a_scanline_may_name_comes_off() {
    #[rustfmt::skip]
    let raw = [
        0, 10, 20, 30, 40, 50, 60,
        1, 1, 2, 3, 4, 5, 6,
        2, 100, 100, 100, 100, 100, 100,
        3, 0, 0, 0, 0, 0, 0,
        4, 150, 0, 0, 0, 0, 0,
    ];
    let image = decode(&framed(2, 5, 2, &raw)).expect("five rows, five filters");
    assert_eq!(
        pixels(&image),
        [
            // None: the bytes as they stand.
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            // Sub: nothing to the left of the first pixel, then 4+1, 5+2, 6+3.
            [1, 2, 3, 255],
            [5, 7, 9, 255],
            // Up: a hundred on every byte of the row above.
            [101, 102, 103, 255],
            [105, 107, 109, 255],
            // Average: half of left and above together, rounded down.
            [50, 51, 51, 255],
            [77, 79, 80, 255],
            // Paeth: above for the first pixel, and left for the second,
            // whose left neighbour is the 200 the row starts with.
            [200, 51, 51, 255],
            [200, 79, 80, 255],
        ]
    );
}

#[test]
fn paeth_picks_the_neighbour_the_prediction_lands_nearest() {
    assert_eq!(paeth(50, 10, 10), 50);
    assert_eq!(paeth(10, 60, 10), 60);
    assert_eq!(paeth(10, 200, 100), 100);
}

/// A real encoder's output: dynamic Huffman codes, which nothing in
/// this crate writes, under filters it chose row by row. The file's own
/// two sums are recomputed while it is read, so arriving at a picture
/// at all is the inflating being right.
#[test]
fn a_picture_another_encoder_wrote_reads() {
    let icon =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../platitude-app/assets/icon-16.png");
    let file = std::fs::read(&icon).expect("the app's icon is in the tree");
    let image = decode(&file).expect("a real encoder's png reads");
    assert_eq!((image.width, image.height), (16, 16));
    let all = pixels(&image);
    assert!(
        all.iter().any(|pixel| pixel[3] == 0) && all.iter().any(|pixel| pixel[3] == 255),
        "an icon has both a transparent margin and opaque ink"
    );
    assert!(
        all.iter()
            .filter(|pixel| pixel[3] > 0)
            .any(|pixel| pixel[..3] != all[0][..3]),
        "the picture is not one flat colour"
    );
}

#[test]
fn a_file_that_is_not_a_picture_says_so_rather_than_reading_bytes() {
    assert!(decode(b"nothing like a png at all").is_err_and(|why| why.contains("signature")));
    assert!(decode(&[]).is_err_and(|why| why.contains("signature")));
}

#[test]
fn a_picture_that_arrived_damaged_is_refused_rather_than_shown() {
    let whole = written(4, 4, |x, _| [x as u8, 0, 0, 255]);
    // The last pixel of the last row: inside the stored block, so the
    // chunk's own sum is what catches it.
    let mut bent = whole.clone();
    let at = bent.len() - 20;
    bent[at] ^= 0xff;
    assert!(decode(&bent).is_err_and(|why| why.contains("checksum")));
    assert!(decode(&whole[..whole.len() - 8]).is_err());
}

#[test]
fn a_shape_this_cannot_read_is_named_rather_than_half_read() {
    let raw = [0u8; 7];
    assert!(decode(&framed(2, 1, 3, &raw)).is_err_and(|why| why.contains("colour type 3")));
    assert!(decode(&framed(0, 1, 2, &raw)).is_err_and(|why| why.contains("0x1")));
}

/// The sizes come out of the file, so the length they multiply out
/// to has to be refused.
#[test]
fn a_header_naming_more_picture_than_there_is_memory_is_refused() {
    let huge = framed(u32::MAX, u32::MAX, 2, &[0u8; 7]);
    assert!(decode(&huge).is_err_and(|why| why.contains("more picture than a machine holds")));
}
