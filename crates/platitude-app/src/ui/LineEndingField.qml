import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// What git does to a file's line endings when it stores it and when it checks it out, and everything that answers for
// a write of it: the chooser, the line saying where it lands, the line saying what git is doing right now, and git's
// own words when the write did not take.
//
// Two chapters ask the same question — the settings screen's `GLOBAL` group and its `REPOSITORY OVERRIDE` — so the
// field is one file and the callers differ only in which of git's files is behind it and what the empty row is called
// there (`LineEndingsModel.scope`). The shape `IdentityFields` keeps, for the same reason.
//
// **The words are here and the values are git's.** `core.autocrlf` takes three answers and the model hands them over
// spelled the way git spells them; what a reader is shown is a sentence about what happens, because a setting's UI
// wording is chosen by what the operation does rather than by the command's own name (CLAUDE.md 絶対制約). The two
// lists below are that translation and nothing more — the row a reader picks is the value at the same index.
//
// **`LF` and `CRLF` are plain text, not code chips** (規約 §改行コードの警告): a chip is lowercase monospace and an
// all-caps abbreviation does not sit in one.
ColumnLayout {
    id: field

    /// What that level of git's configuration sets, in git's own spelling; empty for a level that sets nothing.
    property string held: ""
    /// What git would use in the repository this chapter is about, in the same spelling. Empty is an answer of its
    /// own here — nothing anywhere sets it — so whether the line is said at all is `saysEffective`.
    property string effective: ""
    /// Say what git is doing in that repository right now, under the chooser. The repository chapter does: what its
    /// empty row falls back to lives in a file this screen is not showing. The global chapter does not — it is the
    /// highest level this app writes, so what it says there is what git does, and a line repeating the chooser would
    /// be one more thing to read for nothing.
    property bool saysEffective: false
    /// A read has landed, so an empty chooser can be believed. Until then it looks exactly like a field nobody has
    /// filled in (規約 app-ui.md §「まだ答えが無い」と値 0 / false を分ける).
    property bool ready: false
    /// A write is out. The chooser stays standing — it says what git holds, and a pick that did not take is what the
    /// line at the foot is for — but a second pick would be racing the first.
    property bool busy: false
    /// What the row for "this level writes nothing" is called. The global level has nothing above it to fall back to
    /// and says so; a repository's override falls back to that value and says *that*.
    property string unwrittenWord: qsTr("Not set")
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

    /// The values, and the words for them, at matching indexes. The first row is the one that writes nothing, which
    /// is why it is the one whose word the caller replaces.
    readonly property var values: ["", "true", "input", "false"]
    readonly property var words: [
        field.unwrittenWord,
        qsTr("Store LF, check out CRLF"),
        qsTr("Store LF, check out LF"),
        qsTr("Store the file's own endings")
    ]
    /// The row `held` names. Nothing git wrote that this app can spell falls outside the list, and a level that sets
    /// nothing is the first row — so an unrecognised answer and an unset one land in the same place, which is where
    /// picking anything at all is the way out (`eol::setting::AutoCrlf::of_record`).
    readonly property int heldRow: Math.max(0, field.values.indexOf(field.held))
    /// The whole sentence for what git is doing there now, one per answer rather than a phrase dropped into a frame:
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
    // No caption over the chooser: the chapter's own heading already names it, and one control under a heading that
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
            // What git holds, until a pick reports one. Not a value the field keeps: every answer here comes back
            // from the file it was written into, so the row showing is always one git named.
            wanted: field.ready ? field.words[field.heldRow] : ""
            onActivated: index => field.pick(index)
        }
    }
    // What the file the chooser is showing actually amounts to, which the chooser cannot say on its own: the value it
    // falls back to lives in a file this screen is not showing, and git resolves it through more than one of them.
    // **This one stays under** — it is not what the setting is, it is where the setting has got to.
    HelpText {
        visible: field.saysEffective && field.ready
        text: field.effectiveLine
    }
    Label {
        Layout.fillWidth: true
        visible: text !== ""
        wrapMode: Text.Wrap
        color: Theme.danger
        text: field.errorText
    }
}
