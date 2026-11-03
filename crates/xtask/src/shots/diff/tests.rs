use super::*;

const GROUND: [u8; 4] = [16, 20, 32, 255];
const INK: [u8; 4] = [230, 230, 230, 255];

/// A twenty-by-six picture on the dark ground with ink at the points given.
fn picture_with(ink: &[(u32, u32)]) -> png::Image {
    let file = png::rgba(
        20,
        6,
        |x, y| {
            if ink.contains(&(x, y)) { INK } else { GROUND }
        },
    );
    png::decode(&file).expect("a picture just written reads back")
}

#[test]
fn the_same_picture_twice_changes_nothing_and_frames_nothing() {
    let diff = compare(&picture_with(&[(3, 2)]), &picture_with(&[(3, 2)])).expect("one size");
    assert_eq!(diff.changed, 0);
    assert!(diff.places.is_empty());
    let drawn = png::decode(&picture(&picture_with(&[(3, 2)]), &diff)).expect("a picture");
    // The after, dimmed: its ink a mid grey, its ground near black, both grey.
    let [r, g, b, _] = drawn.pixel(3, 2);
    assert!(
        r == g && g == b && r > 60 && r < 110,
        "dimmed ink: {r},{g},{b}"
    );
    let [r, g, b, _] = drawn.pixel(0, 0);
    assert!(r == g && g == b && r < 20, "dimmed ground: {r},{g},{b}");
}

#[test]
fn ink_that_moved_is_green_where_it_came_and_red_where_it_left_each_framed_alone() {
    let before = picture_with(&[(1, 2)]);
    let after = picture_with(&[(15, 2)]);
    let diff = compare(&before, &after).expect("one size");
    assert_eq!(diff.changed, 2);
    assert_eq!(diff.at(15, 2), Change::Lighter);
    assert_eq!(diff.at(1, 2), Change::Darker);
    // Further apart than `GATHER`: two places, each its own pixel.
    assert_eq!(diff.places, vec![[1, 2, 1, 2], [15, 2, 15, 2]]);
    let drawn = png::decode(&picture(&after, &diff)).expect("a picture");
    assert_eq!(drawn.pixel(15, 2), LIGHTER);
    assert_eq!(drawn.pixel(1, 2), DARKER);
    // Each frame stands `FRAME_AIR` outside its pixel, held inside the picture.
    assert_eq!(drawn.pixel(13, 0), FRAME);
    assert_eq!(drawn.pixel(17, 4), FRAME);
    assert_eq!(
        drawn.pixel(0, 0),
        FRAME,
        "the left place's frame is held at the edge"
    );
    assert_ne!(drawn.pixel(14, 2), FRAME, "inside the frame is the picture");
}

#[test]
fn changes_within_gather_of_each_other_are_one_place() {
    let before = picture_with(&[]);
    let after = picture_with(&[(5, 1), (5 + GATHER, 3)]);
    let diff = compare(&before, &after).expect("one size");
    assert_eq!(diff.places, vec![[5, 1, 5 + GATHER, 3]]);
}

#[test]
fn two_sizes_are_refused_by_name() {
    let small = png::decode(&png::rgba(4, 4, |_, _| GROUND)).expect("a picture");
    let refused = compare(&small, &picture_with(&[])).err().expect("refused");
    assert!(
        refused.contains("4x4") && refused.contains("20x6"),
        "{refused}"
    );
}

#[test]
fn the_difference_lands_beside_the_after_named_for_it() {
    let dir = crate::verify::claim_dir(&crate::temp::root().join("pgg-diff"), "diff")
        .expect("a directory of its own");
    let before = dir.join("before.png");
    let after = dir.join("app-crop.png");
    std::fs::write(&before, png::rgba(20, 6, |_, _| GROUND)).expect("written");
    std::fs::write(
        &after,
        png::rgba(20, 6, |x, _| if x == 7 { INK } else { GROUND }),
    )
    .expect("written");
    run(&[
        before.to_string_lossy().into_owned(),
        after.to_string_lossy().into_owned(),
    ])
    .expect("one size");
    let file = std::fs::read(dir.join("app-crop-diff.png")).expect("written beside the after");
    let drawn = png::decode(&file).expect("a picture");
    assert_eq!(drawn.pixel(7, 3), LIGHTER);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn one_picture_or_three_are_refused() {
    assert!(parse(&["a.png".to_string()]).is_err());
    assert!(
        parse(&[
            "a.png".to_string(),
            "b.png".to_string(),
            "c.png".to_string()
        ])
        .is_err()
    );
}
