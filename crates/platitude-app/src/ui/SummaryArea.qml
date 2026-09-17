import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The summary box at the top of a commit message, shared by the commit
// editor and the commit details pane so the two cannot drift apart.
//
// It holds exactly one line. A commit's summary is its first line
// (platitude-core's split_message), so a newline in here would not
// survive the round trip: everything after it would come back in the
// description box. Wrapped -- which is why this is
// a TextArea -- so a long summary stays
// readable and the frame around it grows to fit.
TextArea {
    id: summary
    wrapMode: TextArea.Wrap
    font.pixelSize: Theme.fontLg
    font.weight: Font.DemiBold
    color: Theme.textPrimary
    background: null
    padding: 0

    // Return is the one keystroke that inserts a newline.
    Keys.onReturnPressed: (event) => { event.accepted = true }
    Keys.onEnterPressed: (event) => { event.accepted = true }
    // A paste, a drop or a prefill can carry any number of them, so the
    // text itself is watched too. They fold to a space
    // -- the way git's own %s folds a multi-line summary -- so
    // the words around one keep their gap. The assignment retriggers
    // this handler; the first test ends that.
    onTextChanged: {
        if (!/[\n\r]/.test(summary.text))
            return
        const kept = summary.text.substring(0, summary.cursorPosition)
        summary.text = summary.text.replace(/[\n\r]+/g, " ")
        summary.cursorPosition = kept.replace(/[\n\r]+/g, " ").length
    }
}
