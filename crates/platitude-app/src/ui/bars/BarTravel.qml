// How a bar over the middle of the page takes the place of the one before it (`RepoPage.centreColumn`; user decision —
// ask before changing): the one going goes in one frame, and the one coming stands at its height in that same frame and
// travels to its own, so the rows under them move by the difference only. The report, the question and the discard
// band all come in this way (`NoticeBar`, `AskBar`, `RecoverBand`); each keeps its own `travelling`, which arms the
// `Behavior` on its height. The page notes the graph's place before each such swap (`GraphPane.keepRowsInPlace`): the
// question and the page's bars reach the list in different passes.
pragma Singleton

import QtQuick

QtObject {
    /// Stands `bar` at `from` in this frame and travels it to `own()` from there: the height set with the animation
    /// off, then bound back with it on. At its own height already, it does not travel: a height written unchanged
    /// moves nothing, so no arrival would put `travelling` down.
    function standThenTravel(bar, from, own) {
        bar.travelling = false
        bar.implicitHeight = from
        bar.travelling = from !== own()
        bar.implicitHeight = Qt.binding(own)
    }
    /// Takes `bar` off the screen in this frame from wherever its travel stood: with the animation off, the height
    /// written stops the one under way (`QQuickBehavior::write`), then is bound back.
    function cut(bar, own) {
        bar.travelling = false
        bar.implicitHeight = 0
        bar.implicitHeight = Qt.binding(own)
    }
}
