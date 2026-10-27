//! The run's contract: the structured lines the harness says under the
//! run's own id (`auto/Awaited.qml`, `auto/WindowCensus.qml`), read as the
//! sequence of events they are and held to the one order a run has — its
//! checkpoints in order, one completion, then its pictures, each saved
//! after the completion, each walked, each claim read at it, and the
//! census named right after its walk. A run passes on this as much as on
//! its must-say line: a picture nobody claimed, a claim never read, a
//! claim that did not hold, a checkpoint reached twice, a part the row
//! promised and the picture lacks, a completion or a picture on a page
//! still arriving, a completion said twice or counting checkpoints it did
//! not have, a picture before the completion, a picture asked for and not
//! saved or saved without being asked for — each is a breach, named
//! (反映前テストの機械化 §検証時点). What a line says never stands in
//! place of what an earlier one said: the first completion stands, a
//! second is a breach, and a breach found while reading is not taken
//! back by a line that follows.

use std::collections::BTreeSet;

/// What one `pictured` line said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pictured {
    pub artifact: String,
    pub name: String,
    /// `true`, `false`, or `error` for a predicate that threw.
    pub holds: String,
    /// The rest of the line (`gained=` / `lost=` for the window's claim).
    pub detail: String,
    /// The page's reads still on their way at the picture (`page=`), for
    /// the window's claim: `Some([])` for a settled page, `None` where the
    /// line said nothing of it.
    pub page: Option<Vec<String>>,
}

/// The structured lines of one run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Said {
    /// The verb the first completion named, once it came.
    pub completed: Option<String>,
    /// How many checkpoints that completion counted.
    pub completed_after: usize,
    /// The page's reads still on their way when the verb completed
    /// (`PageSettled.missing`), and the ones the verb said it would
    /// complete over (`Awaited.arriving`).
    pub arriving: Vec<String>,
    pub expected: Vec<String>,
    /// Every checkpoint, in the order said.
    pub checkpoints: Vec<String>,
    pub pictured: Vec<Pictured>,
    /// The census at the app's picture, once it came.
    pub census: Option<BTreeSet<String>>,
    /// The pictures on disk, by artifact (`app` / `overlay` / `scene`).
    pub saved: BTreeSet<String>,
    /// The pictures the run asked for (`pictures … wanted=`), once said.
    pub wanted: Option<Vec<String>>,
    /// Each picture's save, as the run said it (`picture … saved=`), in order.
    pub pictures: Vec<(String, bool)>,
    /// Lines carrying another run's id: a log read against the wrong run.
    pub of_another_run: usize,
    /// The breaches of the sequence, found as the lines were read, in
    /// the order found.
    pub sequence: Vec<String>,
}

/// A comma-separated list of names, as the harness joins them.
fn names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect()
}

/// The value after `key=` in a line of `key=value` words.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|word| {
        word.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

/// The artifact a `… saved=true` line of the picture's own (Qt's handler, no `run=`) reports on disk.
fn saved_artifact(line: &str) -> Option<&'static str> {
    if field(line, "run").is_some() {
        None
    } else if line.contains("screenshot saved=true") {
        Some("app")
    } else if line.contains("overlay saved=true") {
        Some("overlay")
    } else if line.contains("scene saved=true") {
        Some("scene")
    } else {
        None
    }
}

/// What the run's lines say, read in order against `run` — the run's
/// id, which every structured line carries — and held to the order a run
/// has as they are read (`Said::sequence`). The lines come in the order
/// the app wrote them: every one goes through its stderr, the harness's
/// through `tracing` and the pictures' through Qt's own handler, each
/// written whole as it is said.
pub(super) fn read(lines: &[String], run: &str) -> Said {
    let mut said = Said::default();
    // A `census run=` of this run came, and the `census=` naming its parts has not.
    let mut census_due = false;
    for line in lines {
        if let Some(artifact) = saved_artifact(line) {
            if said.completed.is_none() {
                said.sequence
                    .push(format!("{artifact}.png was saved before the completion"));
            }
            said.saved.insert(artifact.to_string());
            continue;
        }
        // The harness's own line (`WindowCensus.reportAt`): the ceiling's `shot state … census=settled` is not it.
        if let Some(parts) = line.split("bench: census=").nth(1) {
            if !census_due {
                said.sequence.push(
                    "a census named its parts with no `census run=` of this run before it"
                        .to_string(),
                );
                continue;
            }
            census_due = false;
            said.census = Some(names(parts).into_iter().collect());
            continue;
        }
        let Some(kind) = [
            "checkpoint ",
            "pictured ",
            "pictures ",
            "picture ",
            "complete ",
            "census ",
        ]
        .into_iter()
        .find(|kind| line.contains(kind) && field(line, "run").is_some()) else {
            continue;
        };
        if field(line, "run") != Some(run) {
            said.of_another_run += 1;
            continue;
        }
        if census_due {
            said.sequence.push(
                "the census walked and named no parts: `census=` did not follow `census run=`"
                    .to_string(),
            );
            census_due = false;
        }
        match kind {
            "checkpoint " => checkpoint(&mut said, line),
            "complete " => complete(&mut said, line),
            "pictured " => pictured(&mut said, line),
            "pictures " => pictures(&mut said, line),
            "picture " => picture(&mut said, line),
            _ => {
                if said.completed.is_none() {
                    said.sequence.push(
                        "the window was walked for a picture before the completion".to_string(),
                    );
                }
                census_due = true;
            }
        }
    }
    if census_due {
        said.sequence.push(
            "the census walked and named no parts: `census=` did not follow `census run=`"
                .to_string(),
        );
    }
    said
}

/// A `checkpoint` line: the next in `seq`, and before the completion.
fn checkpoint(said: &mut Said, line: &str) {
    let name = field(line, "name").unwrap_or("?").to_string();
    if said.completed.is_some() {
        said.sequence
            .push(format!("the checkpoint `{name}` came after the completion"));
    }
    let due = said.checkpoints.len() + 1;
    let seq = field(line, "seq").and_then(|n| n.parse::<usize>().ok());
    if seq != Some(due) {
        said.sequence.push(format!(
            "the checkpoint `{name}` came with seq={} where {due} was due",
            field(line, "seq").unwrap_or("none")
        ));
    }
    said.checkpoints.push(name);
}

/// A `complete` line: the first stands, and counts the checkpoints
/// reached before it.
fn complete(said: &mut Said, line: &str) {
    if said.completed.is_some() {
        said.sequence
            .push("the completion was said twice; the first stands".to_string());
        return;
    }
    let counted = field(line, "checkpoints")
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    if counted != said.checkpoints.len() {
        said.sequence.push(format!(
            "the completion counted {counted} checkpoint(s) where {} had been reached",
            said.checkpoints.len()
        ));
    }
    said.completed = Some(field(line, "verb").unwrap_or("?").to_string());
    said.completed_after = counted;
    said.arriving = field(line, "page")
        .and_then(|page| page.strip_prefix("arriving:"))
        .map(names)
        .unwrap_or_default();
    said.expected = field(line, "expected").map(names).unwrap_or_default();
}

/// A `pictured` line: a claim read at a picture, after the completion.
fn pictured(said: &mut Said, line: &str) {
    let (Some(artifact), Some(name), Some(holds)) = (
        field(line, "artifact"),
        field(line, "name"),
        field(line, "holds"),
    ) else {
        return;
    };
    if said.completed.is_none() {
        said.sequence.push(format!(
            "the claim `{name}` was read at {artifact}.png before the completion"
        ));
    }
    let detail = line
        .split_once(" holds=")
        .map(|(_, rest)| rest.split_once(' ').map_or("", |(_, d)| d))
        .unwrap_or("")
        .split_whitespace()
        .filter(|word| !word.starts_with("page="))
        .collect::<Vec<_>>()
        .join(" ");
    let page = field(line, "page").map(|page| {
        page.strip_prefix("arriving:")
            .map(names)
            .unwrap_or_default()
    });
    said.pictured.push(Pictured {
        artifact: artifact.to_string(),
        name: name.to_string(),
        holds: holds.to_string(),
        detail,
        page,
    });
}

/// A `pictures` line: the pictures the run asks for, said once, after the completion.
fn pictures(said: &mut Said, line: &str) {
    if said.completed.is_none() {
        said.sequence
            .push("the pictures were asked for before the completion".to_string());
    }
    if said.wanted.is_some() {
        said.sequence
            .push("the pictures were asked for twice; the first asking stands".to_string());
        return;
    }
    said.wanted = Some(field(line, "wanted").map(names).unwrap_or_default());
}

/// A `picture` line: one picture saved or not, after the completion.
fn picture(said: &mut Said, line: &str) {
    let (Some(artifact), Some(saved)) = (field(line, "artifact"), field(line, "saved")) else {
        return;
    };
    if said.completed.is_none() {
        said.sequence
            .push(format!("{artifact}.png was saved before the completion"));
    }
    said.pictures.push((artifact.to_string(), saved == "true"));
}

/// What the run owes and did not deliver. `claims` are the names the
/// verb's row requires at the app's picture; `declared` is what the
/// line's registered row promises to show, `None` for a run nobody could
/// type again.
pub(super) fn breaches(
    said: &Said,
    claims: &[&str],
    declared: Option<&BTreeSet<String>>,
) -> Vec<String> {
    let mut out = said.sequence.clone();
    if said.of_another_run > 0 {
        out.push(format!(
            "{} line(s) of another run were in this run's output",
            said.of_another_run
        ));
    }
    if said.saved.is_empty() {
        // Nothing was pictured: the reds for that are the ceiling's and the
        // exit's, said elsewhere.
        return out;
    }
    match &said.completed {
        None => out.push("the pictures came with no completion said by name".to_string()),
        Some(_) if said.completed_after == 0 => out.push(
            "the verb completed having reached no checkpoint — it named nothing it waited for \
             (`Awaited.all` / `Awaited.at`)"
                .to_string(),
        ),
        Some(_) => {}
    }
    out.extend(arriving_breach(said));
    out.extend(picture_breaches(said));
    let mut seen = BTreeSet::new();
    for name in &said.checkpoints {
        if !seen.insert(name) {
            out.push(format!("the checkpoint `{name}` was reached twice"));
        }
    }
    for artifact in &said.saved {
        match said
            .pictured
            .iter()
            .find(|p| p.name == "window" && &p.artifact == artifact)
        {
            None => out.push(format!(
                "{artifact}.png was saved and the window's claim never read at it"
            )),
            Some(p) if p.holds != "true" => out.push(format!(
                "the window moved between the completion and {artifact}.png ({})",
                p.detail
            )),
            Some(_) => {}
        }
    }
    claim_breaches(said, claims, &mut out);
    match (&said.census, declared) {
        (None, _) => out.push("app.png was saved and the window never walked at it".to_string()),
        (Some(census), Some(declared)) => {
            let unmet: Vec<&String> = declared.iter().filter(|d| !census.contains(*d)).collect();
            if !unmet.is_empty() {
                out.push(format!(
                    "the row promises parts the picture lacks: {}",
                    unmet
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        (Some(_), None) => {}
    }
    out
}

/// The completion on a page still arriving, unless the verb said it would complete over those reads
/// (`expected=`; `*` for every read — a verb photographed during the write it presses).
fn arriving_breach(said: &Said) -> Option<String> {
    let unexpected = unexpected(said, &said.arriving);
    (!unexpected.is_empty()).then(|| {
        format!(
            "the verb completed on a page still arriving ({}) — its report read a page the reads had not \
             finished with (`PageSettled.missing`; a verb that photographs that names it in `Awaited.arriving`)",
            unexpected.join(" ")
        )
    })
}

/// The reads among `terms` the verb did not say it would photograph arriving.
fn unexpected<'a>(said: &Said, terms: &'a [String]) -> Vec<&'a str> {
    let any = said.expected.iter().any(|term| term == "*");
    terms
        .iter()
        .filter(|term| !any && !said.expected.contains(term))
        .map(String::as_str)
        .collect()
}

/// The pictures, each held to the run: the set asked for (`pictures … wanted=`) is the set saved (`picture …
/// saved=true`), nothing more and nothing less, and a save that failed stands whatever came after; and each
/// window's claim says the page at its picture, settled or arriving only in reads the verb named. The parts
/// agreeing between the completion and a picture (`holds=`) does not stand in for the page settled: a read on
/// its way at the picture is a state about to move, parts or no parts.
fn picture_breaches(said: &Said) -> Vec<String> {
    let mut out = Vec::new();
    let said_saved: BTreeSet<&str> = said
        .pictures
        .iter()
        .filter(|(_, saved)| *saved)
        .map(|(artifact, _)| artifact.as_str())
        .collect();
    match &said.wanted {
        None => out.push(
            "the pictures came with no `pictures … wanted=` saying which the run asked for"
                .to_string(),
        ),
        Some(wanted) => {
            for artifact in wanted {
                match said.pictures.iter().find(|(a, _)| a == artifact) {
                    None => out.push(format!(
                        "the run asked for {artifact}.png and never said whether it was saved"
                    )),
                    Some((_, false)) => {
                        out.push(format!(
                            "the run asked for {artifact}.png and could not save it"
                        ));
                    }
                    Some((_, true)) => {}
                }
            }
            for (artifact, _) in &said.pictures {
                if !wanted.contains(artifact) {
                    out.push(format!("{artifact}.png was saved without being asked for"));
                }
            }
        }
    }
    let on_disk: BTreeSet<&str> = said.saved.iter().map(String::as_str).collect();
    if said_saved != on_disk {
        out.push(format!(
            "the pictures the run said it saved ({}) are not the ones on disk ({})",
            said_saved.iter().copied().collect::<Vec<_>>().join(","),
            on_disk.iter().copied().collect::<Vec<_>>().join(",")
        ));
    }
    for p in said.pictured.iter().filter(|p| p.name == "window") {
        match &p.page {
            None => out.push(format!(
                "the window's claim at {}.png said nothing of the page",
                p.artifact
            )),
            Some(terms) => {
                let unexpected = unexpected(said, terms);
                if !unexpected.is_empty() {
                    out.push(format!(
                        "{}.png was taken on a page still arriving ({}) — a state about to move, whatever parts \
                         it shows (`WindowCensus.reportAt`; a verb that photographs that names it in \
                         `Awaited.arriving`)",
                        p.artifact,
                        unexpected.join(" ")
                    ));
                }
            }
        }
    }
    out
}

/// The claims the row requires (`claims`) read at the app's picture and holding at every picture, and the claims
/// the verb placed on its own holding wherever they were read.
fn claim_breaches(said: &Said, claims: &[&str], out: &mut Vec<String>) {
    for claim in claims {
        let read: Vec<&Pictured> = said.pictured.iter().filter(|p| &p.name == claim).collect();
        if !read.iter().any(|p| p.artifact == "app") {
            out.push(format!("the claim `{claim}` was never read at app.png"));
        }
        for p in read {
            match p.holds.as_str() {
                "true" => {}
                "error" => out.push(format!(
                    "the claim `{claim}` could not be read at {}.png",
                    p.artifact
                )),
                _ => out.push(format!(
                    "the claim `{claim}` did not hold at {}.png",
                    p.artifact
                )),
            }
        }
    }
    for p in &said.pictured {
        if p.name != "window" && !claims.contains(&p.name.as_str()) && p.holds != "true" {
            out.push(format!(
                "the claim `{}` {} at {}.png",
                p.name,
                if p.holds == "error" {
                    "could not be read"
                } else {
                    "did not hold"
                },
                p.artifact
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Said, breaches, read};
    use std::collections::BTreeSet;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    /// A run as the harness says one, in order: the checkpoints, the
    /// completion, then each picture saved, its claims read, and the
    /// window walked and its census named at the app's.
    const WELL: &str = "\
INFO bench: checkpoint run=r1 name=card_latch_row seq=1
INFO bench: checkpoint run=r1 name=card_latch_back seq=2
INFO bench: complete run=r1 verb=card-latch checkpoints=2 page=settled
INFO bench: pictures run=r1 wanted=app,overlay
screenshot saved=true ink=0 path=x/app.png
INFO bench: picture run=r1 artifact=app saved=true
INFO bench: pictured run=r1 artifact=app name=card_latch holds=true
INFO bench: pictured run=r1 artifact=app name=window holds=true gained=none lost=none page=settled
INFO bench: census run=r1 page=settled state=open
INFO bench: census=Main,RepoPage,AppCardFace
overlay saved=true popups=1 yield=false
INFO bench: picture run=r1 artifact=overlay saved=true
INFO bench: pictured run=r1 artifact=overlay name=card_latch holds=true
INFO bench: pictured run=r1 artifact=overlay name=window holds=true gained=none lost=none page=settled
";

    fn declared(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    /// `WELL` with one of its lines moved to the front.
    fn moved_first(line: &str) -> String {
        format!("{line}\n{}", WELL.replace(&format!("{line}\n"), ""))
    }

    #[test]
    fn a_run_that_said_everything_its_row_asks_breaches_nothing() {
        let said = read(&lines(WELL), "r1");
        assert_eq!(said.completed.as_deref(), Some("card-latch"));
        assert_eq!(said.checkpoints, ["card_latch_row", "card_latch_back"]);
        assert_eq!(said.saved.len(), 2);
        assert!(said.sequence.is_empty(), "{:?}", said.sequence);
        assert!(
            breaches(
                &said,
                &["card_latch"],
                Some(&declared(&["RepoPage", "AppCardFace"]))
            )
            .is_empty()
        );
    }

    /// The sequence, held: each event taken away, said again or moved
    /// out of its place is a breach of its own, and a breach found is
    /// not taken back by what follows.
    #[test]
    fn an_event_dropped_repeated_or_out_of_order_is_a_breach_that_stands() {
        let dropped = WELL.replace(
            "INFO bench: checkpoint run=r1 name=card_latch_back seq=2\n",
            "",
        );
        assert_eq!(
            read(&lines(&dropped), "r1").sequence,
            ["the completion counted 2 checkpoint(s) where 1 had been reached"]
        );
        let twice = WELL.replace(
            "INFO bench: complete run=r1 verb=card-latch checkpoints=2 page=settled\n",
            "INFO bench: complete run=r1 verb=card-latch checkpoints=2 page=settled\n\
             INFO bench: complete run=r1 verb=other checkpoints=2 page=settled\n",
        );
        let said = read(&lines(&twice), "r1");
        assert_eq!(
            said.sequence,
            ["the completion was said twice; the first stands"]
        );
        assert_eq!(said.completed.as_deref(), Some("card-latch"));
        assert!(
            !breaches(&said, &[], None).is_empty(),
            "nothing after it takes a breach back"
        );
        let misnumbered = WELL.replace("name=card_latch_back seq=2", "name=card_latch_back seq=3");
        assert_eq!(
            read(&lines(&misnumbered), "r1").sequence,
            ["the checkpoint `card_latch_back` came with seq=3 where 2 was due"]
        );
        let late = format!("{WELL}INFO bench: checkpoint run=r1 name=card_latch_after seq=3\n");
        assert_eq!(
            read(&lines(&late), "r1").sequence,
            ["the checkpoint `card_latch_after` came after the completion"]
        );
        let early_claim =
            moved_first("INFO bench: pictured run=r1 artifact=app name=card_latch holds=true");
        assert_eq!(
            read(&lines(&early_claim), "r1").sequence,
            ["the claim `card_latch` was read at app.png before the completion"]
        );
        let early_picture = moved_first("screenshot saved=true ink=0 path=x/app.png");
        assert_eq!(
            read(&lines(&early_picture), "r1").sequence,
            ["app.png was saved before the completion"]
        );
        let early_walk = moved_first("INFO bench: census run=r1 page=settled state=open");
        let said = read(&lines(&early_walk), "r1");
        assert_eq!(
            said.sequence,
            [
                "the window was walked for a picture before the completion",
                "the census walked and named no parts: `census=` did not follow `census run=`",
                "a census named its parts with no `census run=` of this run before it",
            ]
        );
        let unnamed = WELL.replace("INFO bench: census=Main,RepoPage,AppCardFace\n", "");
        assert_eq!(
            read(&lines(&unnamed), "r1").sequence,
            ["the census walked and named no parts: `census=` did not follow `census run=`"]
        );
    }

    #[test]
    fn a_required_claim_never_read_is_a_breach_and_so_is_one_that_fell() {
        let without = WELL.replace("name=card_latch holds=true", "name=other holds=true");
        let breaches_ = breaches(&read(&lines(&without), "r1"), &["card_latch"], None);
        assert_eq!(
            breaches_,
            ["the claim `card_latch` was never read at app.png"]
        );
        let fell = WELL.replace(
            "artifact=overlay name=card_latch holds=true",
            "artifact=overlay name=card_latch holds=false",
        );
        let breaches_ = breaches(&read(&lines(&fell), "r1"), &["card_latch"], None);
        assert_eq!(
            breaches_,
            ["the claim `card_latch` did not hold at overlay.png"]
        );
    }

    #[test]
    fn a_picture_without_the_windows_claim_or_with_a_moved_window_is_a_breach() {
        let unread = WELL.replace(
            "INFO bench: pictured run=r1 artifact=overlay name=window holds=true gained=none lost=none page=settled\n",
            "",
        );
        assert_eq!(
            breaches(&read(&lines(&unread), "r1"), &[], None),
            ["overlay.png was saved and the window's claim never read at it"]
        );
        let moved = WELL.replace(
            "artifact=app name=window holds=true gained=none lost=none",
            "artifact=app name=window holds=false gained=none lost=AppCardFace",
        );
        assert_eq!(
            breaches(&read(&lines(&moved), "r1"), &[], None),
            ["the window moved between the completion and app.png (gained=none lost=AppCardFace)"]
        );
    }

    #[test]
    fn a_completion_on_no_checkpoint_a_repeated_one_and_another_runs_lines_are_breaches() {
        let counted = WELL
            .replace(
                "INFO bench: checkpoint run=r1 name=card_latch_row seq=1\n",
                "",
            )
            .replace(
                "INFO bench: checkpoint run=r1 name=card_latch_back seq=2\n",
                "",
            )
            .replace("checkpoints=2", "checkpoints=0");
        assert!(
            breaches(&read(&lines(&counted), "r1"), &[], None)[0].contains("reached no checkpoint")
        );
        let twice = WELL.replace("name=card_latch_back seq=2", "name=card_latch_row seq=2");
        assert!(
            breaches(&read(&lines(&twice), "r1"), &[], None)
                .iter()
                .any(|b| b == "the checkpoint `card_latch_row` was reached twice")
        );
        let stale =
            format!("{WELL}INFO bench: pictured run=r0 artifact=app name=window holds=false\n");
        let said = read(&lines(&stale), "r1");
        assert_eq!(said.of_another_run, 1);
        assert!(breaches(&said, &[], None)[0].contains("another run"));
    }

    #[test]
    fn a_completion_on_an_arriving_page_is_a_breach_unless_the_verb_said_it_would() {
        let completion = "checkpoints=2 page=settled";
        let arriving = WELL.replace(completion, "checkpoints=2 page=arriving:graph,rows:wip0");
        let said = read(&lines(&arriving), "r1");
        assert_eq!(said.arriving, ["graph", "rows:wip0"]);
        assert_eq!(
            breaches(&said, &[], None),
            [
                "the verb completed on a page still arriving (graph rows:wip0) — its report read a page the reads \
              had not finished with (`PageSettled.missing`; a verb that photographs that names it in \
              `Awaited.arriving`)"
            ]
        );
        let declared = WELL.replace(
            completion,
            "checkpoints=2 page=arriving:graph expected=graph,details",
        );
        assert!(breaches(&read(&lines(&declared), "r1"), &[], None).is_empty());
        let half = WELL.replace(
            completion,
            "checkpoints=2 page=arriving:graph,details expected=graph",
        );
        assert!(breaches(&read(&lines(&half), "r1"), &[], None)[0].contains("arriving (details)"));
        let any = WELL.replace(
            completion,
            "checkpoints=2 page=arriving:graph,details expected=*",
        );
        assert!(breaches(&read(&lines(&any), "r1"), &[], None).is_empty());
    }

    /// A picture is judged on the page as it stood then, as the completion is: settled, or arriving only in reads
    /// the verb named — the parts agreeing says nothing of it. And a claim that says nothing of the page is one.
    #[test]
    fn a_picture_on_an_arriving_page_is_a_breach_unless_the_verb_said_it_would() {
        let overlay = "artifact=overlay name=window holds=true gained=none lost=none page=settled";
        let arriving = WELL.replace(
            overlay,
            "artifact=overlay name=window holds=true gained=none lost=none page=arriving:readings",
        );
        let said = read(&lines(&arriving), "r1");
        assert_eq!(
            said.pictured[3].page.as_deref(),
            Some(&["readings".to_string()][..])
        );
        assert_eq!(
            breaches(&said, &[], None),
            [
                "overlay.png was taken on a page still arriving (readings) — a state about to move, whatever parts \
                 it shows (`WindowCensus.reportAt`; a verb that photographs that names it in `Awaited.arriving`)"
            ]
        );
        let declared = arriving.replace(
            "checkpoints=2 page=settled",
            "checkpoints=2 page=settled expected=readings",
        );
        assert!(breaches(&read(&lines(&declared), "r1"), &[], None).is_empty());
        let unsaid = WELL.replace(
            overlay,
            "artifact=overlay name=window holds=true gained=none lost=none",
        );
        assert_eq!(
            breaches(&read(&lines(&unsaid), "r1"), &[], None),
            ["the window's claim at overlay.png said nothing of the page"]
        );
    }

    /// The pictures a run asks for are the pictures it has to save: one asked for and not saved, or saved
    /// without being asked for, or never said, is a breach — and a save that failed is not taken back by the
    /// pictures after it.
    #[test]
    fn the_pictures_asked_for_are_the_pictures_saved() {
        let scene_owed = WELL.replace("wanted=app,overlay", "wanted=app,overlay,scene");
        assert_eq!(
            breaches(&read(&lines(&scene_owed), "r1"), &[], None),
            ["the run asked for scene.png and never said whether it was saved"]
        );
        let failed = WELL.replace(
            "INFO bench: picture run=r1 artifact=overlay saved=true",
            "INFO bench: picture run=r1 artifact=overlay saved=false",
        );
        let said = read(&lines(&failed), "r1");
        let breaches_ = breaches(&said, &[], None);
        assert!(
            breaches_[0] == "the run asked for overlay.png and could not save it",
            "{breaches_:?}"
        );
        assert!(
            breaches_.iter().any(|b| b.contains("not the ones on disk")),
            "the disk's overlay.png is not the run's: {breaches_:?}"
        );
        let unasked = WELL.replace("wanted=app,overlay", "wanted=app");
        assert_eq!(
            breaches(&read(&lines(&unasked), "r1"), &[], None),
            ["overlay.png was saved without being asked for"]
        );
        let unsaid = WELL.replace("INFO bench: pictures run=r1 wanted=app,overlay\n", "");
        assert_eq!(
            breaches(&read(&lines(&unsaid), "r1"), &[], None),
            ["the pictures came with no `pictures … wanted=` saying which the run asked for"]
        );
        let twice = WELL.replace(
            "INFO bench: pictures run=r1 wanted=app,overlay\n",
            "INFO bench: pictures run=r1 wanted=app,overlay\nINFO bench: pictures run=r1 wanted=app\n",
        );
        assert_eq!(
            read(&lines(&twice), "r1").sequence,
            ["the pictures were asked for twice; the first asking stands"]
        );
        let early = moved_first("INFO bench: pictures run=r1 wanted=app,overlay");
        assert_eq!(
            read(&lines(&early), "r1").sequence,
            ["the pictures were asked for before the completion"]
        );
    }

    #[test]
    fn a_part_the_row_promises_and_the_picture_lacks_is_a_breach_and_an_unwalked_picture_too() {
        let said = read(&lines(WELL), "r1");
        assert_eq!(
            breaches(&said, &[], Some(&declared(&["RepoPage", "SettingsDialog"]))),
            ["the row promises parts the picture lacks: SettingsDialog"]
        );
        let unwalked: Vec<String> = lines(WELL)
            .into_iter()
            .filter(|l| !l.contains("census"))
            .collect();
        assert!(
            breaches(&read(&unwalked, "r1"), &[], None)
                .iter()
                .any(|b| b.contains("never walked"))
        );
        assert!(
            breaches(&Said::default(), &["x"], None).is_empty(),
            "nothing pictured is the ceiling's red"
        );
    }
}
