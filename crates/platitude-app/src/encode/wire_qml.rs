//! The wire shapes across the real QML boundary (`wire`): a QObject's
//! properties and slots, and a list model's roles, read and written by QML
//! in a Qt application of their own. `wire`'s own tests run the two
//! conversions back to back in Rust; these are the calls QML makes.
//!
//! A Qt application cannot share a process with the other tests' threads,
//! nor pick its platform once one is up, so the QML runs offscreen in a
//! copy of this binary started for it ([`qml_side`]).
//!
//! A write from QML runs in a copy of its own: one the wire does not read,
//! made to a `Listed` / `One` property with a `Member`, takes the process
//! down (the bridge's generated write panics inside an `extern "C"` call).
//! What the product holds to and what the bridge happens to do are two
//! tests apart.

#![expect(
    clippy::print_stdout,
    reason = "the QML side says each check on stdout, where the parent reads it"
)]

use std::collections::BTreeMap;
use std::process::{Child, Command, Stdio};

use qtbridge::qtbridge_type_lib::{QGuiApplication, QQmlApplicationEngine, QVariantMap};
use qtbridge::{QListModel, QModelItem, QmlElement, qobject};

use super::wire::{Fields, Listed, One, Optional, Record, field};

/// Three fields of three kinds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sample {
    word: String,
    count: i32,
    on: bool,
}

impl Sample {
    fn new(word: &str, count: i32, on: bool) -> Self {
        Self {
            word: word.into(),
            count,
            on,
        }
    }
}

impl Record for Sample {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("word", &self.word)
            .put("count", &self.count)
            .put("on", &self.on)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            word: field(map, "word")?,
            count: field(map, "count")?,
            on: field(map, "on")?,
        })
    }
}

/// Records inside a record.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Nest {
    name: String,
    inner: Listed<Sample>,
    one: One<Sample>,
}

impl Record for Nest {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("inner", &self.inner)
            .put("one", &self.one)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            inner: field(map, "inner")?,
            one: field(map, "one")?,
        })
    }
}

fn two() -> Listed<Sample> {
    Listed::new(vec![
        Sample::new("main", 3, true),
        Sample::new("日本語", -1, false),
    ])
}

pub struct WireProbe {
    listed: Listed<Sample>,
    one: One<Sample>,
    count: i32,
    word: String,
    words: Vec<String>,
}

impl Default for WireProbe {
    fn default() -> Self {
        Self {
            listed: two(),
            one: One::new(Sample::new("main", 3, true)),
            count: 7,
            word: "main".into(),
            words: vec!["main".into()],
        }
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl WireProbe {
    qproperty!("listed", Member = listed, Notify = changed);
    qproperty!("one", Member = one, Notify = changed);
    // Qt's own types, which the engine converts to before the write reaches Rust.
    qproperty!("count", Member = count, Notify = changed);
    qproperty!("word", Member = word, Notify = changed);
    qproperty!("words", Member = words, Notify = changed);
    // `listed` behind a getter alone, with and without a notification.
    qproperty!("listedRead", Read = listed_read, Notify = changed);
    qproperty!("listedConstant", Read = listed_read, Constant);

    fn listed_read(&self) -> &Listed<Sample> {
        &self.listed
    }

    /// Everything a write could have changed, as one line.
    #[qslot]
    fn state(&self) -> String {
        format!(
            "count={} word={} words={:?} listed={:?} one={}",
            self.count,
            self.word,
            self.words,
            self.held_words(),
            self.one.word
        )
    }

    #[qsignal]
    fn changed(&mut self);

    #[qslot]
    fn echo_list(&self, value: Listed<Sample>) -> Listed<Sample> {
        value
    }

    #[qslot]
    fn echo_one(&self, value: One<Sample>) -> One<Sample> {
        value
    }

    #[qslot]
    fn echo_optional(&self, value: Optional<Sample>) -> Optional<Sample> {
        value
    }

    #[qslot]
    fn echo_nest(&self, value: One<Nest>) -> One<Nest> {
        value
    }

    /// What this side holds after QML wrote `listed`.
    #[qslot]
    fn held_words(&self) -> Vec<String> {
        self.listed.iter().map(|s| s.word.clone()).collect()
    }

    #[qslot]
    fn check(&self, name: String, ok: bool, detail: String) {
        println!("WIRE\t{name}\t{ok}\t{detail}");
    }
}

impl QmlElement for WireProbe {
    const URI: &str = "wireprobe";
    const ELEMENT_NAME: &str = "WireProbe";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = false;
}

/// A row whose roles are the three shapes, and text.
#[derive(QModelItem, Default, Clone)]
pub struct WireRow {
    text: String,
    one: One<Sample>,
    listed: Listed<Sample>,
    maybe: Optional<Sample>,
}

pub struct WireRows {
    rows: Vec<WireRow>,
}

impl Default for WireRows {
    fn default() -> Self {
        Self {
            rows: vec![
                WireRow {
                    text: "修正 #1".into(),
                    one: One::new(Sample::new("main", 3, true)),
                    listed: two(),
                    maybe: Optional::some(Sample::new("main", 3, true)),
                },
                WireRow::default(),
            ],
        }
    }
}

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl WireRows {}

impl QListModel for WireRows {
    type Item = WireRow;

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, index: usize) -> Option<&WireRow> {
        self.rows.get(index)
    }
}

impl QmlElement for WireRows {
    const URI: &str = "wireprobe";
    const ELEMENT_NAME: &str = "WireRows";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = false;
}

/// The QML side: reads and writes every shape, and says how each went.
const CHECKS: &str = r#"
import QtQml
import QtQml.Models
import wireprobe

QtObject {
    id: root

    property WireProbe probe: WireProbe {}
    property WireRows rows: WireRows {}
    property int rolesSeen: 0
    property Instantiator roles: Instantiator {
        model: root.rows
        delegate: QtObject {
            required property int index
            required property string text
            required property var one
            required property var listed
            required property var maybe
            Component.onCompleted: root.role(index, text, one, listed, maybe)
        }
    }
    property Timer ceiling: Timer {
        interval: 10000
        running: true
        onTriggered: Qt.quit()
    }

    function say(name, ok, detail) {
        probe.check(name, !!ok, detail === undefined ? "undefined" : JSON.stringify(detail))
    }

    function sample(word, count, on) {
        return { word: word, count: count, on: on }
    }

    function role(index, text, one, listed, maybe) {
        rolesSeen++
        if (index === 0) {
            say("role-text", text === "修正 #1", text)
            say("role-one", one.word === "main" && one.count === 3 && one.on === true, one)
            say("role-listed", listed.length === 2 && listed[1].word === "日本語"
                && listed[1].count === -1 && listed[1].on === false, listed)
            say("role-optional-some", maybe !== undefined && maybe.word === "main", maybe)
        } else {
            say("role-empty-text", text === "", text)
            say("role-empty-listed", listed.length === 0, listed)
            say("role-optional-none", maybe === undefined, maybe)
        }
    }

    Component.onCompleted: {
        const held = probe.listed
        say("property-listed-read", held.length === 2 && held[0].word === "main"
            && held[0].count === 3 && held[0].on === true && held[1].word === "日本語", held)
        say("property-one-read", probe.one.word === "main" && probe.one.count === 3, probe.one)

        probe.listed = [sample("x", 1, false)]
        say("property-listed-written", probe.listed.length === 1 && probe.listed[0].word === "x"
            && probe.heldWords().join() === "x", probe.heldWords())
        probe.listed = []
        say("property-listed-emptied", probe.listed.length === 0 && probe.heldWords().length === 0,
            probe.heldWords())

        const pair = [sample("a", 1, true), sample("日本語", 2, false)]
        const back = probe.echoList(pair)
        say("slot-list", back.length === 2 && back[1].word === "日本語" && back[1].count === 2
            && back[1].on === false, back)
        say("slot-list-empty", probe.echoList([]).length === 0, probe.echoList([]))
        const kept = probe.echoList([sample("ok", 1, true), { word: "short" }])
        say("slot-list-missing-field-left-out", kept.length === 1 && kept[0].word === "ok", kept)
        const none = probe.echoList("not a list")
        say("slot-list-not-a-list-empty", none.length === 0, none)

        const one = probe.echoOne(sample("w", 2, true))
        say("slot-one", one.word === "w" && one.count === 2 && one.on === true, one)
        const blank = probe.echoOne({ word: "short" })
        say("slot-one-missing-field-default", blank.word === "" && blank.count === 0 && blank.on === false,
            blank)
        let refused
        try { probe.echoOne(5) } catch (e) { refused = String(e) }
        say("slot-one-not-an-object-refused-by-qml", refused !== undefined && refused.startsWith("TypeError"),
            refused)

        const some = probe.echoOptional(sample("s", 5, false))
        say("slot-optional-some", some !== undefined && some.word === "s" && some.count === 5, some)
        say("slot-optional-undefined", probe.echoOptional(undefined) === undefined,
            probe.echoOptional(undefined))
        say("slot-optional-null-none", probe.echoOptional(null) === undefined, probe.echoOptional(null))
        say("slot-optional-missing-field-none", probe.echoOptional({ word: "short" }) === undefined,
            probe.echoOptional({ word: "short" }))

        const nest = probe.echoNest({ name: "outer", inner: pair, one: sample("in", 7, true) })
        say("slot-nested", nest.name === "outer" && nest.inner.length === 2
            && nest.inner[1].word === "日本語" && nest.one.count === 7, nest)
        const broken = probe.echoNest({ name: "outer", inner: [{ word: "short" }], one: sample("in", 7, true) })
        say("slot-nested-missing-field-default", broken.name === "" && broken.inner.length === 0
            && broken.one.word === "", broken)

        Qt.callLater(root.finish)
    }

    function finish() {
        say("roles-seen", rolesSeen === 2, rolesSeen)
        Qt.quit()
    }
}
"#;

/// How many lines [`CHECKS`] says.
const CHECKED: usize = 25;

/// One write of `VALUE` to `PROPERTY`, said before and after: what the
/// write threw, if anything, and what the object holds once it is done.
const WRITE: &str = r#"
import QtQml
import wireprobe

QtObject {
    property WireProbe probe: WireProbe {}

    Component.onCompleted: {
        probe.check("write-before", true, probe.state())
        let threw = ""
        try { probe.PROPERTY = VALUE } catch (e) { threw = String(e) }
        probe.check("write-threw", threw !== "", threw)
        probe.check("write-after", true, probe.state())
        Qt.quit()
    }
}
"#;

/// Where a write is checked, which decides what may answer it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Checked {
    /// Qt's own type: the engine converts to it before the write reaches
    /// Rust, so a value it cannot convert never does.
    ByQt,
    /// `Listed` / `One`: Qt sees a `QVariantList` / `QVariantMap`, and the
    /// elements and fields are the wire's to read.
    ByTheWire,
    /// A property with a getter and no write: nothing on the Rust side to
    /// take the value.
    NoWrite,
    /// `Constant`: read-only to the meta-object itself.
    Constant,
}

/// One write the diagnosis makes: name, property, value, where it is checked.
const WRITES: [(&str, &str, &str, Checked); 11] = [
    ("count-text", "count", r#""abc""#, Checked::ByQt),
    ("count-object", "count", "({ a: 1 })", Checked::ByQt),
    ("word-object", "word", "({ a: 1 })", Checked::ByQt),
    ("words-object", "words", "({ a: 1 })", Checked::ByQt),
    ("words-numbers", "words", "[1, 2]", Checked::ByQt),
    (
        "listed-missing-field",
        "listed",
        r#"[{ word: "short" }]"#,
        Checked::ByTheWire,
    ),
    (
        "listed-not-a-list",
        "listed",
        r#""not a list""#,
        Checked::ByTheWire,
    ),
    (
        "one-missing-field",
        "one",
        r#"({ word: "short" })"#,
        Checked::ByTheWire,
    ),
    (
        "getter-only-well-formed",
        "listedRead",
        "[]",
        Checked::NoWrite,
    ),
    (
        "getter-only-missing-field",
        "listedRead",
        r#"[{ word: "short" }]"#,
        Checked::NoWrite,
    ),
    (
        "constant-well-formed",
        "listedConstant",
        "[]",
        Checked::Constant,
    ),
];

/// The QML a child runs: [`CHECKS`], or one of the [`WRITES`].
fn child_qml(child: &str) -> String {
    WRITES.iter().find(|(name, ..)| *name == child).map_or_else(
        || CHECKS.to_owned(),
        |(_, property, value, _)| WRITE.replace("PROPERTY", property).replace("VALUE", value),
    )
}

/// The child: a Qt application whose QML is [`child_qml`]. Nothing without
/// the parent's mark, so a run of every ignored test does not start one in
/// a process the other tests share.
#[test]
#[ignore = "started by the tests below, offscreen, as a process of its own"]
fn qml_side() {
    let Some(child) = std::env::var_os("WIRE_QML_CHILD") else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("pgg-wire-qml-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory for the QML");
    let file = dir.join("WireChecks.qml");
    std::fs::write(&file, child_qml(&child.to_string_lossy())).expect("the QML");
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    crate::qml_engine::arm(engine.pin_mut());
    WireProbe::register();
    WireRows::register();
    crate::qml_engine::load(engine.pin_mut(), &crate::urlpath::file_url(&file))
        .expect("the checks load");
    app.pin_mut().exec();
    drop(engine);
    qtbridge::collect_garbage();
    drop(app);
    let _ = std::fs::remove_dir_all(&dir);
}

fn start(child: &str) -> Child {
    Command::new(std::env::current_exe().expect("this test binary"))
        .args([
            "encode::wire_qml::qml_side",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("WIRE_QML_CHILD", child)
        .env("QT_QPA_PLATFORM", "offscreen")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the child starts")
}

/// What a child said, by check: (`ok`, detail).
fn said(stdout: &str) -> BTreeMap<&str, (&str, &str)> {
    stdout
        .lines()
        .filter_map(|line| line.split_once("WIRE\t"))
        .filter_map(|(_, rest)| {
            let mut parts = rest.splitn(3, '\t');
            Some((parts.next()?, (parts.next()?, parts.next().unwrap_or(""))))
        })
        .collect()
}

/// What the product holds to: reads, slot calls and roles across the
/// boundary, with what QML hands a slot that does not read.
#[test]
fn the_wire_shapes_cross_the_qml_boundary() {
    let out = start("checks")
        .wait_with_output()
        .expect("the checks child ends");
    let text = String::from_utf8_lossy(&out.stdout);
    let checks = said(&text);
    for (name, (ok, detail)) in &checks {
        println!("{name}: {ok} {detail}");
    }
    assert!(
        out.status.success(),
        "the child failed:\n{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let failed: Vec<&&str> = checks
        .iter()
        .filter(|(_, (ok, _))| *ok != "true")
        .map(|(name, _)| name)
        .collect();
    assert!(failed.is_empty(), "failed: {failed:?}\n{text}");
    assert_eq!(checks.len(), CHECKED, "every check said its line:\n{text}");
}

/// How one write came out.
#[derive(Debug, PartialEq)]
enum Outcome {
    /// The process went down; the panic's message.
    Aborted(String),
    /// QML threw (what it threw) and the object holds what it held.
    Refused(String),
    /// Nothing thrown and nothing changed.
    Ignored,
    /// The write went in: what the object holds after it.
    Taken(String),
}

fn outcome(out: &std::process::Output) -> Outcome {
    let text = String::from_utf8_lossy(&out.stdout);
    let lines = said(&text);
    let detail = |name: &str| lines.get(name).map(|(_, detail)| *detail);
    let before = detail("write-before").unwrap_or_default();
    match (detail("write-threw"), detail("write-after")) {
        (Some(threw), Some(after)) if after == before && !threw.is_empty() => {
            Outcome::Refused(threw.to_string())
        }
        (Some(_), Some(after)) if after == before => Outcome::Ignored,
        (Some(_), Some(after)) => Outcome::Taken(after.to_string()),
        _ => {
            let errors = String::from_utf8_lossy(&out.stderr);
            let panic = errors
                .lines()
                .skip_while(|line| !line.contains("panicked at"))
                .nth(1)
                .unwrap_or("")
                .to_string();
            Outcome::Aborted(panic)
        }
    }
}

/// Runs every write of [`WRITES`] checked as `which`, each in a child of its
/// own (a write may take the process down), and answers how each came out.
fn writes(which: impl Fn(Checked) -> bool) -> Vec<(&'static str, Checked, Outcome)> {
    let children: Vec<_> = WRITES
        .iter()
        .filter(|(.., checked)| which(*checked))
        .map(|(name, _, _, checked)| (*name, *checked, start(name)))
        .collect();
    children
        .into_iter()
        .map(|(name, checked, child)| {
            let out = child.wait_with_output().expect("the write child ends");
            let came = outcome(&out);
            println!("{name} ({checked:?}): {came:?}");
            (name, checked, came)
        })
        .collect()
}

/// What the product holds to: a property the product exposes through a
/// getter alone (`Read`, no `Member`) cannot be changed from QML, nor taken
/// down by a value that does not read.
#[test]
fn a_property_exposed_through_its_getter_alone_cannot_be_written() {
    for (name, _, came) in writes(|checked| checked == Checked::NoWrite) {
        assert!(
            matches!(came, Outcome::Ignored | Outcome::Refused(_)),
            "{name}: {came:?}"
        );
    }
}

/// Not the product's contract: what the bridge does with each kind of
/// write QML can make to a writable property. Fails only on an answer
/// outside what is known for its kind — a bridge that starts refusing what
/// it used to take down is not a failure.
#[test]
fn what_the_bridge_does_with_a_write_from_qml() {
    for (name, checked, came) in writes(|checked| checked != Checked::NoWrite) {
        let known = match (checked, &came) {
            // Qt converts or refuses; it never hands Rust a value of
            // another type.
            (Checked::ByQt, Outcome::Aborted(_)) => false,
            (Checked::ByQt, _) => true,
            // The generated write panics in `extern "C"` on a value the
            // wire does not read (qtbridge-gen `qproperty_info.rs`).
            (Checked::ByTheWire, Outcome::Aborted(panic)) => {
                panic.starts_with("Failed to convert QVariant for qproperty")
            }
            (Checked::ByTheWire, Outcome::Refused(_) | Outcome::Ignored) => true,
            (Checked::Constant, Outcome::Refused(_)) => true,
            _ => false,
        };
        assert!(known, "{name} ({checked:?}): {came:?}");
    }
}
