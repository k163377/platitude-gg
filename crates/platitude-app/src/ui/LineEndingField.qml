import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// What git does to a file's line endings when it stores it and when it checks it out, and everything that answers for
// a write of it: the chooser, the line saying where it lands, the line saying what git is doing right now, and git's
// own words when the write did not take.
//
// **One chapter asks it: the settings screen's `REPOSITORY OVERRIDE`** (規約 §設定の画面). This app writes
// `core.autocrlf` into the repository somebody picked and nowhere else — the machine's own configuration is not an
// application's to rewrite — so everything here is about one repository's file. A file of its own all the same,
// because a chooser, the two lines around it and git's words are a chapter's worth of shape (`IdentityFields`).
//
// **The words are here and the values are git's.** `core.autocrlf` takes three answers and the model hands them over
// spelled the way git spells them; what a reader is shown is a sentence about what happens, because a setting's UI
// wording is chosen by what the operation does (CLAUDE.md 絶対制約). The two
// lists below are that translation and nothing more — the row a reader picks is the value at the same index.
//
// **`LF` and `CRLF` are plain text** (規約 §改行コードの警告): a chip is lowercase monospace and an
// all-caps abbreviation does not sit in one.
ColumnLayout {
    id: field

    /// What that repository's own configuration sets, in git's own spelling; empty for one that sets nothing.
    property string held: ""
    /// What git would use in the repository this chapter is about, in the same spelling. Empty is an answer of its
    /// own here — nothing anywhere sets it — which is why the line below says it in words.
    property string effective: ""
    /// A read has landed, so an empty chooser can be believed. Until then it looks exactly like a field nobody has
    /// filled in (規約 app-ui.md §「まだ答えが無い」と値 0 / false を分ける).
    property bool ready: false
    /// A write is out. The chooser stays standing — it says what git holds, and a pick that did not take is what the
    /// line at the foot is for — but a second pick would be racing the first.
    property bool busy: false
    /// The line under the chooser: where the value lands, and what the empty row means there.
    property string note: ""
    /// git's own words, unedited.
    property string errorText: ""

    /// A row was picked. The value is git's spelling, empty for the row that takes the key out.
    signal picked(string value)

    /// What picking the row at `index` does. The handler below is one line onto it so a run enters the same road a
    /// hand does (規約 §UI 自動化の因果性).
    function pick(index) {
        field.picked(field.values[index])
    }
    /// The row `value` stands at, or -1 for a value this list does not hold. What a run picks by.
    function rowOf(value) {
        return field.values.indexOf(value)
    }

    /// The values, and the words for them, at matching indexes. The first row is the one that writes nothing — what
    /// it gets you is the value above, so it is named for that
    /// (the `inherited` the identity boxes use as a placeholder).
    readonly property var values: ["", "true", "input", "false"]
    readonly property var words: [
        qsTr("Inherited"),
        qsTr("Store LF, check out CRLF"),
        qsTr("Store LF, check out LF"),
        qsTr("Store the file's own endings")
    ]
    /// The row `held` names. Nothing git wrote that this app can spell falls outside the list, and a repository
    /// that sets nothing of its own is the first row — so an unrecognised answer and an unset one land in the same
    /// place, which is where picking anything at all is the way out (`eol::setting::AutoCrlf::of_record`).
    readonly property int heldRow: Math.max(0, field.values.indexOf(field.held))
    /// The whole sentence for what git is doing there now, one per answer:
    /// a sentence assembled from pieces cannot be translated (規約 §改行コードの警告).
    readonly property string effectiveLine: {
        switch (field.effective) {
        case "true":
            return qsTr("git stores LF there and checks out CRLF right now.")
        case "input":
            return qsTr("git stores LF there right now, and checks out what it stored.")
        case "false":
            return qsTr("git leaves line endings alone there right now.")
        default:
            return qsTr("Nothing sets it for that repository, so git leaves line endings alone there.")
        }
    }

    Layout.fillWidth: true
    spacing: Theme.spaceLg

    // What this chapter writes, said before the chooser: a sentence about the setting goes over it, and only what
    // the setting currently amounts to goes under (規約 §設定の画面).
    HelpText {
        visible: text !== ""
        text: field.note
    }
    // The heading is the caption: the chapter's own heading already names it, and one control under a heading that
    // says the same word twice reads as two things (`MERGE EDITOR` is the same shape). A caption is for telling the
    // boxes of a chapter apart, which is why the identity keeps two.
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceSm
        // Picking only: the list is the whole set of answers, because git takes three and nothing else
        // (デザイン規約 §選ぶ欄と打つ欄). As wide as the chapter gives it — the rows are sentences, and an input
        // takes a fixed width only when its content does (§レイアウト初期値).
        AppCombo {
            id: chooser
            Layout.fillWidth: true
            pickOnly: true
            // Nothing to pick against until git has answered: an unanswered chooser stands empty, and a pick made
            // from there would be writing a value chosen without knowing what it replaces.
            enabled: field.ready && !field.busy
            model: field.words
            // What git holds, until a pick reports one. Every answer here comes back
            // from the file it was written into, so the row showing is always one git named.
            wanted: field.ready ? field.words[field.heldRow] : ""
            onActivated: index => field.pick(index)
        }
    }
    // What the file the chooser is showing actually amounts to, which the chooser cannot say on its own: the value it
    // falls back to lives in a file this screen is not showing, and git resolves it through more than one of them.
    // **This one stays under** — it is where the setting has got to.
    HelpText {
        visible: field.ready
        text: field.effectiveLine
    }
    // git's words, and a field like every other line on this screen (規約 §右のペインの字は掴める).
    CardText {
        Layout.fillWidth: true
        visible: text !== ""
        color: Theme.danger
        text: field.errorText
    }
}
