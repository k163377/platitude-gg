use super::*;

fn words(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

/// A directory of this run's own, and the picture the crops are cut
/// from: six by four, every pixel telling which one it is.
fn shot() -> Result<(PathBuf, PathBuf), String> {
    let dir = crate::verify::claim_dir(&std::env::temp_dir().join("pg-crop"), "crop")?;
    let source = dir.join("app.png");
    let png = png::rgba(6, 4, |x, y| [x as u8 * 40, y as u8 * 60, 7, 255]);
    std::fs::write(&source, &png).map_err(|e| e.to_string())?;
    Ok((dir, source))
}

#[test]
fn a_crop_holds_the_region_asked_for_with_every_pixel_a_square_block() {
    let (dir, source) = shot().expect("a picture to cut from");
    let out = dir.join("cut.png");
    run(&words(&[
        &source.to_string_lossy(),
        "--at",
        "2:1:3:2",
        "--scale",
        "4",
        "--out",
        &out.to_string_lossy(),
    ]))
    .expect("a region inside the picture");
    let file = std::fs::read(&out).expect("the crop was written");
    let crop = png::decode(&file).expect("and it is a picture");
    assert_eq!((crop.width, crop.height), (12, 8));
    for y in 0..crop.height {
        for x in 0..crop.width {
            // Nearest neighbour: the source pixel this one stands on,
            // never a value between two of them.
            let (from_x, from_y) = (2 + x / 4, 1 + y / 4);
            assert_eq!(
                crop.pixel(x, y),
                [from_x as u8 * 40, from_y as u8 * 60, 7, 255],
                "at {x},{y}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The default: three times, beside the picture, named for the region
/// so a second region does not land on the first.
#[test]
fn a_crop_nobody_places_lands_beside_the_shot_under_the_region_it_cut() {
    let (dir, source) = shot().expect("a picture to cut from");
    run(&words(&[&source.to_string_lossy(), "--at", "1:0:2:3"])).expect("a region inside");
    let out = dir.join("app-crop-1-0-2x3.png");
    let file = std::fs::read(&out).expect("the crop is beside the shot");
    let crop = png::decode(&file).expect("and it is a picture");
    assert_eq!((crop.width, crop.height), (6, 9));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_region_that_is_not_all_inside_the_picture_is_refused_with_its_size() {
    let (dir, source) = shot().expect("a picture to cut from");
    let outside = run(&words(&[&source.to_string_lossy(), "--at", "4:0:3:1"]));
    assert!(
        outside.is_err_and(|why| why.contains("6x4")),
        "a crop running off the right edge says how wide the picture was"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_magnification_with_a_fraction_in_it_is_refused() {
    assert_eq!(whole("3"), Ok(3));
    for asked in ["2.5", "0", "-1", "x2", ""] {
        assert!(
            whole(asked).is_err_and(|why| why.contains("interpolate")),
            "{asked:?} was taken for a whole number of times"
        );
    }
}

#[test]
fn a_region_is_four_whole_numbers_and_says_so_when_it_is_not() {
    let rect = rect("10:20:30:40").expect("four numbers");
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (10, 20, 30, 40));
    for asked in ["10:20:30", "10:20:30:40:50", "10:20:30:x", ""] {
        assert!(rect_is_err(asked), "{asked:?} was taken for a region");
    }
    // A crop of nothing is not a crop.
    assert!(rect_is_err("0:0:0:8"));
    assert!(rect_is_err("0:0:8:0"));
}

fn rect_is_err(text: &str) -> bool {
    rect(text).is_err()
}

/// The ceiling is on what comes out, so a small region at a large
/// magnification is caught the same as a large one — including the
/// magnification whose square is more than a u64 holds.
#[test]
fn a_crop_too_big_to_be_looked_at_is_refused_before_it_is_written() {
    let (dir, source) = shot().expect("a picture to cut from");
    for asked in ["5000", &u32::MAX.to_string()] {
        let huge = run(&words(&[
            &source.to_string_lossy(),
            "--at",
            "0:0:6:4",
            "--scale",
            asked,
        ]));
        assert!(
            huge.is_err_and(|why| why.contains("undoes the magnifying")),
            "{asked}x was cut"
        );
        assert!(!dir.join("app-crop-0-0-6x4.png").exists());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_call_missing_what_it_needs_says_which_part() {
    assert!(parse(&words(&["--at", "0:0:1:1"])).is_err_and(|why| why.contains("no picture")));
    assert!(parse(&words(&["app.png"])).is_err_and(|why| why.contains("--at")));
    assert!(parse(&words(&["app.png", "--scale"])).is_err_and(|why| why.contains("wants a value")));
    assert!(
        parse(&words(&["app.png", "--at", "0:0:1:1", "--twice"]))
            .is_err_and(|why| why.contains("unknown flag"))
    );
    assert!(
        parse(&words(&["one.png", "--at", "0:0:1:1", "two.png"]))
            .is_err_and(|why| why.contains("one picture at a time"))
    );
}

#[test]
fn a_crop_beside_a_shot_is_named_for_its_region() {
    let at = Rect {
        x: 12,
        y: 340,
        width: 56,
        height: 7,
    };
    assert_eq!(
        beside(Path::new("target/verify-ui/app.png"), &at),
        PathBuf::from("target/verify-ui/app-crop-12-340-56x7.png")
    );
}
