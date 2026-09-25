import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The band's shared button box, measured: the four wordings that cover all seven states, each drawn invisibly as a
// button draws it; the band hands the widest to all three (`TopBar.widestAction`). `stash` never wins but is measured,
// so a re-wording cannot leave the box short. Labels, not `TextMetrics`, which reads a few pixels tighter.
Item {
    id: bandWidest

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
