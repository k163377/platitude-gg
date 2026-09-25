import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The `core.autocrlf` chooser of the settings screen's `REPOSITORY OVERRIDE` — the only place this app writes it
// (デザイン規約 §設定の画面): the chooser, the lines around it and git's words on a failed write. The values are git's
// spelling; the words are sentences about what happens (CLAUDE.md 絶対制約), with `LF` / `CRLF` as plain text
// (デザイン規約 §改行コードの警告).
ColumnLayout {
    id: field

    /// What that repository's own configuration sets, in git's own spelling; empty for one that sets nothing.
    property string held: ""
    /// What git would use there, same spelling; empty = nothing anywhere sets it.
    property string effective: ""
    /// A read has landed, so an empty chooser can be believed
    /// (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    property bool ready: false
    /// A write is out; a second pick would race it.
    property bool busy: false
    /// The line over the chooser: where the value lands, and what the empty row means there.
    property string note: ""
    /// git's own words, unedited.
    property string errorText: ""

    /// A row was picked. The value is git's spelling, empty for the row that takes the key out.
    signal picked(string value)

    /// Picking the row at `index`; the combo's handler is one line onto it, so a run takes the same road as a hand.
    function pick(index) {
        field.picked(field.values[index])
    }
    /// The row `value` stands at, or -1. What a run picks by.
    function rowOf(value) {
        return field.values.indexOf(value)
    }

    /// Values and their words at matching indexes. The first row writes nothing, so it is named for what it gets you.
    readonly property var values: ["", "true", "input", "false"]
    readonly property var words: [
        qsTr("Inherited"),
        qsTr("Store LF, check out CRLF"),
        qsTr("Store LF, check out LF"),
        qsTr("Store the file's own endings")
    ]
    /// The row `held` names; a value git refuses reads as unset (`eol::setting::AutoCrlf::of_record`), so both land on
    /// the first row.
    readonly property int heldRow: Math.max(0, field.values.indexOf(field.held))
    /// A whole sentence per answer — pieces cannot be translated (デザイン規約 §改行コードの警告).
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

    // What the setting is goes over the chooser, what it amounts to now goes under (デザイン規約 §設定の画面).
    HelpText {
        visible: text !== ""
        text: field.note
    }
    // No caption: the chapter heading names it (デザイン規約 §設定の画面「章の見出しが欄の名前を兼ねる」).
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spaceSm
        // Picking only: git takes these values and nothing else (デザイン規約 §選ぶ欄と打つ欄). Full width, since the
        // rows are sentences (§レイアウト初期値).
        AppCombo {
            id: chooser
            Layout.fillWidth: true
            pickOnly: true
            // Not before git has answered: a pick would replace a value nobody has seen.
            enabled: field.ready && !field.busy
            model: field.words
            // What git holds; a pick is read back from the file, so the row showing is always one git named.
            wanted: field.ready ? field.words[field.heldRow] : ""
            onActivated: index => field.pick(index)
        }
    }
    // What the setting amounts to: the fallback lives in files this screen does not show.
    HelpText {
        visible: field.ready
        text: field.effectiveLine
    }
    // A field like every line on this screen (デザイン規約 §右のペインの字は掴める).
    CardText {
        Layout.fillWidth: true
        visible: text !== ""
        color: Theme.danger
        text: field.errorText
    }
}
