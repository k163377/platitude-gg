import QtQuick
import platitude.ui

// One line of what a screen with nothing to show says instead: the window waiting on git, a tab whose repository would
// not open, the graph with no history behind it.
//
// **A field** (`CardText`, 規約 §右のペインの字は掴める). What these lines carry is git's own words as
// often as the app's — the git that would not answer, the repository that would not open — and these are the screens
// where there is nothing else to read them from: the gate stands before any repository is open, so the command log
// that holds git's words everywhere else does not exist yet. A sentence nobody can drag over is one that has to be
// copied out by hand.
//
// Wrapped: such a line is the whole of what is on that screen, so there is nothing beside it to carry
// what a cut would drop. Centred over its own width for the same reason — it is the middle of an empty
// screen.
//
// The width is the instance's: the container is not always the bound (the graph's own pane holds the history's width,
// not the sentence's), and a default read off the parent would be a binding loop inside a column sized to its content.
// `CardText` measures its natural width off a ruler of its own for exactly that reason, so a caller may bound itself
// by `implicitWidth` without standing on its own answer.
CardText {
    horizontalAlignment: Text.AlignHCenter
}
