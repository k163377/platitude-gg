import QtQuick
import QtQuick.Templates as T

// The label for the repository's own words — branch, remote and tag names, paths, commit messages, git's diagnostics,
// configuration values, what the user typed — drawn as written. `Label` is for the app's words (`qsTr` / `Words`).
// Qt's `Text` guesses its format (`Text.AutoText` asks `Qt::mightBeRichText`, which `<b>` or a bare `&lt;` turns
// on), so a name would come back bold or with its entities resolved (`tests/qml/tst_untrustedtext.qml`). Every
// string that crosses the bridge counts as input; the one rule all of it gets is pinned here, and a rule for input
// alone — never for the app's words — lives here. Which label a binding takes is decided by what it binds:
// `cargo xtask structure` (`structure::labels`) reads the `text` of every bare `Label` / `Text` in the product and
// turns away anything but the app's words: string literals, numbers, `Number(…)`, `qsTr(literal)` with `.arg(…)`,
// `Words` and `Theme` and their functions, all of these called with the app's words alone, joined by `+` or chosen
// by a ternary (rules-refs/app-ui.md「入力は `PlainTextLabel`、固定値は素の `Label`」).
// `T.Label` with the body of Fusion's `Label.qml` (its two colours), not Fusion's `Label` itself: a QML-defined base
// adds a creator and a context per instance (`QQmlObjectCreator`), and the rows' labels are built per visible row.
// Markup is drawn only where its pieces are escaped first; those parts name their `textFormat` themselves, and are
// the only files `structure::labels` lets do so (its `NAMING`).
T.Label {
    id: control
    textFormat: Text.PlainText
    color: control.palette.windowText
    linkColor: control.palette.link
}
