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
    readonly property alias fetchCodeWidest: fetchCodeWidest
    readonly property alias fetchWordWidest: fetchWordWidest
    readonly property alias pushCodeWidest: pushCodeWidest
    readonly property alias stashCodeWidest: stashCodeWidest

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
}
