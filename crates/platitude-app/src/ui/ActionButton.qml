import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the
// word to be sure of it. Both halves dim together when disabled.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    /// The state's colour taken a step down for the wait — the ring and
    /// the frame wear it while git is on the network (デザイン規約
    /// §暗く落とした段).
    ///
    /// The state's, not `tone`'s: where the word stays plain and only the
    /// frame and the mark carry a warning, this follows the frame.
    /// A plain button has no darker step of its own and takes `textMuted`.
    property color toneDim: Theme.textMuted
    /// git is on the network for this button: the words step aside for a
    /// turning ring in the middle of the button, and the whole of it goes
    /// as dim and as deaf as a disabled one (デザイン規約 §長押し).
    ///
    /// Dimmed rather than actually disabled: `enabled` would take the
    /// focus away, and the press that started the network call is the
    /// very one that may have come from the keyboard.
    property bool busy: false
    /// Hold the turn still (automation — a spinning icon photographs
    /// differently every time).
    property bool still: false
    /// How long this button has to be held to fire `held()`; zero for an
    /// ordinary button, where a click is the whole gesture (デザイン規約
    /// §進行中・長押しの定数).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at what this button does did not work. Drawn as a mark
    /// standing clear of the word's last letter, in the word's own colour.
    property bool alert: false
    /// The mark's colour, where it is not the word's.
    property color alertTone: actionBtn.fg
    /// Pull the mark back to a letter's distance from the word.
    ///
    /// The mark sits at the end of the label's **advance** width, which
    /// leaves the last glyph's right side bearing as clear air — a word
    /// ending in `)` carries a lot of it, so the mark reads as a separate
    /// thing. One gap back puts it where the letters are.
    property bool alertTight: false
    /// The colour the hold fills the button with — the frame's, since a
    /// framed button fills the frame it drew. A bare one names its own
    /// (the hunk heading's `Discard hunk`, which fills edge to edge the
    /// way a held menu row does — デザイン規約 §長押し).
    property color holdTone: actionBtn.frameColor
    readonly property bool framed: actionBtn.frameColor.a > 0
    /// The label is a git command said in git's own spelling, and wears
    /// the chip that says so — lowercase, mono, on a faint ground
    /// (デザイン規約 §git 用語のコード表記). The command is the whole
    /// label, so `text` itself is what the chip is drawn around — and that
    /// text is never translated: it is the command, not a phrase about it.
    property bool code: false
    /// The icon is a mark standing next to the word, rather than an icon
    /// at the head of a band.
    ///
    /// The step goes with the role (デザイン規約 §寸法「段は役割で選ぶ」
    /// — `iconSm` is the mark beside a word, `iconMd` the icon a row or a
    /// band starts with), and the seat comes in to the icon itself: the
    /// wider seat is there to hold the icon and the hold mark side by
    /// side, and a button that is only ever clicked pays for a pairing it
    /// cannot have (デザイン規約 §余白「印が自分で持っている余白は、隣の
    /// 詰めに数える」).
    ///
    /// The toolbar keeps the wide seat: its buttons can pair, and their
    /// width is measured into a box two of them share.
    property bool besideWord: false
    /// Text the label's box is measured for, and whether that wording is
    /// a command (the two families measure differently, so the box has to
    /// be told which one it is holding). A button whose wording changes
    /// with its state would otherwise move everything beside it in the
    /// toolbar every time the state changed.
    property string widestText: ""
    property bool widestCode: false
    /// Icon and word centred as a pair rather than packed from the left.
    /// A toolbar button is measured to its content and never sees the
    /// difference; one told to fill a pane's width does — the icon would
    /// sit against the far edge with the word adrift from it.
    property bool centred: false
    /// What the shared box hands this state past its own wording: the box
    /// is measured for the longest thing either of the pair ever says, so
    /// every shorter wording is given air it has no use for.
    ///
    /// Split between the button's two ends rather than left where it
    /// falls — packed from the left it all lands behind the word, and the
    /// wash, the warning frame and the hold's fill all reach well past
    /// the last letter.
    ///
    /// The phrase moves whole, so the step from the icon to the word is
    /// the same in every state (デザイン規約 §余白). Centring the word
    /// inside the box instead would open that step wider than the gap to
    /// the button beside it.
    readonly property real slack:
        Math.max(0, btnLabel.box - btnLabel.implicitWidth,
                 actionBtn.floorSlack)
    /// What a framed button is short of `Theme.buttonMinWidth`, which it
    /// takes as slack like any other (so the phrase still moves whole and
    /// the ink still comes out even at the two ends).
    ///
    /// A frame is a box put round a word, and a short word draws a box the
    /// eye reads as a chip rather than as something to press.
    ///
    /// Only where a frame is drawn. A bare button is a word among words —
    /// the hunk heading's two, a dialog's `Cancel` — and a floor there
    /// would stretch the hover wash and the hold's fill well past what the
    /// button names.
    readonly property real floorSlack:
        actionBtn.framed
        ? Theme.buttonMinWidth - btnRow.implicitWidth - 2 * actionBtn.padding
        : 0
    /// The air each end is already holding before the slack is shared out
    /// (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」):
    /// a lone mark sits centred in the two-mark seat, and a command's chip
    /// reaches half a gap past its last letter. Taken off before the
    /// halves are cut, so what comes out even is the **ink** at the two
    /// ends rather than the row between them.
    readonly property real headInk:
        seat.visible ? (seat.implicitWidth - seat.step) / 2 : 0
    readonly property real tailInk: actionBtn.code ? Theme.spaceXs / 2 : 0
    /// Held all the way down.
    signal held()
    /// Pressed and let go, meaning the button's ordinary action.
    ///
    /// A hold button never emits this: neither the release that completes
    /// a hold nor the one that gives up on it part way may fall through
    /// to what this button does when it is not a hold button.
    signal activated()
    readonly property color fg: actionBtn.busy ? actionBtn.toneDim
                                : holdProgress > 0 ? Theme.textOnAccent
                                : enabled ? tone : Theme.textMuted
    /// Nothing here answers a press while git is on the network for it.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }

    // Tab reaches the buttons that need a second way in. The rest of the
    // toolbar stays out of the tab order: a hold is the only gesture here
    // that a pointer alone can fail to make (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0
    Accessible.description: actionBtn.holdMs > 0 ? qsTr("Hold to activate") : ""

    onClicked: if (actionBtn.holdMs <= 0 && actionBtn.live) actionBtn.activated()
    onDownChanged: {
        if (actionBtn.holdMs <= 0 || !actionBtn.live)
            return
        if (actionBtn.down)
            holdDrive.begin()
        else
            holdDrive.letUp()
    }
    // The hold's other hand: focus it, then hold Space or Enter. Accepting
    // the key keeps AbstractButton from also taking Space as a press, which
    // would drive the same fill from `down` a second time.
    //
    // Auto-repeat is dropped on both edges. A held key repeats its press on
    // every platform and its release on some, and either edge would restart
    // the fill from zero for as long as the key was held — the hold could
    // then never complete.
    Keys.onPressed: event => {
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdDrive.holdKey(event.key))
            return
        holdDrive.begin()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdDrive.holdKey(event.key))
            return
        holdDrive.letUp()
        event.accepted = true
    }
    HoldDriver {
        id: holdDrive
        holdMs: actionBtn.holdMs
        onFinished: actionBtn.held()
    }
    background: Rectangle {
        // The same wash every other tool button answers with
        // (`HoverToolButton.washColor`), read rather than left out: a
        // background handed in replaces the one that carries it, so the
        // wash, the frame, the hold's fill and the focus ring all have to
        // be drawn here.
        //
        // Only while it is answering: a button with git out on the network
        // is as deaf as a disabled one (`live`), and the hand that started
        // the fetch is still resting on it — the wash must not stay up for
        // the whole call (2026-08-10 報告).
        color: actionBtn.live ? actionBtn.washColor : "transparent"
        // The frame goes a step down with the rest of the button while git
        // is out on the network: the button is still the one that
        // overwrites a remote, and a frame that dropped to grey would take
        // that back for as long as the wait lasted.
        //
        // A button with no frame of its own grows none: the ring already
        // says the wait has started, and a frame drawn around a button
        // that has never worn one reads as a box laid over the band
        // (2026-08-10 報告「ちょっと浮いて見えた」).
        border.color: actionBtn.busy && actionBtn.framed
                      ? actionBtn.toneDim : actionBtn.frameColor
        border.width: Theme.borderWidth
        radius: Theme.radiusSm
        // The hold, filling from the left. Inset by the border where
        // there is one, so the frame stays a frame while it fills; a bare
        // button fills edge to edge, the way a held menu row does.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: actionBtn.framed ? Theme.borderWidth : 0
            radius: Theme.radiusSm
            // Never thinner than `holdFillMin` while it runs: proportional
            // from zero, the first tenth of the hold is a sub-pixel sliver,
            // so the press reads as not having taken and the whole gesture
            // feels longer than it is (デザイン規約 §進行中・長押しの定数).
            width: actionBtn.holdProgress > 0
                   ? Math.max(Metrics.holdFillMin,
                              (parent.width - 2 * anchors.margins)
                              * actionBtn.holdProgress)
                   : 0
            color: actionBtn.holdTone
            visible: actionBtn.holdProgress > 0
        }
        // Drawn outside the frame rather than in it: the frame's colour is
        // already saying this button is the dangerous one, and focus must
        // not be able to take that over.
        //
        // `visualFocus`, which is focus that arrived from the keyboard —
        // not `activeFocus`, which a press gives it as well (`focusPolicy`
        // is StrongFocus and nothing on this band takes it back, so an
        // `activeFocus` ring would come up on a click and stay —
        // 2026-08-10 報告「押したら色が解除されなくなった」).
        Rectangle {
            anchors.fill: parent
            anchors.margins: -Theme.spaceXs / 2
            color: "transparent"
            border.color: Theme.borderFocus
            border.width: Theme.borderWidth
            radius: Theme.radiusMd
            visible: actionBtn.visualFocus
        }
    }
    // The toolbar's size unless an instance says otherwise — the hunk
    // heading's words take fontSm there, a step under the file's own
    // word (デザイン規約 §diff の中のステージ).
    font.pixelSize: Theme.fontMd

    // The slack goes into the button's own air, so what one end takes the
    // other gives: the width stays the one the pair was measured for and
    // the toolbar's right-hand end does not move (デザイン規約
    // §リモートへ送る — 全状態が同じ幅).
    //
    // The trailing side is what is left rather than the other half
    // rounded: a wording measures in fractions of a pixel, and `floor`
    // beside `ceil` would round the pair of them up to a button a pixel
    // wider than its box. The odd pixel also lands on the side with a
    // mark to stand clear of (`alert`).
    // Only a button with slack to share reads the ink off its two ends —
    // one measured into a shared box (`widestText`) or one held open by
    // the frame's floor (`floorSlack`). A button sized to its own content
    // is already as tight as the two ends can be, and taking the ink off
    // its padding would move every button in the app for the sake of the
    // few that have room to give.
    readonly property real headAir:
        (btnLabel.box > 0 || actionBtn.floorSlack > 0)
        ? Math.floor((actionBtn.slack - actionBtn.headInk
                      - actionBtn.tailInk) / 2)
        : 0
    leftPadding: actionBtn.padding + actionBtn.headAir
    rightPadding: actionBtn.padding + (actionBtn.slack - actionBtn.headAir)

    // The row keeps its size while the network call runs — the toolbar
    // must not shuffle under a pointer that is still resting on the
    // button — so the words step aside by going transparent rather than
    // by leaving the layout, and the ring turns over the middle of what
    // they left.
    contentItem: Item {
        implicitWidth: btnRow.implicitWidth
        implicitHeight: btnRow.implicitHeight

        RowLayout {
            id: btnRow
            anchors.fill: parent
            spacing: Theme.spaceXs
            opacity: actionBtn.busy ? 0 : 1
            Item {
                Layout.fillWidth: true
                visible: actionBtn.centred
            }
            ActionButtonSeat {
                id: seat
                kind: actionBtn.kind
                holdMs: actionBtn.holdMs
                besideWord: actionBtn.besideWord
                holdProgress: actionBtn.holdProgress
                tint: actionBtn.fg
                Layout.alignment: Qt.AlignVCenter
            }
            ActionButtonLabel {
                id: btnLabel
                text: actionBtn.text
                code: actionBtn.code
                widestText: actionBtn.widestText
                widestCode: actionBtn.widestCode
                tint: actionBtn.fg
                alert: actionBtn.alert
                alertTone: actionBtn.alertTone
                alertTight: actionBtn.alertTight
                fontSize: actionBtn.font.pixelSize
            }
            Item {
                Layout.fillWidth: true
                visible: actionBtn.centred
            }
        }
        // Its own item rather than a rotation on the icon above: an
        // animator leaves the angle where it stopped, and the icon that
        // returns must not come back tilted.
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            kind: "spinner"
            tint: actionBtn.toneDim
            visible: actionBtn.busy
            // On the render thread, so it keeps turning while the GUI
            // thread drains models.
            RotationAnimator on rotation {
                running: actionBtn.busy && !actionBtn.still
                loops: Animation.Infinite
                from: 0
                to: 360
                duration: Metrics.spinMs
            }
        }
    }
}
