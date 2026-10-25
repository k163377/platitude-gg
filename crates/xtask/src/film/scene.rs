//! A scene: what a film shows and what the hand does, read from a text
//! file of one line each (the film skill has the whole syntax).
//!
//! The steps are the component's own (`WindowFilmActs.qml.in`), checked
//! here so a typo stops before the release build rather than as an
//! unknown step in the middle of a run.

/// Where a scene's repository comes from. Either way it is built once
/// for every side of the scene and copied for each (`shoot`): a preset can
/// differ between the trees two sides are shot from, and two sides that
/// start from different repositories show that difference as the change.
#[derive(Debug, PartialEq)]
pub(super) enum Repo {
    /// Built by the tree shooting first.
    Preset(String),
    /// Built by hand at the scene's base (`demo-repo <preset> --at`, then
    /// changed): copied as it stands.
    Made,
}

/// One entry of the arg the component plays.
#[derive(Debug, PartialEq)]
pub(super) enum Step {
    /// The caption from here on, by its number in [`Scene::captions`].
    Caption(usize),
    /// An act, spelled as the component reads it.
    Act(String),
}

#[derive(Debug)]
pub(super) struct Scene {
    pub(super) repo: Repo,
    /// git's arguments, run in the copied repository before the window
    /// opens.
    pub(super) prep: Vec<Vec<String>>,
    /// `[layout]` keys of the window's state.toml, over the defaults.
    pub(super) layout: Vec<(String, String)>,
    /// What a frame shows, in window coordinates: x, y, width, height.
    pub(super) views: Vec<[u32; 4]>,
    pub(super) scale: u32,
    /// How often a frame is grabbed, in milliseconds, besides the frame
    /// every act takes at once.
    pub(super) tick: u32,
    pub(super) captions: Vec<String>,
    pub(super) steps: Vec<Step>,
}

/// The keys `k:` and `kc:` name, as the component's `keyOf` maps them.
const KEYS: &[&str] = &[
    "esc",
    "menu",
    "enter",
    "up",
    "down",
    "left",
    "right",
    "tab",
    "f",
    "f2",
    "f10",
    "space",
    "delete",
    "backspace",
    "home",
    "end",
    "pageup",
    "pagedown",
];

const MODIFIERS: &[&str] = &["ctrl", "shift", "alt"];

/// Every act verb, and how many integers it takes (`None`: free text,
/// checked by [`act`]).
const ACTS: &[(&str, Option<usize>)] = &[
    ("w", Some(1)),
    ("m", Some(2)),
    ("g", Some(5)),
    ("l", Some(2)),
    ("r", Some(2)),
    ("d", Some(2)),
    ("u", Some(2)),
    ("wh", Some(1)),
    ("wr", Some(1)),
    ("k", None),
    ("kc", None),
    ("t", None),
    ("fw", None),
    ("cp", None),
    ("git", None),
    ("rt", None),
    ("pg", None),
];

/// Whether a caption is the setup's: frames under it are cut, and each
/// side's clock starts at the first frame after it.
pub(super) fn is_setup(caption: &str) -> bool {
    caption.starts_with('-')
}

pub(super) fn parse(text: &str) -> Result<Scene, String> {
    let mut scene = Scene {
        repo: Repo::Made,
        prep: Vec::new(),
        layout: Vec::new(),
        views: Vec::new(),
        scale: 1,
        tick: 80,
        captions: Vec::new(),
        steps: Vec::new(),
    };
    let mut repo = None;
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |e: String| format!("line {}: {e}", number + 1);
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        if word.contains(':') {
            if scene.captions.is_empty() {
                return Err(at(
                    "the first step comes after a caption (`cap -` for the setup)".to_string(),
                ));
            }
            scene.steps.push(Step::Act(act(line).map_err(at)?));
            continue;
        }
        match word {
            "repo" => {
                repo = Some(match rest {
                    "" => return Err(at("`repo` names a preset, or `made`".to_string())),
                    "made" => Repo::Made,
                    preset => Repo::Preset(preset.to_string()),
                });
            }
            "prep" if !rest.is_empty() => scene
                .prep
                .push(rest.split_whitespace().map(String::from).collect()),
            "layout" => match rest.split_once(char::is_whitespace) {
                Some((key, value)) => scene
                    .layout
                    .push((key.to_string(), value.trim().to_string())),
                None => return Err(at("`layout` takes a key and its value".to_string())),
            },
            "view" => scene.views.push(four(rest).map_err(at)?),
            "scale" => scene.scale = positive(rest).map_err(at)?,
            "tick" => scene.tick = positive(rest).map_err(at)?,
            "cap" if !rest.is_empty() => {
                scene.steps.push(Step::Caption(scene.captions.len()));
                scene.captions.push(rest.to_string());
            }
            "prep" | "cap" => return Err(at(format!("`{word}` takes the rest of its line"))),
            _ => return Err(at(format!("`{word}` is neither a setting nor a step"))),
        }
    }
    scene.repo = repo.ok_or("a scene names its repository (`repo <preset>` or `repo made`)")?;
    if scene.views.is_empty() {
        return Err("a scene names what its frames show (`view <x> <y> <width> <height>`)".into());
    }
    if scene.captions.iter().all(|caption| is_setup(caption)) {
        return Err("a scene has a caption past its setup, or its film is empty".into());
    }
    Ok(scene)
}

/// One act, checked against what the component reads.
fn act(line: &str) -> Result<String, String> {
    if line.contains(';') {
        return Err("a step holds no `;` (the arg's separator)".to_string());
    }
    let (verb, arg) = line.split_once(':').unwrap_or((line, ""));
    let Some((_, count)) = ACTS.iter().find(|(name, _)| *name == verb) else {
        let known: Vec<&str> = ACTS.iter().map(|(name, _)| *name).collect();
        return Err(format!(
            "`{verb}:` is no step (`cap` and {})",
            known.join(" ")
        ));
    };
    match (verb, count) {
        (_, Some(count)) => {
            let numbers: Vec<&str> = arg.split(',').collect();
            if numbers.len() != *count || numbers.iter().any(|n| n.parse::<i64>().is_err()) {
                return Err(format!("`{verb}:` takes {count} integers, comma-separated"));
            }
        }
        ("k", None) if !KEYS.contains(&arg) => {
            return Err(format!("`k:{arg}` names no key ({})", KEYS.join(" ")));
        }
        ("kc", None) => match arg.split_once(',') {
            Some((modifier, key)) if MODIFIERS.contains(&modifier) && KEYS.contains(&key) => {}
            _ => {
                return Err(format!(
                    "`kc:` takes a modifier ({}) and a key",
                    MODIFIERS.join(" ")
                ));
            }
        },
        ("fw" | "cp", None) => {
            let parts: Vec<&str> = arg.split('|').collect();
            if parts.len() < 2 || parts[0].is_empty() || (verb == "cp" && parts.len() != 2) {
                return Err(format!("`{verb}:` takes a path, `|`, and what goes there"));
            }
        }
        (_, None) if arg.is_empty() || arg.starts_with('|') => {
            return Err(format!("`{verb}:` takes an argument"));
        }
        _ => {}
    }
    Ok(line.to_string())
}

fn four(text: &str) -> Result<[u32; 4], String> {
    let numbers: Result<Vec<u32>, _> = text.split_whitespace().map(str::parse::<u32>).collect();
    match numbers.as_deref() {
        Ok([x, y, width, height]) if *width > 0 && *height > 0 => Ok([*x, *y, *width, *height]),
        _ => Err("`view` takes x, y, width and height: whole numbers, the sizes above 0".into()),
    }
}

fn positive(text: &str) -> Result<u32, String> {
    match text.parse::<u32>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("`{text}` is not a whole number above 0")),
    }
}

/// The arg the component plays: the views, the scale, the tick, the
/// side's tag and colour, then the steps. Text a person wrote goes
/// percent-encoded, so no separator inside it splits it.
pub(super) fn arg(scene: &Scene, tag: &str, colour: &str) -> String {
    let views: Vec<String> = scene
        .views
        .iter()
        .map(|v| format!("{},{},{},{}", v[0], v[1], v[2], v[3]))
        .collect();
    let mut parts = vec![
        views.join("/"),
        scene.scale.to_string(),
        scene.tick.to_string(),
        encoded(tag),
        colour.to_string(),
    ];
    for step in &scene.steps {
        parts.push(match step {
            Step::Caption(n) => format!("c:{n}|{}", encoded(&scene.captions[*n])),
            Step::Act(act) => act.clone(),
        });
    }
    parts.join(";")
}

/// The milliseconds a scene's waits and glides add up to: what the run
/// is given beyond its start.
pub(super) fn length_ms(scene: &Scene) -> u64 {
    scene
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Act(act) => act.split_once(':'),
            Step::Caption(_) => None,
        })
        .filter_map(|(verb, arg)| match verb {
            "w" => arg.parse::<u64>().ok(),
            "g" => arg.rsplit(',').next().and_then(|ms| ms.parse::<u64>().ok()),
            _ => None,
        })
        .sum()
}

/// As `encodeURIComponent` leaves it for `decodeURIComponent`: every byte
/// but the unreserved ones as `%XX`.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{ACTS, Repo, Step, arg, encoded, length_ms, parse};

    const SCENE: &str = "\
# A card, and a commit made from outside.
repo deep
prep branch zz-probe
layout sidebar_width 300
view 260 88 760 340
tick 40
cap -
m:900,700
w:600
cap 先頭の HEAD の行に手を置く(カードが出る)
g:900,700,600,110,450
w:1500
git:commit --allow-empty -q -m more1
";

    #[test]
    fn a_scene_reads_into_its_settings_and_steps() {
        let scene = parse(SCENE).expect("parses");
        assert_eq!(scene.repo, Repo::Preset("deep".to_string()));
        assert_eq!(scene.prep, [["branch", "zz-probe"]]);
        assert_eq!(
            scene.layout,
            [("sidebar_width".to_string(), "300".to_string())]
        );
        assert_eq!(scene.views, [[260, 88, 760, 340]]);
        assert_eq!(scene.tick, 40);
        assert_eq!(scene.captions.len(), 2);
        assert_eq!(scene.steps[0], Step::Caption(0));
        assert_eq!(scene.steps[3], Step::Caption(1));
        assert_eq!(length_ms(&scene), 600 + 450 + 1500);
    }

    #[test]
    fn the_arg_carries_the_header_and_encodes_what_a_person_wrote() {
        let scene = parse(SCENE).expect("parses");
        let played = arg(&scene, "今", "success");
        let parts: Vec<&str> = played.split(';').collect();
        assert_eq!(
            parts[..5],
            ["260,88,760,340", "1", "40", "%E4%BB%8A", "success"]
        );
        assert_eq!(parts[5], "c:0|-");
        assert!(parts[8].starts_with("c:1|%E5%85%88"));
        assert_eq!(parts[11], "git:commit --allow-empty -q -m more1");
        assert_eq!(encoded("a b;c|d"), "a%20b%3Bc%7Cd");
    }

    #[test]
    fn a_step_the_component_would_not_read_stops_the_scene() {
        let with = |step: &str| {
            parse(&format!(
                "repo deep\nview 0 0 10 10\ncap -\ncap x\n{step}\n"
            ))
        };
        assert!(with("m:1,2").is_ok());
        for wrong in [
            "m:1",
            "m:1,x",
            "k:nokey",
            "kc:meta,esc",
            "cp:only",
            "zz:1",
            "t:a;b",
            "rt:",
        ] {
            assert!(with(wrong).is_err(), "{wrong}");
        }
        assert!(parse("repo deep\nview 0 0 10 10\nm:1,2\ncap x\n").is_err());
        assert!(parse("repo deep\ncap -\ncap x\n").is_err());
        assert!(parse("view 0 0 10 10\ncap -\ncap x\n").is_err());
        assert!(parse("repo deep\nview 0 0 10 10\ncap -\n").is_err());
    }

    /// Every verb a scene may write is one the component plays.
    #[test]
    fn every_step_verb_is_one_the_component_answers_to() {
        let component = super::super::inject::component();
        for (verb, _) in ACTS {
            assert!(
                component.contains(&format!("verb === \"{verb}\"")),
                "the component plays `{verb}:`"
            );
        }
        assert!(component.contains("verb === \"c\""));
    }
}
