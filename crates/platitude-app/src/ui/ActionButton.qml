import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the word to be sure of it.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    /// The state's colour a step down, worn by the ring, the frame and the marks while git is on the network
    /// (デザイン規約 §暗く落とした段). Where only the frame and the mark carry a warning it follows the frame, not the
    /// word; a plain button has no darker step and takes `textMuted`.
    property color toneDim: Theme.textMuted
    /// git is on the network for this button: the ring takes the icon's seat, the word drops to the disabled colour,
    /// and presses are ignored — with `enabled` kept, since dropping it takes the focus from a keyboard press
    /// (デザイン規約 §進行中・長押しの定数).
    property bool busy: false
    /// How long a press has to last to fire `held()`; zero for an ordinary click button (デザイン規約 §進行中・長押しの定数).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The length the press under way was given (`HoldDriver.armedMs`; the live one between presses). An owner that
    /// dresses itself from the same condition as the length reads this, so nothing changes under a hand already
    /// holding (rules-refs の「長押しか否かは押した瞬間に確定する」の行).
    readonly property alias armedMs: holdDrive.armedMs
    /// A gesture is under way, the fill's slide back included (`HoldDriver.gesturing`) — what an owner latches on when
    /// it dresses itself from an answer other than the length.
    readonly property alias gesturing: holdDrive.gesturing
    /// What this button's press is aimed at (`HoldDriver.premise`) — set it wherever the target can be swapped out
    /// from under the hand without the length moving: a band button re-pointed at another tab, a list delegate re-used
    /// for another row.
    property alias premise: holdDrive.premise
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at this button's action failed: a mark standing clear of the word's last letter.
    property bool alert: false
    /// The mark's colour, where it is not the word's.
    property color alertTone: actionBtn.fg
    /// Pull the mark back to a letter's distance from the word: it sits at the end of the label's advance width, and a
    /// word ending in `)` leaves so much side bearing there that the mark reads as a separate thing.
    property bool alertTight: false
    /// The hold's fill colour — the frame's by default. A bare button names its own and fills edge to edge like a held
    /// menu row (the hunk heading's `Discard hunk` — デザイン規約 §長押し).
    property color holdTone: actionBtn.frameColor
    readonly property bool framed: actionBtn.frameColor.a > 0
    /// The whole label is a git command in git's own spelling and wears the code chip (デザイン規約 §git 用語のコード表記);
    /// `text` is then left untranslated.
    property bool code: false
    /// The word over the mark rather than beside it — the operation panel's shape, two lines deep. The height does not
    /// move: narrowing, a stacked button gives up its word and the mark grows into the room the word was in
    /// (`stackSeat.step`).
    property bool stacked: false
    property int wordWeight: Font.Normal
    /// The label is a phrase with a command at each end — a chip, `text` between them, and a second chip in a colour
    /// of its own (`ActionButtonLabel.phraseHead`). Empty is the ordinary single-wording button.
    property string phraseHead: ""
    property string phraseCount: ""
    property string phraseTail: ""
    property color phraseTailTint: actionBtn.fg
    /// A short warning after the phrase's words — a menu row's tag brought inside the phrase
    /// (`ActionButtonLabel.phraseNote`).
    property string phraseNote: ""
    property color phraseNoteTint: Theme.warning
    /// Whom the action will be attributed to, at the end of the phrase, and how many more it credits
    /// (`ActionButtonLabel.phraseFace`).
    property int phraseFace: -1
    property string phraseFaceUrl: ""
    property int phraseMates: 0
    property string phraseSignature: ""
    property string phraseSignatureTip: ""
    /// Stands in for the pointer on that tick, so its one line can be photographed — hover cannot be injected.
    property bool phraseSignaturePointedAt: false
    readonly property bool phraseSignatureTipShown: btnLabel.phraseSignatureTipShown
    /// The icon is a mark beside the word: `iconSm` rather than `iconMd` (デザイン規約 §寸法「段は役割で選ぶ」), and the
    /// seat shrinks to the icon — the wide seat holds the icon and the hold mark side by side, a pairing a click-only
    /// button cannot have (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」). The toolbar keeps the
    /// wide seat: its buttons can pair.
    property bool besideWord: false
    /// The wording the label's box is measured for, and whether it is a command (the two families measure
    /// differently), so a button whose wording changes with its state does not move its neighbours.
    property string widestText: ""
    property bool widestCode: false
    /// The narrowest this button draws its word before giving it up for the icon alone (`BandWidest.wordFloor`).
    /// Zero, the default, is a button measured to its own content — every one outside the band; only a cell the band
    /// lays out (`TopBar`) can be narrower than its word (規約 §ウィンドウの縁).
    property real wordFloor: 0
    /// Told from outside that the operation panel has reached the width where the words go, whether or not this cell
    /// is short of room yet (`TopBar.actionsFolded` / `findFolded`). The button also gives up on its own when the cell
    /// it was handed cannot hold the floor.
    property bool foldRequested: false
    /// The room the word gets in this cell once the marks and the air either side are paid for. Read off the style's
    /// fixed `padding`: `leftPadding` / `rightPadding` are answered from here (through `slack`), and reading them back
    /// would close a loop.
    readonly property real wordRoom:
        actionBtn.wordFloor <= 0 ? btnLabel.box
        : Math.max(0, actionBtn.width - 2 * actionBtn.padding - seat.implicitWidth - btnRow.spacing)
    /// Down to the icon: nothing left to say the word in, or the band has said so.
    readonly property bool folded: actionBtn.wordFloor > 0
        && (actionBtn.foldRequested || actionBtn.wordRoom < actionBtn.wordFloor)
    /// The widest this button is ever drawn, and the narrowest it is drawn with a word still on it. Neither moves with
    /// the shape the button is in — the row lays the cell out from these, so giving the word up cannot change the
    /// width that decides whether to give it up (`TopBar`).
    readonly property real naturalWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing + btnLabel.box
    /// The flag is counted into the floor because it never gives (`ActionButtonLabel.flagRoom`): without it `push -f`
    /// is cut to a `…` and one letter before the fold. The set folds on the widest of its floors (`TopBar.actionFold`).
    readonly property real foldWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing
        + actionBtn.wordFloor + btnLabel.flagRoom
    /// Automation: the ink left after a cut, and whether there was one — a picture cannot tell a wording's own `…`
    /// from an elision (`PGG_AUTO_ACT=band-actions`).
    readonly property real wordInk: btnLabel.inkWidth
    readonly property bool wordCut: btnLabel.capped
    /// …the width this state's wording wants, and the box the set was measured for: a run aiming between the two
    /// shapes needs both, since the cell that starts cutting is the wording plus `naturalWidth - wordBox`.
    readonly property real wordWant: btnLabel.wantWidth
    readonly property real wordBox: btnLabel.box
    /// Icon and word centred as a pair, for a button told to fill a pane's width — packed from the left, the word
    /// would sit adrift from its icon.
    property bool centred: false
    /// What the shared box (the longest wording of the set) hands this state past its own wording. Split between the
    /// two ends with the phrase moving whole: all of it behind the word stretches the wash, the frame and the fill past
    /// the last letter, and centring the word alone would open the icon-to-word step (デザイン規約 §余白).
    ///
    /// A cell narrower than the box has no slack — its word elides (`ActionButtonLabel.cap`). Measured against the
    /// *wanted* width: the painted width is what this arithmetic produces.
    readonly property real slack:
        actionBtn.folded ? 0
        : Math.max(0, Math.min(btnLabel.box, actionBtn.wordRoom) - btnLabel.wantWidth, actionBtn.floorSlack)
    /// What a framed button is short of `Theme.buttonMinWidth`, taken as slack like any other — a short word in a
    /// frame reads as a chip, not a button.
    ///
    /// Framed only: on a bare button the floor stretches the wash and the fill past the word. Not for a phrase: its
    /// cell asks for no width (`ActionButtonLabel.phraseRoom`), so the floor would charge the padding a whole
    /// `buttonMinWidth` out of the room the phrase elides into. Not once folded: a box round a mark takes the mark's
    /// size (規約 §ウィンドウの縁「枠を着ていても `buttonMinWidth` の外」).
    readonly property real floorSlack:
        actionBtn.framed && !btnLabel.phrased && !actionBtn.folded
        ? Theme.buttonMinWidth - btnRow.implicitWidth - 2 * actionBtn.padding : 0
    /// The air each end already holds (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」): a lone mark
    /// centred in the two-mark seat, a chip reaching half a gap past its last letter. Taken off before the halves are
    /// cut, so the ink comes out even.
    readonly property real headInk: seat.visible ? (seat.implicitWidth - seat.step) / 2 : 0
    readonly property real tailInk: actionBtn.code ? Theme.spaceXs / 2 : 0
    /// Held all the way down.
    signal held()
    /// The ordinary action: a plain press let go. A hold button raises it on neither release — the one that completes
    /// the hold or the one that gives up part way.
    signal activated()
    readonly property color fg: actionBtn.busy ? Theme.textMuted
                                : holdProgress > 0 ? Theme.textOnAccent : enabled ? tone : Theme.textMuted
    /// The marks' colour: while busy, the state a step down like the ring and the frame
    /// (デザイン規約 §進行中・長押しの定数「リング・枠・印は、そのボタンが待つ前に着ていた色の 1 段下」). Apart from `fg`
    /// because a dimmed word has one colour only (§無効), while a mark keeps its hue so a wait says which button waits.
    readonly property color markFg: actionBtn.busy ? actionBtn.toneDim : actionBtn.fg
    /// A press lands only with git off the network for this button.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: run the hold without a press, gated the way a hand's press is (`onDownChanged`). Answers whether it
    /// went in, so a run that met the button busy presses again later rather than filling a hold nothing can land.
    function completeHold() {
        if (!actionBtn.live)
            return false
        holdDrive.begin()
        return true
    }

    // Only hold buttons join the tab order (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0
    Accessible.description: actionBtn.holdMs > 0 ? Words.holdToActivate : ""

    /// Whether the press just let go was a plain one, judged before the latch opens — the click follows the release in
    /// the same delivery, when the length is live again. `clickJudged` says there was a press at all: `clicked()` is
    /// also raised with no press behind it (`TopBar.stashNow` / `fetchNow`, `TabStrip`, `WipBucketHeader`), and that
    /// one is answered from the live length.
    property bool clickWanted: false
    property bool clickJudged: false
    onClicked: {
        if (!actionBtn.live)
            return
        const plain = actionBtn.clickJudged ? actionBtn.clickWanted : actionBtn.holdMs <= 0
        actionBtn.clickJudged = false
        if (plain)
            actionBtn.activated()
    }
    /// Gone deaf under the hand (git went onto the network, or the state it acts on went): the fill blanks and no
    /// signal is raised.
    onLiveChanged: if (!actionBtn.live) {
        // Both, so a following press-less `clicked()` is judged from the live length.
        actionBtn.clickWanted = false
        actionBtn.clickJudged = false
        holdDrive.blank()
    }
    onDownChanged: {
        // Only the press is gated: an unanswered release leaves the fill running to fire a hold after the hand let go
        // (and the latch below never reopens).
        if (actionBtn.down && !actionBtn.live)
            return
        if (actionBtn.down) {
            // Whatever the length: a plain press is answered from the latch too, and a press that never closed it can
            // still change meaning under the hand.
            actionBtn.clickJudged = false
            holdDrive.begin()
            return
        }
        // A hold reports no click, and a press whose premise went reports nothing (`HoldDriver.stale` —
        // デザイン規約 §長押し).
        actionBtn.clickWanted = holdDrive.armedMs <= 0 && !holdDrive.stale
        actionBtn.clickJudged = true
        holdDrive.letUp()
    }
    // The hold's other hand (`HoldDriver.pressKey`), offered only while `live`: a button waiting on git answers no key
    // either.
    Keys.onPressed: event => {
        if (actionBtn.live)
            holdDrive.pressKey(event)
    }
    Keys.onReleased: event => {
        if (actionBtn.live)
            holdDrive.releaseKey(event)
    }
    // A key's release goes wherever the focus is by then: moved away, this fill would run out and fire a hold nobody
    // was still making (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!actionBtn.activeFocus) holdDrive.focusLost()
    HoldDriver {
        id: holdDrive
        holdMs: actionBtn.holdMs
        onFinished: actionBtn.held()
    }
    /// How far in from the cell's top and bottom the frame is drawn. A folded cell fills the band's height like the ☰
    /// and the window's three, and the frame keeps the button's own box height — a line along the band's top edge
    /// reads as a box glued to the window. The wash still fills the whole cell: that is the target.
    ///
    /// Read only when folded: elsewhere the height is the implicit height, and a binding reading both is a loop to the
    /// engine even where the arithmetic is not.
    readonly property real frameInset:
        actionBtn.folded ? Math.max(0, (actionBtn.height - actionBtn.implicitHeight) / 2) : 0
    /// The fill inside the frame, for a button that has to read as a box laid on its ground (the operation panel's).
    /// Under the wash, so the hand still lights it.
    property color faceColor: "transparent"

    background: Rectangle {
        id: btnGround
        color: "transparent"
        radius: Theme.radiusSm
        // The frame and the hold's fill: its own item, so a cell taller than the box washes edge to edge with the
        // frame still round the box (`frameInset`).
        Rectangle {
            anchors.fill: parent
            anchors.topMargin: actionBtn.frameInset
            anchors.bottomMargin: actionBtn.frameInset
            color: actionBtn.faceColor
            // While busy the frame goes a step down rather than to grey (grey would take its warning back for the
            // whole wait), and a button with no frame grows none (デザイン規約 §進行中・長押しの定数).
            border.color: actionBtn.busy && actionBtn.framed ? actionBtn.toneDim : actionBtn.frameColor
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            HoldFill {
                progress: actionBtn.holdProgress
                tone: actionBtn.holdTone
                inset: actionBtn.framed ? Theme.borderWidth : 0
            }
        }
        // `HoverToolButton`'s wash (`washColor`), redrawn because a background handed in replaces the one that carried
        // it. Over the face (an opaque face would hide it) and under the focus ring, across the whole cell so the
        // target matches its neighbours'. Only while `live` (デザイン規約 §進行中・長押しの定数).
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusSm
            color: actionBtn.live ? actionBtn.washColor : "transparent"
        }
        // Outside the frame: the frame's colour is saying this button is the dangerous one (デザイン規約 §長押し).
        // `visualFocus`, not `activeFocus`: a click gives focus too and nothing on the band takes it back, so the ring
        // would come up on a click and stay.
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
    // The toolbar's size; the hunk heading's words override it (デザイン規約 §diff の中のステージ).
    font.pixelSize: Theme.fontMd

    // The slack goes into the button's own air, so the width stays the one the set was measured for
    // (デザイン規約 §リモートへ送る — 全状態が同じ幅). The trailing side takes what is left: `floor` beside `ceil` on a
    // fractional wording makes the button a pixel wider than its box, and the odd pixel lands by the `alert` mark.
    //
    // Only a button with slack (a shared box or the frame's floor) reads the ink off its ends: on one sized to its
    // content that would move every button in the app.
    readonly property real headAir:
        actionBtn.folded ? 0
        : (btnLabel.box > 0 || actionBtn.floorSlack > 0)
          ? Math.floor((actionBtn.slack - actionBtn.headInk - actionBtn.tailInk) / 2)
          : 0
    // Folded, no padding: the mark is centred in an end-cell-wide cell like the ☰'s
    // (規約 §ウィンドウの縁「帯の両端は同じ組み方」).
    leftPadding: actionBtn.folded ? 0 : actionBtn.padding + actionBtn.headAir
    rightPadding: actionBtn.folded ? 0 : actionBtn.padding + (actionBtn.slack - actionBtn.headAir)
    // Stacked, nothing over or under the pair: the row already keeps its own step (`opsBarHeight`).
    topPadding: actionBtn.stacked ? 0 : actionBtn.padding
    bottomPadding: actionBtn.stacked ? 0 : actionBtn.padding

    /// A band-laid button's `implicitWidth` does not move with its shape, like `naturalWidth` / `foldWidth`. The
    /// Control's own is content plus paddings, both answered from the width the row handed over (`wordRoom` →
    /// `folded` → `slack`), and until the row sets a width `setImplicitWidth` writes one — a binding loop at startup.
    ///
    /// Only where the row lays the cell out: elsewhere it would leave the wording adrift in a box it was never
    /// measured for.
    Binding {
        target: actionBtn
        property: "implicitWidth"
        value: actionBtn.naturalWidth
        when: actionBtn.wordFloor > 0
    }

    contentItem: Item {
        implicitWidth: actionBtn.stacked ? stackCol.implicitWidth : btnRow.implicitWidth
        implicitHeight: actionBtn.stacked ? stackCol.implicitHeight : btnRow.implicitHeight

        RowLayout {
            id: btnRow
            anchors.fill: parent
            visible: !actionBtn.stacked
            spacing: Theme.spaceXs
            Item {
                Layout.fillWidth: true
                // Hidden for a phrase, which fills the row and centres itself — spacers would split the slack three
                // ways and set it off-centre. Shown when folded, so the lone mark centres like its neighbours'.
                visible: (actionBtn.centred || actionBtn.folded) && !btnLabel.phrased
            }
            ActionButtonSeat {
                id: seat
                kind: actionBtn.kind
                // A phrase carries the hold's mark itself (`ActionButtonLabel.phraseHoldMs`): this seat sits at the
                // row's left edge, apart from the centred phrase.
                holdMs: btnLabel.phrased ? 0 : actionBtn.armedMs
                besideWord: actionBtn.besideWord
                holdProgress: actionBtn.holdProgress
                busy: actionBtn.busy
                tint: actionBtn.markFg
                // Folded, the `!` comes to the mark's corner: the warning survives the fold
                // (規約 §長押し「警告の色は枠と印が持つ」).
                cornerAlert: actionBtn.folded && actionBtn.alert
                cornerAlertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                Layout.alignment: Qt.AlignVCenter
            }
            ActionButtonLabel {
                id: btnLabel
                // Out of the row once folded: a zero-width child still takes `spacing` on both sides and pulls the
                // mark half a gap off centre. Its bindings keep answering while hidden.
                //
                // This cannot unfold the button: the fold is decided by `wordRoom`, which never reads this row — a
                // fold read off the row would hand the word its room back by hiding it.
                visible: !actionBtn.folded
                text: actionBtn.text
                code: actionBtn.code
                wordWeight: actionBtn.wordWeight
                widestText: actionBtn.widestText
                widestCode: actionBtn.widestCode
                // -1 is "as much as it wants": every button the band does not lay out.
                cap: actionBtn.wordFloor > 0 ? actionBtn.wordRoom : -1
                folded: actionBtn.folded
                tint: actionBtn.fg
                alert: actionBtn.alert
                // A step down while busy, like every mark (デザイン規約 §進行中・長押しの定数).
                alertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                alertTight: actionBtn.alertTight
                fontSize: actionBtn.font.pixelSize
                phraseHead: actionBtn.phraseHead
                phraseHoldMs: actionBtn.armedMs
                phraseHoldProgress: actionBtn.holdProgress
                phraseCount: actionBtn.phraseCount
                phraseTail: actionBtn.phraseTail
                phraseTailTint: actionBtn.phraseTailTint
                phraseNote: actionBtn.phraseNote
                phraseNoteTint: actionBtn.phraseNoteTint
                phraseFace: actionBtn.phraseFace
                phraseFaceUrl: actionBtn.phraseFaceUrl
                phraseMates: actionBtn.phraseMates
                phraseSignature: actionBtn.phraseSignature
                phraseSignatureTip: actionBtn.phraseSignatureTip
                phraseSignaturePointedAt: actionBtn.phraseSignaturePointedAt
            }
            Item {
                Layout.fillWidth: true
                visible: (actionBtn.centred || actionBtn.folded) && !btnLabel.phrased
            }
        }

        // The word over the mark (`stacked`), as its own pair: a `RowLayout` cannot be turned, and the layout that can
        // took the window's floor down with it.
        ColumnLayout {
            id: stackCol
            visible: actionBtn.stacked
            // Centred, not filling the cell: a layout shares extra room out between its cells, and each button would
            // draw its mark at a height of its own.
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.verticalCenter
            // The seat carries air its ink never uses (`ActionButtonSeat.implicitHeight`) while the word is ink from
            // its first row, so centred by their boxes the pair reads high.
            anchors.verticalCenterOffset: Metrics.opticalDrop
            // No spacing: the word's descent and the seat's step over its ink are already air between the lines, and
            // more puts the mark further from the word than the word is from the frame.
            spacing: 0

            ActionButtonLabel {
                id: stackLabel
                Layout.alignment: Qt.AlignHCenter | Qt.AlignBottom
                visible: !actionBtn.folded
                text: actionBtn.text
                code: actionBtn.code
                // The step's line, not the family's, so the marks under the words stand on one row
                // (`ActionButtonLabel.lineBox`).
                lineBox: Theme.fontSmLine
                wordWeight: actionBtn.wordWeight
                widestText: actionBtn.widestText
                widestCode: actionBtn.widestCode
                cap: actionBtn.wordFloor > 0 ? actionBtn.wordRoom : -1
                folded: actionBtn.folded
                tint: actionBtn.fg
                alert: actionBtn.alert
                alertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                alertTight: actionBtn.alertTight
                // A step under the band's: two lines in a two-line row, and the word is the half a reader already
                // knows.
                fontSize: Theme.fontSm
            }
            ActionButtonSeat {
                id: stackSeat
                // Folded, on the button's middle: held to the top it would sit in the upper half of a two-line box.
                Layout.alignment: actionBtn.folded ? Qt.AlignHCenter | Qt.AlignVCenter
                                                   : Qt.AlignHCenter | Qt.AlignTop
                // A step up: the seat's air is uneven about its ink, so a mark at the seat's middle rests low. The two
                // margins cancel, so the cell keeps its depth.
                Layout.topMargin: actionBtn.folded ? 0 : -Metrics.opticalDrop
                Layout.bottomMargin: actionBtn.folded ? 0 : Metrics.opticalDrop
                kind: actionBtn.kind
                holdMs: actionBtn.armedMs
                // The mark is what this button is read by, so it takes a lone band mark's step (規約 §寸法); folded,
                // it grows into the word's room.
                step: actionBtn.folded ? Theme.iconXl : Theme.iconLg
                besideWord: false
                holdProgress: actionBtn.holdProgress
                busy: actionBtn.busy
                tint: actionBtn.markFg
                cornerAlert: actionBtn.folded && actionBtn.alert
                cornerAlertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
            }
        }
    }
}
