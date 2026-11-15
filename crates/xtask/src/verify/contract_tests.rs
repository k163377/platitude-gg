use super::contract::{Said, breaches, read};
use std::collections::BTreeSet;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

/// The page's operation that claims and dispatches, as the harness
/// says it: begun, then its two stages.
const ACT: [&str; 3] = [
    "INFO bench: begin run=r1 owner=page op=act gen=1 order=claimed,dispatched",
    "INFO bench: stage run=r1 owner=page op=act gen=1 from=start to=claimed seq=?",
    "INFO bench: stage run=r1 owner=page op=act gen=1 from=claimed to=dispatched seq=?",
];

/// A run as the harness says one, in order: the operations' stages,
/// the completion, then each picture saved, its claims read, and the
/// window walked and its census named at the app's.
const WELL: &str = "\
INFO bench: begin run=r1 owner=page op=act gen=1 order=claimed,dispatched
INFO bench: stage run=r1 owner=page op=act gen=1 from=start to=claimed seq=1
INFO bench: stage run=r1 owner=page op=act gen=1 from=claimed to=dispatched seq=2
INFO bench: begin run=r1 owner=page op=card_latch gen=1
INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=start to=back seq=3
INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=back to=complete seq=4
INFO bench: complete run=r1 verb=card-latch owner=page op=card_latch gen=1 stages=4 page=settled
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

/// The lines of `WELL` after its completion: the pictures.
fn pictures() -> String {
    WELL.split_once("INFO bench: complete ")
        .map(|(_, rest)| rest.split_once('\n').map_or("", |(_, after)| after))
        .unwrap_or("")
        .to_string()
}

fn declared(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_string()).collect()
}

/// `WELL` with one of its lines moved to the front.
fn moved_first(line: &str) -> String {
    format!("{line}\n{}", WELL.replace(&format!("{line}\n"), ""))
}

#[test]
fn only_this_runs_completion_report_can_satisfy_a_judgment() {
    let report =
        "INFO bench: report run=r1 owner=page op=card_latch gen=1 text=card_latch was=true\n";
    let at_report = |extra: &str| {
        WELL.replace(
            "INFO bench: complete ",
            &format!("{extra}INFO bench: complete "),
        )
    };
    let valid = read(&lines(&at_report(report)), "r1");
    assert!(valid.sequence.is_empty(), "{:?}", valid.sequence);
    assert_eq!(valid.reports, ["card_latch was=true"]);
    assert!(valid.satisfies("card_latch was=true"));
    assert!(valid.satisfies("overlay saved=true popups=1 yield=false"));
    let wanted = "overlay saved=true popups=3 yield=true";
    for forged in [
        format!("INFO bench: note {wanted}\n"),
        report.replace("card_latch was=true", wanted),
    ] {
        assert!(
            !read(&lines(&at_report(&forged)), "r1").satisfies(wanted),
            "{forged}"
        );
    }
    for extra in [
        "INFO bench: card_latch was=true\n".to_string(),
        "INFO bench: said run=r1 owner=page op=card_latch gen=1 stage=back: card_latch was=true\n".to_string(),
        report.replace("run=r1", "run=other"),
    ] {
        assert!(read(&lines(&at_report(&extra)), "r1").reports.is_empty(), "{extra}");
    }
    let late = read(&lines(&format!("{WELL}{report}")), "r1");
    assert!(late.reports.is_empty());
    assert!(
        late.sequence
            .iter()
            .any(|why| why.contains("report came after"))
    );
}

#[test]
fn measurements_keep_their_observation_point_and_belong_to_perf_alone() {
    let result = "perf_colour_frame elapsed_ms=12";
    let measurement = format!("INFO bench: measurement run=r1 text={result}\n");
    let perf = WELL.replace("verb=card-latch", "verb=perf");
    let valid = read(&lines(&format!("{measurement}{perf}")), "r1");
    assert!(valid.sequence.is_empty(), "{:?}", valid.sequence);
    assert!(valid.satisfies(result));
    let ordinary = read(&lines(&format!("{measurement}{WELL}")), "r1");
    assert!(!ordinary.satisfies(result));
    for note in [
        format!("INFO bench: {result}\n"),
        measurement.replace("run=r1", "run=other"),
        format!("INFO bench: report run=r1 owner=act op=1 gen=1 text={result}\n"),
    ] {
        assert!(!read(&lines(&format!("{note}{perf}")), "r1").satisfies(result));
    }
    let late = read(&lines(&format!("{perf}\n{measurement}")), "r1");
    assert!(!late.satisfies(result));
    assert!(
        late.sequence
            .iter()
            .any(|line| line.contains("measurement came after"))
    );
}

/// A stage's own lines and the facts kept are the run's history — read
/// as such, never as a claim — and a completion names each witness its
/// report read as it was kept.
#[test]
fn history_is_read_as_history_and_a_completion_names_the_witnesses_it_read() {
    let run = WELL
        .replace(
            "INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=start to=back seq=3\n",
            "INFO bench: witness run=r1 owner=page op=card_latch gen=1 stage=start name=was\n\
             INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=start to=back seq=3\n\
             INFO bench: said run=r1 owner=page op=card_latch gen=1 stage=back: card_latch was=true\n",
        )
        .replace(
            "stages=4 page=settled\n",
            "stages=4 page=settled witnesses=page/card_latch/1/start/was\n",
        );
    let said = read(&lines(&run), "r1");
    assert!(said.sequence.is_empty(), "{:?}", said.sequence);
    assert!(said.reports.is_empty());
    let unkept = run.replace(
        "witnesses=page/card_latch/1/start/was",
        "witnesses=page/card_latch/1/back/was",
    );
    assert!(
        read(&lines(&unkept), "r1")
            .sequence
            .iter()
            .any(|b| b.contains("never kept")),
        "{:?}",
        read(&lines(&unkept), "r1").sequence
    );
}

#[test]
fn a_run_that_said_everything_its_row_asks_breaches_nothing() {
    let said = read(&lines(WELL), "r1");
    assert_eq!(said.completed.as_deref(), Some("card-latch"));
    assert_eq!(said.stages.said, 4);
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

#[test]
fn an_event_dropped_repeated_or_out_of_order_is_a_breach_that_stands() {
    let dropped = WELL.replace(
        "INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=start to=back seq=3\n",
        "",
    );
    let found = read(&lines(&dropped), "r1").sequence;
    assert!(
        found
            .contains(&"the stage line `complete` came with seq=4 where 3 was due".to_string())
            && found.contains(
                &"page/card_latch moved from back to complete where it stood at start"
                    .to_string()
            )
            && found.contains(
                &"the completion counted 4 stage line(s) where 3 had been said".to_string()
            ),
        "{found:?}"
    );
    let twice = WELL.replace(
        "INFO bench: complete run=r1 verb=card-latch owner=page op=card_latch gen=1 stages=4 page=settled\n",
        "INFO bench: complete run=r1 verb=card-latch owner=page op=card_latch gen=1 stages=4 page=settled\n\
         INFO bench: complete run=r1 verb=other owner=page op=card_latch gen=1 stages=4 page=settled\n",
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
    let misnumbered = WELL.replace("to=complete seq=4", "to=complete seq=5");
    assert_eq!(
        read(&lines(&misnumbered), "r1").sequence,
        ["the stage line `complete` came with seq=5 where 4 was due"]
    );
    let late = format!(
        "{WELL}INFO bench: begin run=r1 owner=window op=late gen=1\n\
         INFO bench: stage run=r1 owner=window op=late gen=1 from=start to=pressed seq=5\n"
    );
    assert_eq!(
        read(&lines(&late), "r1").sequence,
        [
            "window/late began generation 1 after the completion",
            "window/late moved to pressed after the completion",
        ]
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

/// A stage's move to `complete` and a breach whose reason names one
/// are never read as a completion.
#[test]
fn a_word_later_in_a_line_never_decides_its_kind() {
    let said = read(&lines(WELL), "r1");
    assert_eq!(said.completed.as_deref(), Some("card-latch"));
    let refused = WELL.replace(
        "INFO bench: complete run=r1",
        "INFO bench: stage_breach run=r1 owner=page op=x gen=1 stage=start why=complete run=r2 twice\n\
         INFO bench: complete run=r1",
    );
    let said = read(&lines(&refused), "r1");
    assert_eq!(said.of_another_run, 0);
    assert_eq!(
        said.sequence,
        [
            "the harness refused a stage: run=r1 owner=page op=x gen=1 stage=start why=complete run=r2 twice"
        ]
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

/// A completion is of an operation's own transition: from a
/// generation that is over, from one standing short of `complete`, or
/// the end a breach made, it is no completion of the verb's.
#[test]
fn a_completion_off_its_operations_chain_and_another_runs_lines_are_breaches() {
    let unmoved = WELL.replace(
        "INFO bench: stage run=r1 owner=page op=card_latch gen=1 from=back to=complete seq=4\n",
        "",
    );
    assert!(
        read(&lines(&unmoved), "r1").sequence.contains(
            &"page/card_latch completed standing at back — no transition to complete came before"
                .to_string()
        )
    );
    let stale = WELL.replace(
        "INFO bench: complete run=r1",
        "INFO bench: begin run=r1 owner=page op=card_latch gen=2\nINFO bench: complete run=r1",
    );
    assert!(
        read(&lines(&stale), "r1").sequence.contains(
            &"page/card_latch completed from generation 1, which is over: generation 2 began"
                .to_string()
        )
    );
    let broken = WELL.replace(
        "owner=page op=card_latch gen=1 stages=4",
        "owner=none op=none gen=0 stages=4",
    );
    assert!(
        read(&lines(&broken), "r1")
            .sequence
            .contains(&"the run was ended by a breach, not completed".to_string())
    );
    let other =
        format!("{WELL}INFO bench: pictured run=r0 artifact=app name=window holds=false\n");
    let said = read(&lines(&other), "r1");
    assert_eq!(said.of_another_run, 1);
    assert!(breaches(&said, &[], None)[0].contains("another run"));
}

/// Two operations with no join between them: every interleaving of
/// their lines, and every observation said between them, passes or
/// fails alike — here, every one passes.
#[test]
fn independent_operations_pass_alike_in_every_interleaving() {
    let window = [
        "INFO bench: begin run=r1 owner=window op=clear gen=1 order=pressed,cleared",
        "INFO bench: stage run=r1 owner=window op=clear gen=1 from=start to=pressed seq=?",
        "INFO bench: stage run=r1 owner=window op=clear gen=1 from=pressed to=cleared seq=?",
    ];
    let noise = "INFO bench: clear_ready waiting=page";
    let mut verdicts = BTreeSet::new();
    for mask in 0u32..(1 << 6) {
        if mask.count_ones() != 3 {
            continue;
        }
        let (mut a, mut w) = (ACT.iter(), window.iter());
        let mut said = Vec::new();
        for bit in 0..6 {
            let next = if mask & (1 << bit) != 0 {
                a.next()
            } else {
                w.next()
            };
            said.push(next.expect("three of each").to_string());
            // The observations between them: said or not, held or not.
            if bit % 2 == 0 {
                said.push(noise.to_string());
                said.push("INFO bench: clear_ready waiting=nothing".to_string());
            }
        }
        let mut seq = 0;
        for line in &mut said {
            if line.contains("seq=?") {
                seq += 1;
                *line = line.replace("seq=?", &format!("seq={seq}"));
            }
        }
        said.push(format!(
            "INFO bench: stage run=r1 owner=window op=clear gen=1 from=cleared to=complete seq={}",
            seq + 1
        ));
        said.push(format!(
            "INFO bench: complete run=r1 verb=commands-clear owner=window op=clear gen=1 stages={} \
             page=settled",
            seq + 1
        ));
        said.extend(lines(&pictures()));
        verdicts.insert(breaches(&read(&said, "r1"), &[], None));
    }
    assert_eq!(verdicts.len(), 1, "{verdicts:?}");
    assert!(verdicts.iter().all(Vec::is_empty), "{verdicts:?}");
}

#[test]
fn a_completion_on_an_arriving_page_is_a_breach_unless_the_verb_said_it_would() {
    let completion = "stages=4 page=settled";
    let arriving = WELL.replace(completion, "stages=4 page=arriving:graph,rows:wip0");
    let said = read(&lines(&arriving), "r1");
    assert_eq!(said.arriving, ["graph", "rows:wip0"]);
    assert_eq!(
        breaches(&said, &[], None),
        [
            "the verb completed on a page still arriving (graph rows:wip0) — its report read a page the reads \
             had not finished with (`PageSettled.missing`; a verb that photographs that declares it in its \
             completion, `StageTimer.complete`)"
        ]
    );
    let declared = WELL.replace(
        completion,
        "stages=4 page=arriving:graph expected=graph,details",
    );
    assert!(breaches(&read(&lines(&declared), "r1"), &[], None).is_empty());
    let half = WELL.replace(
        completion,
        "stages=4 page=arriving:graph,details expected=graph",
    );
    assert!(breaches(&read(&lines(&half), "r1"), &[], None)[0].contains("arriving (details)"));
    let any = WELL.replace(
        completion,
        "stages=4 page=arriving:graph,details expected=*",
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
             it shows (`WindowCensus.reportAt`; a verb that photographs that declares it in its completion, \
             `StageTimer.complete`)"
        ]
    );
    let declared = arriving.replace(
        "stages=4 page=settled",
        "stages=4 page=settled expected=readings",
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
