//! Runs a read lands in as a picture is taken, and what the saved
//! pictures show afterwards (`cargo xtask shot-check`).
//!
//! A picture is one claim: the texture saved is of the page settled, read
//! as the grab answers (`AutoShotDriver`'s attempts). Whether a picture a
//! read landed in is taken again from its source — the mirror's texture
//! refreshed, the window drawn afresh — and not off the texture the read
//! was drawn into cannot be read off the run's lines alone: the lines say
//! what the driver believed of its attempts. So the read is injected
//! (`PGG_FAULT_OWED_AT_SHOT`, `WindowHarness.owedForPicture`), a mark is
//! drawn into the content for as long as the injected read stands, and
//! the saved pictures are decoded: the mark in one is a texture the
//! attempt should have thrown away. Two cases stand the mark and inject
//! nothing, so that a mark never reaching a picture would be seen too.

use std::path::{Path, PathBuf};
use std::time::Instant;

/// A verb that opens a menu: the overlay stands, so the scene's picture —
/// the mirror of the content — is taken as well as the window's.
const VERB: &str = "commit-menu";

/// The mark's colour, as `WindowHarness.faultMark` draws it: a hue the
/// theme has nowhere.
const MARK: [u8; 3] = [0xff, 0x00, 0xff];

/// The pictures a run can leave; the ones it did not take are not read.
const PICTURES: [&str; 3] = ["app", "overlay", "scene"];

/// One run with a read injected into one picture's readings.
struct Case {
    /// `PGG_FAULT_OWED_AT_SHOT`'s value: the picture, and when.
    fault: &'static str,
    /// Must all appear in what the run printed: the attempts' own lines
    /// (`AutoShotDriver.say`).
    wants: &'static [&'static str],
    /// Must not.
    forbids: &'static [&'static str],
    /// The read discarded the first attempt, so a later one saves the
    /// picture; otherwise any attempt may — the ink can discard one of
    /// its own, whatever the read did.
    saved_later: bool,
    /// The pictures the mark belongs in; every other picture on disk must
    /// be without it.
    marked: &'static [&'static str],
}

impl Case {
    fn artifact(&self) -> &'static str {
        self.fault.split('-').next().unwrap_or(self.fault)
    }

    /// What the run should have said and did not, said and should not
    /// have, and the attempt that saved the picture against `saved_later`.
    fn missing(&self, said: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .wants
            .iter()
            .filter(|want| !said.contains(**want))
            .map(|want| format!("missing `{want}`"))
            .chain(
                self.forbids
                    .iter()
                    .filter(|word| said.contains(**word))
                    .map(|word| format!("unexpected `{word}`")),
            )
            .collect();
        let key = format!("artifact={} attempt=", self.artifact());
        let saved_on = said
            .lines()
            .filter(|line| line.contains(&key) && line.contains(" saved=true"))
            .find_map(|line| {
                line.split(&key)
                    .nth(1)?
                    .split(' ')
                    .next()?
                    .parse::<u32>()
                    .ok()
            });
        match saved_on {
            None => out.push(format!("missing `{key}<n> saved=true`")),
            Some(1) if self.saved_later => {
                out.push("saved on the first attempt, which the read landed in".to_string());
            }
            _ => {}
        }
        out
    }

    /// The pictures wrong about the mark.
    fn mismarked(&self, dir: &Path) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for name in PICTURES {
            let file = dir.join(format!("{name}.png"));
            let belongs = self.marked.contains(&name);
            let Ok(bytes) = std::fs::read(&file) else {
                if belongs {
                    out.push(format!("{name}.png is not on disk"));
                }
                continue;
            };
            let image =
                crate::png::decode(&bytes).map_err(|e| format!("{}: {e}", file.display()))?;
            if carries_the_mark(&image) != belongs {
                out.push(format!(
                    "{name}.png {}",
                    if belongs {
                        "has no mark: the mark never reached the picture"
                    } else {
                        "carries the mark: a texture the fault was drawn into was saved"
                    }
                ));
            }
        }
        Ok(out)
    }
}

fn carries_the_mark(image: &crate::png::Image) -> bool {
    (0..image.height).any(|y| (0..image.width).any(|x| image.pixel(x, y)[..3] == MARK))
}

/// The read before the window's grab, as it answers, and never; the same
/// three for the scene, whose source is a mirror's texture.
const CASES: &[Case] = &[
    Case {
        fault: "app-before",
        wants: &["artifact=app attempt=1 waiting=fault"],
        forbids: &["held=fault"],
        saved_later: false,
        marked: &[],
    },
    Case {
        fault: "app-during",
        wants: &["artifact=app attempt=1 held=fault"],
        forbids: &[],
        saved_later: true,
        marked: &[],
    },
    Case {
        fault: "app-shown",
        wants: &[],
        forbids: &["waiting=fault", "held=fault"],
        saved_later: false,
        // The scene mirrors the content after the window's picture, with the mark still standing.
        marked: &["app", "scene"],
    },
    Case {
        fault: "scene-before",
        wants: &["artifact=scene attempt=1 waiting=fault"],
        forbids: &["held=fault"],
        saved_later: false,
        marked: &[],
    },
    Case {
        fault: "scene-during",
        wants: &["artifact=scene attempt=1 held=fault"],
        forbids: &[],
        saved_later: true,
        marked: &[],
    },
    Case {
        fault: "scene-shown",
        wants: &[],
        forbids: &["waiting=fault", "held=fault"],
        saved_later: false,
        marked: &["scene"],
    },
];

/// Every case must pass verify-ui — the picture taken again is the
/// product — and leave the lines and the pictures the attempts claim.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let reuse = reuse_the_build(args, "shot-check")?;
    let me = std::env::current_exe().map_err(|e| format!("could not find this binary: {e}"))?;
    let mut failed = Vec::new();
    for (nth, case) in CASES.iter().enumerate() {
        println!("shot-check: {}", case.fault);
        // waits(measured): what the case cost, for the line under it — the verdict is the words and the pixels
        let began = Instant::now();
        // Only the first case builds the app, and none when the release stands.
        let said = drive(&me, case, nth == 0 && !reuse)?;
        let mut wrong = case.missing(&said);
        match shot_dir(&said) {
            Some(dir) => wrong.extend(case.mismarked(&dir)?),
            None => wrong.push(
                "no `screenshot saved=… path=` line: where the pictures went is unsaid".to_string(),
            ),
        }
        println!(
            "  {} in {:.1}s",
            if wrong.is_empty() {
                "read back"
            } else {
                "NOT read back"
            },
            began.elapsed().as_secs_f32()
        );
        if !wrong.is_empty() {
            for line in said.lines() {
                println!("  > {line}");
            }
            failed.push(format!("{}: {wrong:?}", case.fault));
        }
    }
    if failed.is_empty() {
        println!("shot-check: a picture a read landed in is taken again from its source");
        return Ok(());
    }
    Err(format!(
        "the pictures did not come back as the attempts claimed:\n  {}",
        failed.join("\n  ")
    ))
}

/// Whether `args` say the release stands already — `--no-build`, the one
/// word the gate's block hands a step after the first built it
/// (`gate::sides::verbs`): the first case then builds nothing either.
pub(super) fn reuse_the_build(args: &[String], check: &str) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [word] if word == "--no-build" => Ok(true),
        _ => Err(format!(
            "{check} takes --no-build alone, and got {}",
            args.join(" ")
        )),
    }
}

/// Runs one case and answers with both streams, together. A run the
/// injected read fails is a red here, whatever it said.
fn drive(me: &Path, case: &Case, build: bool) -> Result<String, String> {
    let mut command = std::process::Command::new(me);
    command
        .arg("verify-ui")
        .arg(VERB)
        .arg("--fault-owed-at-shot")
        .arg(case.fault);
    // Nobody asked to look at a run made to retake its picture.
    command.arg("--no-board");
    if !build {
        command.arg("--no-build");
    }
    let out = crate::subprocess::run_captured(&mut command)?;
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        return Err(format!(
            "verify-ui failed under the injected read {}:\n{said}",
            case.fault
        ));
    }
    Ok(said)
}

/// Where the run put its pictures, off the window's own line
/// (`screenshot saved=… path=<dir>/app.png`).
fn shot_dir(said: &str) -> Option<PathBuf> {
    said.lines()
        .find(|line| line.contains("screenshot saved="))
        .and_then(|line| line.split(" path=").nth(1))
        .map(|path| PathBuf::from(path.trim()))
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

#[cfg(test)]
mod tests {
    use super::{CASES, MARK, PICTURES, VERB, shot_dir};

    /// Each case names a picture and one of the three moments, and wants the lines that moment leaves: a read
    /// before the grab holds the first attempt; one as it answers discards it, so a later attempt saves; none leaves
    /// the mark in the picture.
    #[test]
    fn every_case_wants_the_lines_its_moment_leaves() {
        for case in CASES {
            let (artifact, when) = case.fault.split_once('-').expect(case.fault);
            assert!(PICTURES.contains(&artifact), "{}", case.fault);
            assert_eq!(case.artifact(), artifact);
            match when {
                "before" => {
                    assert!(
                        case.wants
                            .contains(&&*format!("artifact={artifact} attempt=1 waiting=fault"))
                    );
                    assert!(case.forbids.contains(&"held=fault"));
                    assert!(!case.saved_later);
                    assert!(case.marked.is_empty());
                }
                "during" => {
                    assert!(
                        case.wants
                            .contains(&&*format!("artifact={artifact} attempt=1 held=fault"))
                    );
                    assert!(case.saved_later);
                    assert!(case.marked.is_empty());
                }
                "shown" => {
                    assert!(case.forbids.contains(&"waiting=fault"));
                    assert!(case.forbids.contains(&"held=fault"));
                    assert!(!case.saved_later);
                    assert!(case.marked.contains(&artifact), "{}", case.fault);
                }
                other => panic!("{}: no such moment {other}", case.fault),
            }
            assert!(
                !case.marked.contains(&"overlay"),
                "the overlay mirrors the popups, not the content"
            );
        }
        assert!(
            CASES.iter().any(|case| case.fault.starts_with("scene-")),
            "a mirror's texture is the one that can go stale"
        );
    }

    /// A run that said the words with the mark in the picture is wrong, one with the words missing is wrong, and
    /// one that saved the picture on the attempt the read landed in is wrong; an attempt the ink discarded is
    /// nobody's concern.
    #[test]
    fn the_words_and_the_pictures_are_both_read() {
        let during = CASES
            .iter()
            .find(|case| case.fault == "app-during")
            .expect("app-during");
        let said = "picture run=r artifact=app attempt=1 held=fault\n\
                    picture run=r artifact=app attempt=2 saved=true";
        assert!(during.missing(said).is_empty());
        assert_eq!(
            during.missing("picture run=r artifact=app attempt=1 saved=true"),
            vec![
                "missing `artifact=app attempt=1 held=fault`",
                "saved on the first attempt, which the read landed in",
            ]
        );
        assert_eq!(
            during.missing("picture run=r artifact=app attempt=1 held=fault"),
            vec!["missing `artifact=app attempt=<n> saved=true`"]
        );
        let before = CASES
            .iter()
            .find(|case| case.fault == "app-before")
            .expect("app-before");
        let inked = "picture run=r artifact=app attempt=1 waiting=fault\n\
                     picture run=r artifact=app attempt=1 ink=3\n\
                     picture run=r artifact=app attempt=2 saved=true";
        assert!(
            before.missing(inked).is_empty(),
            "{:?}",
            before.missing(inked)
        );
        let dir = crate::temp::root().join("pgg-shot-check-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the test's directory");
        std::fs::write(
            dir.join("app.png"),
            crate::png::rgba(2, 1, |x, _| {
                if x == 0 {
                    [20, 30, 40, 255]
                } else {
                    [255, 0, 255, 255]
                }
            }),
        )
        .expect("a picture with the mark");
        let wrong = during.mismarked(&dir).expect("read");
        assert_eq!(wrong.len(), 1, "{wrong:?}");
        assert!(
            wrong[0].starts_with("app.png carries the mark"),
            "{wrong:?}"
        );
        let shown = CASES
            .iter()
            .find(|case| case.fault == "app-shown")
            .expect("app-shown");
        let wrong = shown.mismarked(&dir).expect("read");
        assert_eq!(
            wrong,
            vec!["scene.png is not on disk".to_string()],
            "the window's picture carries it, the scene's is owed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The mark is the harness's: one colour, spelled in the QML it is
    /// drawn in.
    #[test]
    fn the_mark_is_the_colour_the_harness_draws() {
        let harness = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../platitude-app/src/auto/WindowHarness.qml"
        );
        let text = std::fs::read_to_string(harness).expect("the harness");
        let spelled = format!("\"#{:02x}{:02x}{:02x}\"", MARK[0], MARK[1], MARK[2]);
        assert!(
            text.contains(&spelled),
            "{spelled} is not in WindowHarness.qml"
        );
        assert!(text.contains("faultMark"));
    }

    #[test]
    fn the_shot_directory_is_read_off_the_windows_line() {
        let said = "  | INFO bench: pictures run=r1 wanted=app\n  | screenshot saved=true ink=0 path=C:/t/r1/app.png\n";
        assert_eq!(
            shot_dir(said).as_deref(),
            Some(std::path::Path::new("C:/t/r1"))
        );
        assert_eq!(shot_dir("nothing"), None);
    }

    #[test]
    fn the_release_is_reused_on_the_one_word_the_block_hands_down() {
        assert_eq!(super::reuse_the_build(&[], "shot-check"), Ok(false));
        assert_eq!(
            super::reuse_the_build(&["--no-build".to_string()], "shot-check"),
            Ok(true)
        );
        assert!(
            super::reuse_the_build(&["--fresh".to_string()], "shot-check")
                .is_err_and(|why| why.contains("shot-check takes --no-build alone"))
        );
    }

    #[test]
    fn the_verb_opens_a_menu() {
        assert!(!VERB.contains(' '), "{VERB}");
        assert!(
            VERB.ends_with("-menu"),
            "the scene's picture needs the overlay standing: {VERB}"
        );
    }
}
