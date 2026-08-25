import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The hidden measurements behind the band's shared button box: the four candidate wordings, each drawn (invisibly)
// exactly as a button would draw it. The band ranks them once and hands the winner to all three buttons
// (`TopBar.widestAction`).
//
// `push` is inside `push -f`, fetch says the command in every shape but the stopped one, and stash has the one
// wording, so four cover all seven states. `stash` never wins — five monospaced cells is what `fetch` already is —
// but it is measured rather than reasoned about, so re-wording any of them cannot leave the box short. Labels
// rather than TextMetrics: TextMetrics reports a few pixels tighter than a Label — the set could be ranked on one
// measure and sized by another, and the loser could then be the wider of them.
Item {
    id: bandWidest

    readonly property alias fetchCodeWidest: fetchCodeWidest
    readonly property alias fetchWordWidest: fetchWordWidest
    readonly property alias pushCodeWidest: pushCodeWidest
    readonly property alias stashCodeWidest: stashCodeWidest

    /// The narrowest a wording is drawn before the button gives it up altogether: two characters and the mark a cut
    /// leaves — counted in characters rather than pixels, for the reason the state badges' floor is (規約 §ウィンドウの縁:
    /// the same count costs a different number of pixels in each platform's font).
    ///
    /// **Drawn rather than added up.** Two things went wrong the other way, and both were silent. Counted in the
    /// family's *average* character — which is what the tab names and the state badges count in — the floor came out
    /// **above the box** on Linux (58 against a box of 52) and the wordings were never drawn at all: an average that
    /// counts a family's full-width glyphs is twice a mono advance, which is harmless where the box is a whole badge
    /// word and fatal where it is one short command. Summed from the parts, it came out a hair under what the same
    /// three glyphs are laid out at, and the cut that was meant to leave two characters left one (`p…`). Both 実測
    /// 2026-08-25.
    ///
    /// In the command family, whichever wording happens to be the widest: what this band says is commands, `Resume`
    /// being the one exception and the state a stopped timer leaves.
    readonly property real wordFloor: floorWidest.implicitWidth

    component Widest: Label {
        visible: false
        property bool code: false
        font.family: code ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: code ? -Theme.spaceXs : 0
        font.pixelSize: Theme.fontMd
    }
    Widest {
        id: fetchCodeWidest
        code: true
        text: "fetch"
    }
    Widest {
        id: fetchWordWidest
        text: qsTr("Resume")
    }
    Widest {
        id: pushCodeWidest
        code: true
        text: "push -f"
    }
    Widest {
        id: stashCodeWidest
        code: true
        text: "stash"
    }
    // Two characters and the mark a cut leaves, drawn the way a cut wording is drawn (`wordFloor`).
    Widest {
        id: floorWidest
        code: true
        text: "nn…"
    }
}
