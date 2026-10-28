//! The reach rule held to fixtures of the shapes it reads: the same code
//! in an operation's body, in a helper of its file and in a function of
//! another file is judged alike.

use std::collections::BTreeSet;

/// The operation's type, as the machinery declares it: what each function
/// may be called in, and what its function arguments run in.
const STAGE_TIMER: &str = r#"import QtQuick
Timer {
    id: op
    property string owner: ""
    required property string name
    property var order: []
    // reach(read)
    function at(stage) {
        return Awaited.stageOf(op) === stage
    }
    // reach(beat; 1=read)
    function advance(stage, report) {
        return Awaited.move(op, stage, report)
    }
    // reach(beat; 1=press, 2=read)
    function input(stage, press, report) {
        return Awaited.move(op, stage, report)
    }
    // reach(beat, press)
    function witness(name, value) {
        return Awaited.keep(op, name, value)
    }
    // reach(read)
    function witnessed(name) {
        return Awaited.kept(op, name)
    }
    // reach(beat; 0=spec)
    function complete(spec) {
        return Awaited.move(op, "complete", spec)
    }
}
"#;

const PAGE_STAGE: &str = "import QtQuick\nStageTimer {\n    owner: \"page\"\n}\n";

const AWAITED: &str = r#"pragma Singleton
import QtQuick
QtObject {
    id: awaited
    property var observed: null
    property var records: ({})
    // reach(beat)
    function all(name, terms) {
        awaited.observed = true
        return true
    }
    function stageOf(op) {
        return "start"
    }
    function move(op, stage, report) {
        return true
    }
    function keep(op, name, value) {
        awaited.records[name] = value
        return value
    }
    function kept(op, name) {
        return awaited.records[name]
    }
}
"#;

/// A product pane: one function that reads, one that presses, one signal,
/// and a model of a Rust type.
const PANE: &str = r#"import QtQuick
Item {
    id: pane
    property int chosen: -1
    property ListThing things
    signal picked(int row)
    function rowOf(name) {
        const at = name.length
        return at
    }
    function choose(row) {
        pane.chosen = row
    }
    function rowFor(name) {
        return pane.rowAt(name.length)
    }
    function rowAt(i) {
        return i + 1
    }
}
"#;

/// Another product file whose function of the same name presses.
const OTHER: &str = r#"import QtQuick
ListView {
    id: list
    function rowAt(i) {
        list.positionViewAtIndex(i, ListView.Contain)
        return list.itemAtIndex(i)
    }
}
"#;

const RUST: &str = r#"
#[qobject(Base = QObject)]
impl ListThing {
    #[qslot]
    fn count_of(&self, name: String) -> i32 {
        self.rows.len() as i32
    }
    #[qslot]
    fn take(&mut self, row: i32) {
        self.rows.remove(row as usize);
    }
}
"#;

/// The page's driver: the machinery's file, with the verbs' family in it.
fn driver(common: &str) -> String {
    format!(
        r#"import QtQuick
Item {{
    id: driver
    property Pane pane
{common}
    AutoActFooVerbs {{ driver: driver }}
}}
"#
    )
}

/// A verb family with one operation, its beat and helpers as given.
fn verbs(members: &str, beat: &str) -> String {
    format!(
        r#"import QtQuick
Item {{
    id: acts
    required property var driver
    readonly property var pane: driver.pane
{members}
    PageStage {{
        id: op
        name: "count"
        property int beats: 0
        property bool pressed: false
        onTriggered: {{
{beat}
        }}
    }}
}}
"#
    )
}

/// The rules found over a program of the fixtures above, with `driver`'s
/// common functions and `verbs` as the family.
fn found(driver_text: &str, verbs_text: &str) -> Vec<(String, usize, &'static str)> {
    let harness = vec![
        (
            "src/auto/StageTimer.qml".to_string(),
            STAGE_TIMER.to_string(),
        ),
        ("src/auto/PageStage.qml".to_string(), PAGE_STAGE.to_string()),
        ("src/auto/Awaited.qml".to_string(), AWAITED.to_string()),
        (
            "src/auto/AutoActDriver.qml".to_string(),
            driver_text.to_string(),
        ),
        (
            "src/auto/AutoActFooVerbs.qml".to_string(),
            verbs_text.to_string(),
        ),
    ];
    let product = vec![
        ("src/ui/Pane.qml".to_string(), PANE.to_string()),
        ("src/ui/Other.qml".to_string(), OTHER.to_string()),
    ];
    let machinery = ["Awaited.qml", "StageTimer.qml", "AutoActDriver.qml"];
    super::findings(&harness, &product, &[RUST.to_string()], &machinery)
        .into_iter()
        .map(|f| {
            let file = f.file.rsplit('/').next().unwrap_or_default().to_string();
            (file, f.line, f.rule)
        })
        .collect()
}

fn rules(found: &[(String, usize, &'static str)]) -> BTreeSet<&'static str> {
    found.iter().map(|(_, _, rule)| *rule).collect()
}

const COMPLETE: &str = r#"op.complete({ "when": () => ({}), "report": () => "done" })"#;

/// The counterexample a body-only check let through: a count of beats kept
/// in the operation's own state, held against a number, then the
/// completion. Named alike — a stage kept by hand, and a count of beats —
/// whether the count stands in the body, in a helper of the family's, or in
/// a function of the driver's the operation is handed to.
#[test]
fn counting_two_beats_then_completing_is_named_alike_in_every_form() {
    let counting = format!(
        "            op.beats++\n            if (op.beats < 2)\n                return\n            {COMPLETE}"
    );
    let body = found(&driver(""), &verbs("", &counting));
    let helper = found(
        &driver(""),
        &verbs(
            &format!(
                "    function countThenComplete(op) {{\n        op.beats++\n        if (op.beats < 2)\n            \
                 return\n        {COMPLETE}\n    }}"
            ),
            "            acts.countThenComplete(op)",
        ),
    );
    let common = found(
        &driver(&format!(
            "    function countThenComplete(op) {{\n        op.beats++\n        if (op.beats < 2)\n            \
             return\n        {COMPLETE}\n    }}"
        )),
        &verbs("", "            driver.countThenComplete(op)"),
    );
    let wanted: BTreeSet<&str> = ["beats", "stage"].into_iter().collect();
    assert_eq!(rules(&body), wanted, "{body:?}");
    assert_eq!(rules(&helper), wanted, "{helper:?}");
    assert_eq!(rules(&common), wanted, "{common:?}");
    // Each is named where it stands.
    assert!(
        helper
            .iter()
            .all(|(file, _, _)| file == "AutoActFooVerbs.qml")
    );
    assert!(
        common
            .iter()
            .all(|(file, _, _)| file == "AutoActDriver.qml")
    );
}

/// An input put in from a beat, past the press, is named wherever the call
/// to the product stands; the same input inside the operation's `input` is
/// a press, and stands.
#[test]
fn an_input_outside_the_press_is_named_alike_in_every_form() {
    let body = found(&driver(""), &verbs("", "            pane.choose(2)"));
    let helper = found(
        &driver(""),
        &verbs(
            "    function pick() {\n        pane.choose(2)\n    }",
            "            acts.pick()",
        ),
    );
    let common = found(
        &driver("    function pick() {\n        driver.pane.choose(2)\n    }"),
        &verbs("", "            driver.pick()"),
    );
    let wanted: BTreeSet<&str> = ["input"].into_iter().collect();
    assert_eq!(rules(&body), wanted, "{body:?}");
    assert_eq!(rules(&helper), wanted, "{helper:?}");
    assert_eq!(rules(&common), wanted, "{common:?}");
    let pressed = found(
        &driver(""),
        &verbs("", "            op.input(\"chosen\", () => pane.choose(2))"),
    );
    assert!(pressed.is_empty(), "{pressed:?}");
}

/// A product call is judged by its receiver's type: a slot on `&self`
/// reads, one on `&mut self` presses, a QML function that writes presses,
/// one that only reads reads, and a signal said is an input.
#[test]
fn the_product_answers_by_the_receivers_type() {
    let reads = found(
        &driver(""),
        &verbs(
            "",
            "            if (!Awaited.all(\"x\", { \"n\": pane.things.countOf(\"a\") > pane.rowOf(\"b\") }))\n                \
             return",
        ),
    );
    assert!(reads.is_empty(), "{reads:?}");
    for press in ["pane.things.take(0)", "pane.choose(1)", "pane.picked(1)"] {
        let pressed = found(&driver(""), &verbs("", &format!("            {press}")));
        assert_eq!(
            rules(&pressed),
            ["input"].into_iter().collect(),
            "{press}: {pressed:?}"
        );
    }
}

/// A product function's own calls answer by the file that makes them: a
/// read of its own is a read, though another file's function of the same
/// name presses.
#[test]
fn a_product_functions_own_calls_answer_by_its_own_file() {
    let read = found(
        &driver(""),
        &verbs(
            "",
            "            if (!Awaited.all(\"x\", { \"row\": pane.rowFor(\"a\") > 0 }))\n                return",
        ),
    );
    assert!(read.is_empty(), "{read:?}");
}

/// What a signal's handler hands the machinery to read later is a reading
/// like any other: one carrying a value of the handler's is named.
#[test]
fn a_reading_a_signal_handler_hands_on_is_held_to_reading() {
    let declared = "    // reach(beat, press; 0=read)\n    function reportAtCompletion(report) {\n        \
                    driver.pane = driver.pane\n    }";
    let handed = found(
        &driver(declared),
        &verbs(
            "    Connections {\n        target: pane\n        function onPicked(row) {\n            \
             const n = row + 1\n            driver.reportAtCompletion(() => \"n=\" + n)\n        }\n    }",
            "",
        ),
    );
    assert_eq!(
        rules(&handed),
        ["stale"].into_iter().collect(),
        "{handed:?}"
    );
}

/// A reading reads what is there when it is read: a value carried in from
/// the code it was written in is named; an object named through a local,
/// a witness the generation kept, and the reading's own locals stand.
#[test]
fn a_reading_that_carries_a_value_in_is_named() {
    let carried = found(
        &driver(""),
        &verbs(
            "",
            "            const before = pane.chosen\n            \
             op.complete({ \"when\": () => ({}), \"report\": () => \"was=\" + before })",
        ),
    );
    assert_eq!(
        rules(&carried),
        ["stale"].into_iter().collect(),
        "{carried:?}"
    );
    let witnessed = found(
        &driver(""),
        &verbs(
            "",
            "            const p = pane\n            op.witness(\"before\", pane.chosen)\n            \
             op.complete({ \"when\": () => ({ \"chosen\": p.chosen >= 0 }), \"report\": () => {\n                \
             const now = p.chosen\n                return \"was=\" + op.witnessed(\"before\") + \" now=\" + now\n            } })",
        ),
    );
    assert!(witnessed.is_empty(), "{witnessed:?}");
}

/// A report that answers a value read before it is named alike whether
/// the completion stands in the body, in a helper of the family's handed
/// the value, or in a function of the driver's handed it.
#[test]
fn a_report_carrying_a_value_read_before_is_named_alike_in_every_form() {
    let completing =
        "        op.complete({ \"when\": () => ({}), \"report\": () => \"was=\" + before })";
    let body = found(
        &driver(""),
        &verbs(
            "",
            &format!("            const before = pane.chosen\n    {completing}"),
        ),
    );
    let helper = found(
        &driver(""),
        &verbs(
            &format!("    function completeWith(op, before) {{\n{completing}\n    }}"),
            "            acts.completeWith(op, pane.chosen)",
        ),
    );
    let common = found(
        &driver(&format!(
            "    function completeWith(op, before) {{\n{completing}\n    }}"
        )),
        &verbs("", "            driver.completeWith(op, pane.chosen)"),
    );
    let wanted: BTreeSet<&str> = ["stale"].into_iter().collect();
    assert_eq!(rules(&body), wanted, "{body:?}");
    assert_eq!(rules(&helper), wanted, "{helper:?}");
    assert_eq!(rules(&common), wanted, "{common:?}");
}

/// A change to the harness's state is judged by what it changes, not by
/// the name it is written through: the family's array pushed to directly,
/// through a local naming it, in a helper, through a helper's parameter,
/// through what a helper returns, or by a global that changes its first
/// argument, is a stage kept by hand in a beat and a reading that acts in
/// a report — the same rule each way.
#[test]
fn a_change_to_shared_state_is_named_through_every_alias() {
    let ticks = "    property var ticks: []";
    let helpers = "    function sampledEnough() {\n        const samples = acts.ticks\n        samples.push(true)\n        \
                   return samples.length >= 2\n    }\n    function add(list) {\n        list.push(true)\n    }\n    \
                   function samples() {\n        return acts.ticks\n    }";
    let forms = [
        "            acts.ticks.push(true)",
        "            const samples = acts.ticks\n            samples.push(true)",
        "            if (!acts.sampledEnough())\n                return",
        "            acts.add(acts.ticks)",
        "            acts.samples().push(true)",
        "            const kept = acts.ticks\n            kept[0] = false",
        "            Object.assign(acts.ticks, [true])",
    ];
    for form in forms {
        let beat = found(&driver(""), &verbs(&format!("{ticks}\n{helpers}"), form));
        assert_eq!(
            rules(&beat),
            ["stage"].into_iter().collect(),
            "{form}: {beat:?}"
        );
    }
    let report = found(
        &driver(""),
        &verbs(
            ticks,
            "            op.complete({ \"when\": () => ({}), \"report\": () => {\n                \
             const samples = acts.ticks\n                samples.push(true)\n                \
             return \"n=\" + samples.length\n            } })",
        ),
    );
    assert_eq!(rules(&report), ["read"].into_iter().collect(), "{report:?}");
}

/// A change through a value nothing says the origin of — an element a
/// callback is handed, what a call nobody answers for returns — is not
/// passed as a change to something of the code's own.
#[test]
fn a_change_through_a_value_of_no_known_origin_is_named() {
    let ticks = "    property var ticks: []";
    for beat in [
        "            acts.ticks.forEach(t => t.push(true))",
        "            const got = pane.things.countOf(\"a\")\n            got.push(true)",
    ] {
        let read = found(&driver(""), &verbs(ticks, beat));
        assert!(
            rules(&read).contains("unread") || rules(&read).contains("input"),
            "{beat}: {read:?}"
        );
        assert!(!read.is_empty(), "{beat}");
    }
}

/// A value the code made itself is its own to build: an array a beat or
/// a report fills with lines, a copy of the family's array, an object
/// it gives fields. One made by the beat and filled from a press is a
/// value carried out of the press.
#[test]
fn a_value_the_code_made_is_its_own_to_build() {
    let ticks = "    property var ticks: []";
    let built = found(
        &driver(""),
        &verbs(
            ticks,
            "            const lines = []\n            lines.push(\"a\")\n            const copy = acts.ticks.slice()\n            \
             copy.push(true)\n            const o = {}\n            o.n = lines.length\n            \
             op.complete({ \"when\": () => ({}), \"report\": () => {\n                const out = [\"x\"]\n                \
             out.push(\"y=\" + acts.ticks.length)\n                return out\n            } })",
        ),
    );
    assert!(built.is_empty(), "{built:?}");
    let carried = found(
        &driver(""),
        &verbs(
            "",
            "            const got = []\n            op.input(\"chosen\", () => {\n                got.push(pane.chosen)\n            })",
        ),
    );
    assert_eq!(
        rules(&carried),
        ["stage"].into_iter().collect(),
        "{carried:?}"
    );
}

/// A value carried out of a press into the beat's own local is state kept
/// by hand, and so is a property of the operation given a reading; a
/// property given its value right before the operation that holds it
/// starts is that start's condition, and stands.
#[test]
fn state_kept_past_the_operations_api_is_named_and_a_start_condition_is_not() {
    let smuggled = found(
        &driver(""),
        &verbs(
            "",
            "            let got = 0\n            op.input(\"chosen\", () => {\n                got = pane.chosen\n            })",
        ),
    );
    assert_eq!(
        rules(&smuggled),
        ["stage"].into_iter().collect(),
        "{smuggled:?}"
    );
    let kept = found(
        &driver(""),
        &verbs("", "            op.pressed = pane.chosen > 0"),
    );
    assert_eq!(rules(&kept), ["stage"].into_iter().collect(), "{kept:?}");
    let configured = found(
        &driver(""),
        &verbs(
            "    PageStage {\n        id: other\n        name: \"other\"\n        property int row: 0\n    }",
            "            other.row = 2\n            other.start()",
        ),
    );
    assert!(configured.is_empty(), "{configured:?}");
}

/// What the check cannot follow is named, not passed: a call into the
/// product nothing answers for, a function called through a value, and a
/// completion with no `when` and `report` of its own.
#[test]
fn what_cannot_be_followed_is_named() {
    for beat in [
        "            pane.somethingNobodyDefines()",
        "            const f = pane.rowOf\n            f(1)",
        "            op.complete(() => \"done\")",
    ] {
        let read = found(&driver(""), &verbs("", beat));
        assert_eq!(
            rules(&read),
            ["unread"].into_iter().collect(),
            "{beat}: {read:?}"
        );
    }
}

/// Only the machinery declares what its functions may be called in.
#[test]
fn a_reach_declared_outside_the_machinery_is_named() {
    let declared = found(
        &driver(""),
        &verbs(
            "    // reach(read)\n    function peek() {\n        return pane.chosen\n    }",
            "",
        ),
    );
    assert_eq!(
        rules(&declared),
        ["reach"].into_iter().collect(),
        "{declared:?}"
    );
}

/// Each line of `body` set in by `by`.
fn indented(body: &str, by: &str) -> String {
    body.lines()
        .map(|line| format!("{by}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `body` run in a report that answers `answer`.
fn in_report(body: &str, answer: &str) -> String {
    format!(
        "            op.complete({{ \"when\": () => ({{}}), \"report\": () => {{\n{}\n                \
         return {answer}\n            }} }})",
        indented(body, "                ")
    )
}

/// The family's borrowed array, put inside a container the code made and
/// changed there: named the same whether the code puts it in itself, a
/// helper puts it into the code's container — under the name the code
/// holds it by or another — or a function of the language's own copies it
/// in, in a beat and in a report alike.
#[test]
fn a_borrowed_value_put_inside_the_codes_own_container_is_named_however_it_got_there() {
    let members = "    property var ticks: []\n    function keep(box, borrowed) {\n        box.push(borrowed)\n    }\n    \
                   function wrap() {\n        return [acts.ticks]\n    }";
    let forms = [
        "const box = []\nbox.push(acts.ticks)\nbox[0].push(true)",
        "const box = [acts.ticks]\nbox[0].push(true)",
        "const box = []\nbox[0] = acts.ticks\nbox[0].push(true)",
        "const box = {}\nbox.samples = acts.ticks\nbox.samples.push(true)",
        "const box = []\nacts.keep(box, acts.ticks)\nbox[0].push(true)",
        "const box = []\nconst same = box\nacts.keep(same, acts.ticks)\nbox[0].push(true)",
        "const o = Object.assign({}, { samples: acts.ticks })\no.samples.push(true)",
        "const c = [].concat([acts.ticks])\nc[0].push(true)",
        "const c = [...[acts.ticks]]\nc[0].push(true)",
        "acts.wrap()[0].push(true)",
        "Object.values({ s: acts.ticks })[0].push(true)",
    ];
    for form in forms {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert_eq!(
            rules(&beat),
            ["stage"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert_eq!(
            rules(&report),
            ["read"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
}

/// What nothing was put inside, and what a call no contract answers for
/// returns, are of no known origin: a change through them is not passed.
#[test]
fn a_change_through_a_container_of_unknown_contents_is_named() {
    for beat in [
        "const box = []\nbox[0].push(true)",
        "const got = [].nothingTheTableHolds()\ngot.push(true)",
        "const parsed = Object.nothingTheTableHolds({})\nparsed.push(true)",
    ] {
        let read = found(
            &driver(""),
            &verbs(
                "    property var ticks: []",
                &indented(beat, "            "),
            ),
        );
        assert_eq!(
            rules(&read),
            ["unread"].into_iter().collect(),
            "{beat}: {read:?}"
        );
    }
}

/// Containers the code made, holding only what it made, are its own to
/// build however they are filled — by itself, by a helper it hands them
/// to, or by a function of the language's own — in a beat and a report.
#[test]
fn containers_holding_only_what_the_code_made_are_its_own_to_build() {
    let members = "    property var ticks: []\n    function collect(list) {\n        list.push({ k: 1 })\n    }";
    let body = "const out = []\nout.push(acts.ticks.length)\nout.push({ n: 2 })\nout[1].n = 3\n\
                const rows = []\nacts.collect(rows)\nrows[0].k = 2\nconst o = Object.assign({}, { n: 1 })\no.n = 2\n\
                const copy = out.slice()\ncopy.push(4)\nconst parsed = JSON.parse(\"{}\")\nparsed.a = 1";
    let beat = found(
        &driver(""),
        &verbs(members, &indented(body, "            ")),
    );
    assert!(beat.is_empty(), "{beat:?}");
    let report = found(&driver(""), &verbs(members, &in_report(body, "out")));
    assert!(report.is_empty(), "{report:?}");
}
