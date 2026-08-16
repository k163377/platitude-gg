import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The word half of a toolbar action: the wording itself, a command's
// flag drawn rather than typed, the chip a command wears, and the mark
// that says the last go did not work. The box it is measured into is the
// widest wording the button ever says, so the toolbar does not move when
// the state does.
Item {
    id: btnLabel

    /// What this state says.
    property string text: ""
    /// The label is a git command said in git's own spelling, and wears
    /// the chip that says so (デザイン規約 §git 用語のコード表記).
    property bool code: false
    /// Text the box is measured for, and whether that wording is a
    /// command (the two families measure differently, so the box has to
    /// be told which one it is holding).
    property string widestText: ""
    property bool widestCode: false
    /// The colour the word is drawn in — the button's own `fg`.
    property color tint: Theme.textPrimary
    /// The last go at what this button does did not work.
    property bool alert: false
    property color alertTone: btnLabel.tint
    /// Pull the mark back to a letter's distance from the word.
    property bool alertTight: false
    property int fontSize: Theme.fontMd

    // The box is the widest wording's ink and nothing else.
    // What stands between that ink and the frame is the
    // button's own air, shared out by one rule in every state
    // (`ActionButton.slack`) — the widest included, whose slack is
    // nothing and whose air is therefore the padding itself. No extra
    // gap charged to one family and not the other: that makes
    // the box jump whenever the two wordings cross in width.
    //
    // Measured from the font even where the flag is drawn (see
    // below) — the box is what holds the toolbar still, and it
    // must not move when a shorter rule is chosen for the flag.
    readonly property real box: widest.implicitWidth
    /// A command's flag, set apart from the command itself so
    /// its dashes can be drawn rather than typed. Every dash
    /// the mono family carries is the same 7px rule in an 8px
    /// cell (measured over U+002D / 2010 / 2011 / 2212), and
    /// on the wording the shared box was measured for that is
    /// what leaves the mark no room past the word. Drawn, the
    /// rule's length and the air either side are ours to pick
    /// (デザイン規約 §git 用語のコード表記).
    readonly property int flagAt: btnLabel.code
                                  ? btnLabel.text.indexOf(" -") : -1
    readonly property bool splitFlag: btnLabel.flagAt > 0
    readonly property string head: btnLabel.splitFlag
        ? btnLabel.text.substring(0, btnLabel.flagAt) : btnLabel.text
    /// The flag with its leading dashes taken off, and how
    /// many of them there were.
    readonly property string flagRest: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).replace(/^-+/, "") : ""
    readonly property int dashCount: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).length
          - btnLabel.flagRest.length : 0

    implicitWidth: headText.width
                   + (btnLabel.splitFlag ? flagRow.width : 0)
    implicitHeight: headText.implicitHeight
    Layout.maximumWidth: 240
    // Its own width: what the shared box asks for past this
    // wording is held by the button's padding (`slack`), so
    // the chip and the mark, both measured off this cell,
    // keep sitting on the word.
    Layout.preferredWidth: btnLabel.implicitWidth
    Layout.alignment: Qt.AlignVCenter

    // Measured, never drawn: a hidden item is left out of the
    // layout, and a Label measures the way the visible one does —
    // TextMetrics reports a few pixels tighter, which is enough of
    // a difference to shift the toolbar it is here to hold still.
    Label {
        id: widest
        visible: false
        text: btnLabel.widestText
        font.family: btnLabel.widestCode ? Theme.monoFamily
                                         : Theme.uiFamily
        font.wordSpacing: btnLabel.widestCode ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    Label {
        id: headText
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // Bounded by the cell's own ceiling rather than by its
        // width: the width comes from this, so reading it back
        // would close a loop.
        width: Math.min(implicitWidth, btnLabel.Layout.maximumWidth)
        text: btnLabel.head
        color: btnLabel.tint
        font.family: btnLabel.code ? Theme.monoFamily
                                   : Theme.uiFamily
        // A command and its flag are one thing said, and a mono
        // space is far wider than the air the chip keeps at its
        // own ends — left alone, `-f` drifts away from the
        // `push` it belongs to and the chip reads as two words
        // on one ground (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
        elide: Text.ElideRight
    }
    // The flag: the air the mono space held, a drawn rule for
    // each dash, and the letters after them in the font. The
    // rule is `borderWidth` thick because that is what the
    // font's own dash measures at this size, and a hair of air
    // follows it so the letter does not touch.
    Row {
        id: flagRow
        visible: btnLabel.splitFlag
        anchors.left: headText.right
        anchors.verticalCenter: headText.verticalCenter
        spacing: 0
        Item {
            width: Theme.spaceXs
            height: headText.height
        }
        Repeater {
            model: btnLabel.dashCount
            Item {
                width: Theme.spaceXs + Theme.borderWidth
                height: headText.height
                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.spaceXs
                    height: Theme.borderWidth
                    color: btnLabel.tint
                }
            }
        }
        Label {
            text: btnLabel.flagRest
            color: btnLabel.tint
            font.family: Theme.monoFamily
            font.pixelSize: btnLabel.fontSize
        }
    }
    // The chip a command wears, behind the glyphs and only as
    // wide as they are — the box around them is measured for
    // the longest wording of the pair, and a ground stretched
    // to that would draw a chip the word does not fill. Half a
    // gap of tint hangs off either end, the same as a menu
    // row's (デザイン規約 §git 用語のコード表記).
    Rectangle {
        z: -1
        visible: btnLabel.code
        x: -Theme.spaceXs / 2
        width: btnLabel.implicitWidth + Theme.spaceXs
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    // Past the word's end — past the chip's edge where there
    // is one (a mark crossing that edge reads as stuck to the
    // chip rather than said after the word). Closer still
    // after a flag: that is the longest thing the button says
    // and the one wording whose right-hand side is short of
    // room (デザイン規約 §リモートへ送る).
    NavIcon {
        visible: btnLabel.alert
        kind: "bang"
        tint: btnLabel.alertTone
        width: Theme.iconSm
        height: Theme.iconSm
        x: btnLabel.implicitWidth
           - (btnLabel.splitFlag ? Theme.spaceXs / 2 : 0)
           - (btnLabel.alertTight ? Theme.spaceXs : 0)
        y: -Theme.spaceXs
    }
}
