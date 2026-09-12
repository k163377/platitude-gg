import QtQuick

// ---- who the standing command log belongs to -------------------------
// A panel that is up is the reader's: a failure raises it and it stays up until they take it down, because the next
// success is not permission to take the message away (デザイン規約 §git が言ったことを読む場所). The one exception is the
// panel a failed fetch raised, which a fetch that reaches the remote again takes back down — a machine that sleeps
// loses the remote as a matter of course, and the news of that outliving the network is the whole of what the reader
// is left reading.
//
// **The rule lives here because it is a question of order, and order is the one thing the page cannot be asked**: the
// paths run through failures that came before the fetch's, failures that come after it, and the reader's own press in
// between, and each of those decides who the panel belongs to. `tests/qml/tst_commandsowner.qml` walks them.
//
// Nothing is drawn here, so this is a `QtObject` (rules-refs/structure.md §描かないホスト).
QtObject {
    id: owner

    /// Whether the panel standing is the failed fetch's doing — and so the fetch's to take down again.
    property bool heldByFetch: false

    /// The first fetch of a run has failed and the page is about to raise the panel. `open` is whether one was
    /// already standing: a panel the reader had up is theirs, while one already held stays the fetch's (a second run
    /// of failures reaches this before the first panel has been taken down).
    function fetchRaises(open) {
        owner.heldByFetch = owner.heldByFetch || !open
    }

    /// The reader has pressed the seat, `Clear`, or the closing mark. The panel is theirs from here in both
    /// directions — the press that opens it again is not a fetch's doing either.
    function readerTakes() {
        owner.heldByFetch = false
    }

    /// News that is not this fetch's has been raised into the panel: another command's failure, or the answer to a
    /// write that nothing else on screen explains. The panel is saying two things now and only one of them goes down
    /// by itself, so it stops going down at all. **News that came before the fetch's does not reach here** — it was
    /// raised into a panel the reader has already taken down, and the mark in the corner is what still carries it.
    function newsTakes() {
        owner.heldByFetch = false
    }

    /// A fetch has settled clean. `failures` is the run of failures the tab is still counting (the recovery is the
    /// drain that puts it back to zero) and `line` whether the header is carrying one: the fetch's own line is taken
    /// down by this same answer, so a line still standing is somebody else's and the panel stays up under it.
    /// Answers whether the panel goes down, and hands it back when it does.
    function landingTakesItDown(failures, line) {
        if (!owner.heldByFetch || failures !== 0 || line)
            return false
        owner.heldByFetch = false
        return true
    }
}
