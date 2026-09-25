import QtQuick

// Who the standing command log belongs to (デザイン規約 §git が言ったことを読む場所): the reader's, except a panel a
// failed fetch raised, which a recovering fetch takes back down. The rule lives here because it turns on the order
// failures and presses arrive in, which the page cannot be asked; `tests/qml/tst_commandsowner.qml` walks the orders.
QtObject {
    id: owner

    /// Whether the panel standing is the failed fetch's doing — and so the fetch's to take down again.
    property bool heldByFetch: false

    /// The first fetch of a run has failed and the page is about to raise the panel. `open`: one was already
    /// standing — the reader's if they had it up, still the fetch's if it already held it.
    function fetchRaises(open) {
        owner.heldByFetch = owner.heldByFetch || !open
    }

    /// The reader pressed the seat, `Clear`, or the closing mark, opening or closing: the panel is theirs.
    function readerTakes() {
        owner.heldByFetch = false
    }

    /// News that is not this fetch's was raised into the panel, so it stops going down by itself (news from before
    /// the fetch's never reaches here). Call only from answers on the tab's own feed: `CommandsModel.onFailure` runs
    /// on another feed in no fixed order, and a fetch's refusal read there as somebody else's never comes down.
    function newsTakes() {
        owner.heldByFetch = false
    }

    /// A fetch settled clean. `failures`: the run the tab still counts (recovery is the drain that zeroes it); `line`:
    /// whether the header carries one — the fetch's own is taken down by this same answer, so a standing line is
    /// somebody else's. Answers whether the panel goes down.
    function landingTakesItDown(failures, line) {
        if (!owner.heldByFetch || failures !== 0 || line)
            return false
        owner.heldByFetch = false
        return true
    }
}
