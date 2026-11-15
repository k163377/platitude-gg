//! The run's contract: the structured lines the harness says under the
//! run's own id (`auto/Awaited.qml`, `auto/WindowCensus.qml`), read as the
//! sequence of events they are. Each operation keeps its own chain of
//! stages ([`super::stages`]), and the run as a whole has one order: its
//! stage lines numbered in turn, one completion, then its pictures, each
//! saved after the completion, each walked, each claim read at it, and the
//! census named right after its walk. A run passes on this as much as on
//! its must-say line: a line out of that order or of another run, a stage
//! the harness refused, a claim unread or fallen, a completion or a
//! picture on a page still arriving, a picture asked for and not saved or
//! saved unasked, a part the row promised and the picture lacks — each is
//! a breach, named (反映前テストの機械化 §検証時点). What a line says never
//! stands in place of what an earlier one said: the first completion
//! stands, a second is a breach, and a breach found while reading is not
//! taken back by a line that follows.

use std::collections::BTreeSet;

use super::stages::Stages;

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
    /// Only the completion route's report, carrying this run's identity and read at its completion.
    pub reports: Vec<String>,
    /// The capture mechanism's own result lines, emitted through Qt's handler rather than Harness.report.
    pub captures: Vec<String>,
    /// Measurements at their own causal boundaries, used only by the performance run's judgment.
    pub measurements: Vec<String>,
    /// The verb the first completion named, once it came.
    pub completed: Option<String>,
    /// The page's reads still on their way when the verb completed
    /// (`PageSettled.missing`), and the ones the completion said it would
    /// complete over (`StageTimer.complete`'s `arriving`).
    pub arriving: Vec<String>,
    pub expected: Vec<String>,
    /// The operations' chains, as their lines moved them.
    pub stages: Stages,
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
    /// The breaches of the sequence, as the lines were read.
    pub sequence: Vec<String>,
}

/// A comma-separated list of names, as the harness joins them.
pub(super) fn names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect()
}

/// The value after `key=` in a line of `key=value` words.
pub(super) fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|word| {
        word.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

/// The artifact a `… saved=true` line of the picture's own (Qt's handler, no `run=`) reports on disk.
fn saved_artifact(line: &str) -> Option<&'static str> {
    let line = capture_text(line)?;
    if line.starts_with("screenshot saved=true") {
        Some("app")
    } else if line.starts_with("overlay saved=true") {
        Some("overlay")
    } else if line.starts_with("scene saved=true") {
        Some("scene")
    } else {
        None
    }
}

/// The capture protocol, optionally prefixed by Qt's `qml:`. A bench note quoting it is not a capture.
pub(super) fn capture_text(line: &str) -> Option<&str> {
    let text = line.strip_prefix("qml: ").unwrap_or(line);
    ["screenshot saved=", "overlay saved=", "scene saved="]
        .iter()
        .any(|prefix| text.starts_with(prefix))
        .then_some(text)
}

impl Said {
    /// Performance judgments read measurements; other judgments read the capture protocol or completion report.
    /// Neither current-state reports nor diagnostic notes substitute for measurements taken at their own boundary.
    pub(super) fn satisfies(&self, wanted: &str) -> bool {
        let source = if self.completed.as_deref() == Some("perf") {
            &self.measurements
        } else if capture_text(wanted).is_some() {
            &self.captures
        } else {
            &self.reports
        };
        source.iter().any(|line| line.contains(wanted))
    }
}

/// The kinds of structured line.
const KINDS: [&str; 12] = [
    "measurement",
    "report",
    "begin",
    "stage",
    "stage_breach",
    "witness",
    "said",
    "complete",
    "pictured",
    "pictures",
    "picture",
    "census",
];

/// Which structured line `line` is: the word right before its first ` run=`
/// (`INFO bench: stage run=…`), so a word the rest of the line carries —
/// `to=complete`, a breach's reason — never decides it.
fn kind_of(line: &str) -> Option<&'static str> {
    let at = line.find(" run=")?;
    let word = line[..at].rsplit(' ').next()?;
    KINDS.iter().copied().find(|kind| *kind == word)
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
        if let Some(text) = capture_text(line) {
            said.captures.push(text.to_string());
        }
        if let Some(artifact) = saved_artifact(line) {
            if said.completed.is_none() {
                said.sequence
                    .push(format!("{artifact}.png was saved before the completion"));
            }
            said.saved.insert(artifact.to_string());
            continue;
        }
        // The harness's own line (`WindowCensus.reportAt`): `census=` as the first word after `bench:`, not a later word.
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
        let Some(kind) = kind_of(line) else {
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
        let completed = said.completed.is_some();
        match kind {
            "begin" => {
                let found = said.stages.begin(line, completed);
                said.sequence.extend(found);
            }
            "stage" => stage(&mut said, line),
            "witness" => {
                let found = said.stages.witness(line, completed);
                said.sequence.extend(found);
            }
            // A stage's own lines: history, held to nothing but the run they name.
            "said" => {}
            "measurement" => {
                if completed {
                    said.sequence
                        .push("a measurement came after the completion".to_string());
                } else if let Some((_, text)) = line.split_once(" text=") {
                    said.measurements.push(text.to_string());
                } else {
                    said.sequence
                        .push("a measurement named no text".to_string());
                }
            }
            "report" => {
                said.sequence.extend(said.stages.completion(line));
                if completed {
                    said.sequence
                        .push("a report came after the completion".to_string());
                } else if let Some((_, text)) = line.split_once(" text=") {
                    said.reports.push(text.to_string());
                } else {
                    said.sequence.push("a report named no text".to_string());
                }
            }
            "stage_breach" => said.sequence.push(format!(
                "the harness refused a stage: {}",
                line.split_once("stage_breach ")
                    .map_or(line.as_str(), |(_, rest)| rest)
            )),
            "complete" => complete(&mut said, line),
            "pictured" => pictured(&mut said, line),
            "pictures" => pictures(&mut said, line),
            "picture" => picture(&mut said, line),
            _ => {
                if !completed {
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

/// A `stage` line: the next in `seq`, and a move of its operation's chain.
fn stage(said: &mut Said, line: &str) {
    let due = said.stages.said + 1;
    let seq = field(line, "seq").and_then(|n| n.parse::<usize>().ok());
    if seq != Some(due) {
        said.sequence.push(format!(
            "the stage line `{}` came with seq={} where {due} was due",
            field(line, "to").unwrap_or("?"),
            field(line, "seq").unwrap_or("none")
        ));
    }
    let found = said.stages.stage(line, said.completed.is_some());
    said.sequence.extend(found);
}

/// A `complete` line: the first stands, counts the stage lines said
/// before it, and comes from its operation's current generation standing
/// at `complete`.
fn complete(said: &mut Said, line: &str) {
    if said.completed.is_some() {
        said.sequence
            .push("the completion was said twice; the first stands".to_string());
        return;
    }
    let counted = field(line, "stages")
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    if counted != said.stages.said {
        said.sequence.push(format!(
            "the completion counted {counted} stage line(s) where {} had been said",
            said.stages.said
        ));
    }
    let found = said.stages.completion(line);
    said.sequence.extend(found);
    said.completed = Some(field(line, "verb").unwrap_or("?").to_string());
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
    if said.completed.is_none() {
        out.push("the pictures came with no completion said by name".to_string());
    }
    out.extend(arriving_breach(said));
    out.extend(picture_breaches(said));
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
             finished with (`PageSettled.missing`; a verb that photographs that declares it in its completion, \
             `StageTimer.complete`)",
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

/// The pictures, each held to the run: the set asked for (`pictures … wanted=`) is the set the run said it
/// saved (`picture … saved=true`) and the set on disk, and a save that failed stands whatever came after; and
/// each window's claim says the page at its picture, settled or arriving only in reads the verb named. The
/// parts agreeing between the completion and a picture (`holds=`) does not stand in for the page settled: a
/// read on its way at the picture is a state about to move, parts or no parts.
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
                         it shows (`WindowCensus.reportAt`; a verb that photographs that declares it in its \
                         completion, `StageTimer.complete`)",
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
