//! `cargo xtask verbs` — the registered lines (`gate::lines`) held against
//! the harness, the judgment tables and the sources, before any gate
//! chooses off them (verify-ui skill「gate が回す動詞は登録表
//! `crates/xtask/verb-lines.txt` の行」).
//!
//! What it refuses, each a selection that would be short or a run that
//! would wait out its ceiling saying nothing:
//!
//! - a verb the harness answers with no registered line — a scenario no
//!   change re-photographs, however it would be judged if it ran;
//! - a registered line whose verb the harness no longer names — the gate
//!   runs it, and the app ignores the name;
//! - a line spelled otherwise than `verify-ui` reads it back;
//! - a judgment row (`verify::verbs`) no registered line reaches — a
//!   state no gate puts the app in, whose runs pass on the picture alone;
//! - a row naming what is no component, or what every line shows;
//! - the tier table disagreeing with the lines (`gate::tiers`);
//! - a harness file nothing loads — a branch no verb reaches;
//! - a verb the window answers that the page's completion ledger does not
//!   defer (`AutoActCompletion.defersCompletion`) — the page completes it
//!   ahead of the window as soon as a page family or `isWriteAct` takes it.
//!
//! Both sides are read from the tree: the names the harness compares
//! `_PGG_AUTO_ACT` against, and each line's first word — not the skill's
//! prose, which undercounts. The failure for a gone verb is judged against
//! every kebab-case word the harness holds, not only dispatches — a verb
//! can be reached through a list (the file-row family), and the narrower
//! reading would turn every gate red over how the harness spells one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::gate::{Composition, Lines, Tiers};

/// Where the harness answers a verb by name. Nothing outside this
/// directory reads `_PGG_AUTO_ACT`'s value as a verb.
const HARNESS: &str = "crates/platitude-app/src/auto/";

/// The file whose `defersCompletion` names the verbs the page leaves to
/// someone else to complete.
const LEDGER: &str = "crates/platitude-app/src/auto/AutoActCompletion.qml";

/// The stem every file of the window's half of the harness starts with:
/// `WindowAutoActDriver`, the `Window*Acts` it builds, and the parts they
/// stand on (`WindowHarness`, `WindowStage`, …).
const WINDOW_STEM: &str = "Window";

pub fn run(args: &[String]) -> Result<(), String> {
    let root = crate::tree::workspace_root();
    let lines = Lines::load(&root)?;
    let composition = Composition::read(&root)?;
    match args {
        [] => {}
        [flag] if flag == "--core" => {
            for stem in lines.core(&composition) {
                println!("{stem}");
            }
            return Ok(());
        }
        [flag, line] if flag == "--shown" => return show_line(&lines, &composition, line),
        [flag, stem] if flag == "--why" => return why(&lines, &composition, stem),
        [flag] if flag == "--ledger" => return ledger(&lines, &Tiers::load(&root)?),
        other => {
            return Err(format!(
                "unknown option {other:?} (verbs takes --core, --shown <line>, --why <component>, \
                 --ledger or nothing)"
            ));
        }
    }
    let harness = harness(&root)?;
    let tiers = Tiers::load(&root)?;
    let registered = registered(&lines);
    // A verb whose every row is a twin has no line on purpose.
    let twins_only = tiers.twinned_away();

    let unrecorded: Vec<&String> = harness
        .answered
        .difference(&registered)
        .filter(|verb| !twins_only.contains(*verb))
        .collect();
    let gone: Vec<&String> = registered.difference(&harness.mentioned).collect();
    summary(
        &lines,
        &composition,
        &tiers,
        &harness,
        &registered,
        &twins_only,
    );

    let mut complaints: Vec<String> = Vec::new();
    for verb in &unrecorded {
        complaints.push(format!(
            "the harness answers `{verb}` and {} holds no line of it — register the line(s) \
             that run it (with what each brings up), or the verb is one no change re-photographs",
            crate::gate::LINES_FILE
        ));
    }
    for verb in &gone {
        complaints.push(format!(
            "{} holds line(s) of `{verb}`, which the harness no longer names anywhere — the gate \
             would run them and each would wait out its whole ceiling saying nothing. Drop the \
             rows or restore the name",
            crate::gate::LINES_FILE
        ));
    }
    for verb in crate::verify::verbs::verbs_with_claims() {
        if !harness.answered.contains(verb) {
            complaints.push(format!(
                "the claims table (verify/verbs.rs) names `{verb}`, which the harness does not \
                 answer — the claim would be owed by no run"
            ));
        }
    }
    complaints.extend(misspelled(&lines));
    complaints.extend(lines.complaints(&composition));
    complaints.extend(tiers.complaints(&lines));
    complaints.extend(
        crate::verify::verbs::unreached_rows(&typed(&lines, &tiers))
            .into_iter()
            .map(|row| format!("no registered line reaches the judgment row {row} — its runs would pass on the picture alone: register a line that puts the app in that state, or take the row out")),
    );
    complaints.extend(unreached_harness_files(&harness));
    complaints.extend(undeferred_window_verbs(&harness));
    if complaints.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the registered lines do not agree with the harness, the tables or the sources:\n{}",
        complaints
            .iter()
            .map(|c| format!("  {c}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// Every registered line with what its run owes, one tab-separated row
/// each, so a case is traceable from the table alone: the line (the verb,
/// its argument and the fixture it runs on), the tier it runs in, the
/// judgment row it must say, the claims it must picture, and the parts the
/// table promises its picture shows beyond the `*` row. What the run
/// waited for is the run's own to say (`stage` lines in its log).
fn ledger(lines: &Lines, tiers: &Tiers) -> Result<(), String> {
    println!("line\ttier\tmust_say\tclaims\tpromises");
    for line in lines.lines() {
        let tier = tiers.tier(line).map_or_else(
            || "premerge".to_string(),
            |t| format!("{t:?}").to_lowercase(),
        );
        let judgment = crate::verify::judgment_of(line);
        let mut must_say = "-";
        let mut claims = "-";
        for field in judgment.lines() {
            if let Some(rest) = field.strip_prefix("must_say=") {
                must_say = rest;
            } else if let Some(rest) = field.strip_prefix("claims=") {
                claims = if rest.is_empty() { "-" } else { rest };
            }
        }
        let promises = lines
            .entries
            .get(line)
            .map(|set| set.iter().cloned().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        println!(
            "{line}\t{tier}\t{must_say}\t{claims}\t{}",
            if promises.is_empty() { "-" } else { &promises }
        );
    }
    Ok(())
}

/// Where one component stands: in the core or not, and who composes it
/// or opens onto it.
fn why(lines: &Lines, composition: &Composition, stem: &str) -> Result<(), String> {
    let (composed, opened) = composition.declarers_of(stem);
    let or_dash = |names: &[String]| {
        if names.is_empty() {
            "-".to_string()
        } else {
            names.join(" ")
        }
    };
    println!(
        "{stem}: {}; composed by: {}; opened onto by: {}",
        if lines.core(composition).contains(stem) {
            "in the core"
        } else {
            "not in the core"
        },
        or_dash(&composed),
        or_dash(&opened),
    );
    Ok(())
}

/// The counts a person reads above the complaints.
fn summary(
    lines: &Lines,
    composition: &Composition,
    tiers: &Tiers,
    harness: &Harness,
    registered: &BTreeSet<String>,
    twins_only: &BTreeSet<String>,
) {
    println!("verbs the harness answers: {}", harness.answered.len());
    println!("verbs with a registered line: {}", registered.len());
    println!(
        "verbs no gate runs, every line a twin: {}",
        twins_only.difference(registered).count()
    );
    let (fonts, whole, twins, host) = tiers.counts();
    println!(
        "registered lines before a merge: {} (host), {fonts} of them on the container too, {host} \
         on the host alone; {whole} for the whole gate only; {twins} twin(s) no gate runs",
        lines.lines().filter(|line| tiers.owed(line, false)).count()
    );
    println!(
        "the window's core: {} component(s); subjects beyond it: {}",
        lines.shown(composition).core().len(),
        lines
            .entries
            .values()
            .flatten()
            .collect::<BTreeSet<_>>()
            .len()
    );
}

/// What one registered line shows beyond the core, for a person moving
/// its row: the subjects, then every component they bring up.
fn show_line(lines: &Lines, composition: &Composition, line: &str) -> Result<(), String> {
    let Some(subjects) = lines.entries.get(line) else {
        return Err(format!(
            "{line:?} is not registered in {}",
            crate::gate::LINES_FILE
        ));
    };
    let shown = lines.shown(composition);
    let core = shown.core();
    let beyond: Vec<&String> = shown
        .of(line)
        .map(|all| all.iter().filter(|stem| !core.contains(*stem)).collect())
        .unwrap_or_default();
    println!(
        "subjects: {}",
        if subjects.is_empty() {
            "-".to_string()
        } else {
            subjects.iter().cloned().collect::<Vec<_>>().join(" ")
        }
    );
    println!(
        "beyond the core ({} component(s)): {}",
        beyond.len(),
        beyond
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(())
}

/// Every registered line's verb: its first word.
fn registered(lines: &Lines) -> BTreeSet<String> {
    lines
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// A line spelled otherwise than `verify-ui` reads it back is one the
/// gate would run as another: the default preset written out, an option
/// in another order, an argument with a space.
fn misspelled(lines: &Lines) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines.lines() {
        match crate::verify::canonical_line(line) {
            Ok(Some(canonical)) if canonical == *line => {}
            Ok(Some(canonical)) => out.push(format!(
                "{} spells {line:?}, which verify-ui reads back as {canonical:?} — register that \
                 spelling",
                crate::gate::LINES_FILE
            )),
            Ok(None) => out.push(format!(
                "{} spells {line:?}, which nobody can type again (an argument with a space, or \
                 an option that pins the run to this machine) — a line the gate cannot run",
                crate::gate::LINES_FILE
            )),
            Err(why) => out.push(format!(
                "{} spells {line:?}, which verify-ui refuses: {why}",
                crate::gate::LINES_FILE
            )),
        }
    }
    out
}

/// Every line the tables may be reached through, as the tables read one:
/// the registered lines and the twins the tier table holds (a twin is a
/// run too, only not the gate's).
fn typed(lines: &Lines, tiers: &Tiers) -> Vec<crate::verify::verbs::Typed> {
    lines
        .lines()
        .cloned()
        .chain(tiers.twins())
        .filter_map(|line| {
            let words: Vec<String> = line.split_whitespace().map(String::from).collect();
            let opts = super::options::parse(&words).ok()?;
            Some(crate::verify::verbs::Typed {
                verb: opts.verb.clone(),
                arg: opts.arg.clone(),
                presets: super::repos::presets_as_built(&opts),
            })
        })
        .collect()
}

/// A harness file no other QML file names and nothing loads by URL is a
/// branch no verb reaches (rules-refs/app-ui.md「ハーネスの枝は動詞から到達
/// できる形で足す」). Read off the code: a name in a comment reaches
/// nothing.
fn unreached_harness_files(harness: &Harness) -> Vec<String> {
    let mentions: Vec<(&String, String)> = harness
        .mentions
        .iter()
        .map(|(other, text)| {
            (
                other,
                crate::lex::blanked(text, crate::lex::Lang::Qml, true).0,
            )
        })
        .collect();
    let mut out = Vec::new();
    for file in &harness.files {
        let stem = crate::gate::stem_of(file);
        let named = mentions
            .iter()
            .any(|(other, code)| *other != file && names_type(code, &stem));
        if !named {
            out.push(format!(
                "{file} is named by no other QML file and loaded by nothing — a harness branch \
                 no verb reaches, so no gate ever runs it"
            ));
        }
    }
    out
}

/// Whether `text` holds `stem` as a word of its own — a type declared,
/// a singleton read (`Awaited.all`), a property typed, a part named by
/// URL — and not as part of a longer name.
fn names_type(text: &str, stem: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(stem).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + stem.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// Every verb the window answers has to be one the page defers: the page
/// dispatches every run it claims, the window's verbs too, and completes a
/// verb that is not deferred itself — at the dispatch if one of its families
/// takes the verb, at the write barrier if `isWriteAct` names it. Only a
/// verb neither reaches is passed on (`auto_act unknown=`), so a gap here
/// is silent until the verb moves into a page family or onto the write
/// list, and then the shot runs ahead of the window's verb.
///
/// Refused as well where the check would hold nothing: a window file read
/// short, no window verb read at all, or a ledger it cannot read.
fn undeferred_window_verbs(harness: &Harness) -> Vec<String> {
    let mut out: Vec<String> = harness
        .window_unread
        .iter()
        .map(|stop| {
            format!(
                "{stop}, so the verbs past it are not known and none of them is held \
                 against `defersCompletion`"
            )
        })
        .collect();
    let deferred = match &harness.deferred {
        Ok(deferred) => deferred,
        Err(why) => {
            out.push(format!(
                "{LEDGER}: {why} — the window's verbs cannot be held against it"
            ));
            return out;
        }
    };
    if harness.window_verbs.is_empty() {
        out.push(format!(
            "no `{WINDOW_STEM}*` file under {HARNESS} compares `autoAct` against a verb — \
             the window's half is not read where this expects it, so nothing is held \
             against `defersCompletion`"
        ));
    }
    out.extend(
        harness
            .window_verbs
            .iter()
            .filter(|(verb, _)| !deferred.contains(*verb))
            .map(|(verb, file)| {
                format!(
                    "{file} answers `{verb}`, which `defersCompletion` ({LEDGER}) does not \
                     name — the page would complete it ahead of the window once a page family \
                     or `isWriteAct` takes it: add it to the list"
                )
            }),
    );
    out
}

/// One QML text with its comments blanked, a comparison or a list entry in
/// a comment answering nothing. `Err` names the line the reading stopped
/// at: what stands past it is not known.
fn code_of(text: &str) -> Result<String, String> {
    let (code, unread) = crate::lex::blanked(text, crate::lex::Lang::Qml, true);
    match unread {
        None => Ok(code),
        Some(unread) => {
            let line = text.chars().take(unread.at).filter(|c| *c == '\n').count() + 1;
            Err(format!("read short at line {line} ({})", unread.what))
        }
    }
}

/// The names one window file compares `autoAct` (or a local `act`)
/// against, read off its code.
fn window_verbs_in(text: &str) -> Result<BTreeSet<String>, String> {
    Ok(verbs_in(&code_of(text)?))
}

/// The names `defersCompletion` lists, read off the code from its head to
/// the `.indexOf(` that closes its list.
fn deferred_in(text: &str) -> Result<BTreeSet<String>, String> {
    let code = code_of(text)?;
    let missing = || {
        "holds no `defersCompletion` list this can read (from its head to the `.indexOf(` \
         that ends it)"
            .to_string()
    };
    let head = code
        .find("function defersCompletion(")
        .ok_or_else(missing)?;
    let body = &code[head..];
    let end = body.find(".indexOf(").ok_or_else(missing)?;
    Ok(words_in(&body[..end]))
}

/// Whether `file` is one of the window's half of the harness.
fn is_window_file(file: &str) -> bool {
    crate::gate::stem_of(file).starts_with(WINDOW_STEM)
}

struct Harness {
    /// Names a dispatch asks for outright.
    answered: BTreeSet<String>,
    /// The names the window's files compare `autoAct` (or a local `act`)
    /// against, each with the first file that does.
    window_verbs: BTreeMap<String, String>,
    /// The window's files read short, each with where it stopped.
    window_unread: Vec<String>,
    /// What [`LEDGER`]'s `defersCompletion` names, or why it cannot be
    /// read.
    deferred: Result<BTreeSet<String>, String>,
    /// Every kebab-case word in the harness, lists
    /// (`AutoActDriver.fileRowActs`) and completion tables included.
    mentioned: BTreeSet<String>,
    /// The harness's files.
    files: Vec<String>,
    /// Every QML file of the app with its text, for the question of who
    /// names a harness file.
    mentions: Vec<(String, String)>,
}

fn harness(root: &Path) -> Result<Harness, String> {
    let mut harness = Harness {
        answered: BTreeSet::new(),
        window_verbs: BTreeMap::new(),
        window_unread: Vec::new(),
        deferred: Err("is not among the harness's files".to_string()),
        mentioned: BTreeSet::new(),
        files: Vec::new(),
        mentions: Vec::new(),
    };
    let mut assigned = BTreeSet::new();
    for file in crate::gate::qml_files(root)? {
        let text = std::fs::read_to_string(root.join(&file))
            .map_err(|e| format!("could not read {file}: {e}"))?;
        harness.mentions.push((file.clone(), text.clone()));
        if !file.starts_with(HARNESS) {
            continue;
        }
        if file == LEDGER {
            harness.deferred = deferred_in(&text);
        }
        if is_window_file(&file) {
            match window_verbs_in(&text) {
                Ok(verbs) => {
                    for verb in verbs {
                        harness
                            .window_verbs
                            .entry(verb)
                            .or_insert_with(|| file.clone());
                    }
                }
                Err(why) => harness.window_unread.push(format!("{file} is {why}")),
            }
        }
        harness.files.push(file);
        harness.answered.extend(verbs_in(&text));
        assigned.extend(stages_in(&text));
        harness.mentioned.extend(words_in(&text));
    }
    if harness.files.is_empty() {
        // Otherwise every registered line would read as stale.
        return Err(format!(
            "no QML under {HARNESS} — the harness is not where this expects it, so nothing \
             here can be said about which verbs are answered"
        ));
    }
    // A name a driver writes into its own `act` is a stage of one verb,
    // not a verb (`settled.act = "diff-find-step-moved"`).
    harness.answered.retain(|verb| !assigned.contains(verb));
    harness
        .window_verbs
        .retain(|verb, _| !assigned.contains(verb));
    Ok(harness)
}

/// The names `act` (the page's families) or `autoAct` (the window's) is
/// compared against in one file, read off the last word so
/// `planOpenTimer.act` counts too.
fn verbs_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices("===") {
        let asked = text[..at]
            .trim_end()
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
            .next()
            .unwrap_or_default();
        let asked = asked.rsplit('.').next().unwrap_or_default();
        if asked != "act" && asked != "autoAct" {
            continue;
        }
        let Some(rest) = text[at + 3..].trim_start().strip_prefix('"') else {
            continue;
        };
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = &rest[..end];
        if is_verb(name) {
            found.insert(name.to_string());
        }
    }
    found
}

/// The names a driver assigns to an `act` of its own (`x.act = "name"`,
/// one `=`): the stages of a verb, compared against later as a verb is.
fn stages_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (at, _) in text.match_indices(".act = \"") {
        let rest = &text[at + ".act = \"".len()..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let name = &rest[..end];
        if is_verb(name) {
            found.insert(name.to_string());
        }
    }
    found
}

/// Every quoted word in one file that could be a verb name, wherever it
/// stands.
fn words_in(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else {
            break;
        };
        if is_verb(&after[..close]) {
            found.insert(after[..close].to_string());
        }
        rest = &after[close + 1..];
    }
    found
}

/// The shape of a verb name: kebab-case with a letter in it. Argument
/// words (`"go"`) share it; only what they are compared with sets them
/// apart ([`verbs_in`]).
fn is_verb(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && name.contains(|c: char| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        deferred_in, is_verb, is_window_file, stages_in, undeferred_window_verbs, verbs_in,
        window_verbs_in, words_in,
    };

    /// Read past the list's end, the next function's names would count as
    /// deferred; read with its comments, a name the list only talks about
    /// would.
    #[test]
    fn the_ledger_is_read_from_its_head_to_the_end_of_its_list() {
        let text = r#"
            function isWriteAct(act) {
                return ["commit"].indexOf(act) >= 0
            }
            function defersCompletion(act) {
                return ["publish", "tab-drag",
                        // "said-only" is no entry
                        "worktree-add"].indexOf(act) >= 0
            }
            function owesStatus(act) {
                return ["amend"].indexOf(act) >= 0
            }
        "#;
        assert_eq!(
            deferred_in(text).map(|set| set.into_iter().collect::<Vec<_>>()),
            Ok(vec![
                "publish".to_string(),
                "tab-drag".to_string(),
                "worktree-add".to_string()
            ])
        );
        assert!(deferred_in("function isWriteAct(act) {}").is_err());
    }

    /// A comparison in a comment answers nothing; a local `act` compared
    /// is the window's verb as much as `Harness.autoAct` is.
    #[test]
    fn a_window_file_answers_what_its_code_compares() {
        let text = r#"
            // running: Harness.autoAct === "said-only"
            running: Harness.autoAct === "tab-drag"
            function openOn(act) { return act === "clone-go" }
        "#;
        assert_eq!(
            window_verbs_in(text).map(|set| set.into_iter().collect::<Vec<_>>()),
            Ok(vec!["clone-go".to_string(), "tab-drag".to_string()])
        );
        let short = window_verbs_in("running: Harness.autoAct === \"tab-drag\"\n/* open");
        assert!(short.is_err_and(|why| why.contains("line 2")));
    }

    #[test]
    fn the_window_files_are_the_ones_named_for_it() {
        assert!(is_window_file(
            "crates/platitude-app/src/auto/WindowTabActs.qml"
        ));
        assert!(is_window_file(
            "crates/platitude-app/src/auto/WindowStage.qml"
        ));
        assert!(!is_window_file(
            "crates/platitude-app/src/auto/AutoActDriver.qml"
        ));
        assert!(!is_window_file(
            "crates/platitude-app/src/auto/PageWindowProbe.qml"
        ));
    }

    fn holding(window_verbs: &[(&str, &str)], deferred: &[&str]) -> super::Harness {
        super::Harness {
            answered: BTreeSet::new(),
            window_verbs: window_verbs
                .iter()
                .map(|(verb, file)| (verb.to_string(), file.to_string()))
                .collect(),
            window_unread: Vec::new(),
            deferred: Ok(deferred.iter().map(|verb| verb.to_string()).collect()),
            mentioned: BTreeSet::new(),
            files: Vec::new(),
            mentions: Vec::new(),
        }
    }

    #[test]
    fn a_window_verb_off_the_ledger_is_named_with_its_file() {
        let harness = holding(
            &[
                ("tab-drag", "WindowTabActs.qml"),
                ("band", "WindowBandActs.qml"),
            ],
            &["band"],
        );
        let out = undeferred_window_verbs(&harness);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].starts_with("WindowTabActs.qml answers `tab-drag`"));
        assert!(undeferred_window_verbs(&holding(&[("band", "W.qml")], &["band"])).is_empty());
    }

    /// Each a reading that would hold nothing, and pass for it.
    #[test]
    fn a_check_that_holds_nothing_is_refused() {
        assert_eq!(undeferred_window_verbs(&holding(&[], &["band"])).len(), 1);
        let mut short = holding(&[("band", "W.qml")], &["band"]);
        short
            .window_unread
            .push("W2.qml is read short at line 3".to_string());
        assert_eq!(undeferred_window_verbs(&short).len(), 1);
        let mut no_ledger = holding(&[("band", "W.qml")], &[]);
        no_ledger.deferred = Err("holds no list".to_string());
        assert_eq!(undeferred_window_verbs(&no_ledger).len(), 1);
    }

    /// Missed, a verb dispatched only this way would drop out of the count
    /// of answered verbs.
    #[test]
    fn an_act_is_asked_for_through_its_holder_too() {
        let text = r#"
            if (planOpenTimer.act === "rebase-plan") { return }
            if (act === "commit") { return }
            if (Harness.autoAct === "band") { return }
            if (mode === "after") { return }
        "#;
        let found = verbs_in(text);
        assert!(found.contains("rebase-plan"));
        assert!(found.contains("commit"));
        assert!(found.contains("band"));
        assert!(!found.contains("after"), "{found:?}");
    }

    /// A stage a driver writes into its own `act` is compared against
    /// like a verb and is none.
    #[test]
    fn a_stage_assigned_to_an_act_is_no_verb() {
        let text = r#"
            settled.act = "diff-find-step-moved"
            if (settled.act === "diff-find-step-moved") { return }
        "#;
        assert!(verbs_in(text).contains("diff-find-step-moved"));
        assert_eq!(
            stages_in(text).into_iter().collect::<Vec<_>>(),
            ["diff-find-step-moved"]
        );
    }

    #[test]
    fn a_type_is_named_as_a_word_and_not_inside_a_longer_one() {
        assert!(super::names_type("Awaited.all(\"x\", {})", "Awaited"));
        assert!(super::names_type(
            "part: \"PageHarness.qml\"",
            "PageHarness"
        ));
        assert!(super::names_type("property TabProbe probe", "TabProbe"));
        assert!(!super::names_type("AutoActDriverFoo {}", "AutoActDriver"));
        let in_a_comment = crate::lex::blanked(
            "// see PageHarness.qml\nItem {}",
            crate::lex::Lang::Qml,
            true,
        )
        .0;
        assert!(
            !super::names_type(&in_a_comment, "PageHarness"),
            "a comment reaches nothing"
        );
        assert!(!super::names_type("myAwaited", "Awaited"));
    }

    #[test]
    fn every_quoted_kebab_word_counts_as_mentioned() {
        let found = words_in(
            r#"readonly property var acts: ["stage-many", "file-menu"]; x = "Not-A-Verb""#,
        );
        assert!(found.contains("stage-many") && found.contains("file-menu"));
        assert!(!found.contains("Not-A-Verb"));
        assert!(is_verb("a1-b") && !is_verb("123") && !is_verb(""));
    }
}
