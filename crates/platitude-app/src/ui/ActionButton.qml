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
    /// (デザイン規約 §進行中・回数押しの定数).
    property bool busy: false
    /// How many presses fire `held()`; zero for an ordinary click button (デザイン規約 §進行中・回数押しの定数).
    property int holdPresses: 0
    /// How far into the presses the button has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The count the gesture under way was given (`HoldDriver.armedPresses`; the live one between gestures). An owner
    /// that dresses itself from the same condition as the count reads this, so nothing changes between presses
    /// (rules-refs の「回数押しか否かは最初の押下で確定する」の行).
    readonly property alias armedPresses: holdDrive.armedPresses
    /// A gesture is under way, its completion included (`HoldDriver.gesturing`) — what an owner latches on when it
    /// dresses itself from an answer other than the count.
    readonly property alias gesturing: holdDrive.gesturing
    /// The ring turns in the mark's seat: git is on the network for this button, and the full fill of the press that
    /// started it has had its floor (デザイン規約 §回数押し).
    readonly property bool ringing: actionBtn.busy && !holdDrive.settling
    /// What this button's press is aimed at (`HoldDriver.premise`) — set it wherever the target can be swapped out
    /// from under the hand without the count moving: a band button re-pointed at another tab, a list delegate re-used
    /// for another row.
    property alias premise: holdDrive.premise
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at this button's action failed: a mark standing clear of the word's last letter.
    property bool alert: false
    /// The mark's colour, where it is not the word's.
    property color alertTone: actionBtn.fg
    /// The fill's colour — the frame's by default. A bare button names its own and fills edge to edge like a confirmed
    /// menu row (the hunk heading's `Discard hunk` — デザイン規約 §回数押し).
    property color holdTone: actionBtn.frameColor
    /// The hold mark's colour while it is not over the fill or waiting: what pressing through costs, the fill's, beside
    /// grey words too (デザイン規約 §回数押しの色). The hunk heading's asleep `Discard hunk` names its grey words' instead.
    property color holdHue: actionBtn.holdTone
    readonly property bool framed: actionBtn.frameColor.a > 0
    /// The whole label is a git command in git's own spelling and wears the code chip (デザイン規約 §git 用語のコード表記);
    /// `text` is then left untranslated.
    property bool code: false
    /// The word over the mark rather than beside it — the operation panel's shape, two lines deep. The height does not
    /// move: narrowing, a stacked button gives up its word and the mark grows into the room the word was in
    /// (`stackSeat.step`).
    property bool stacked: false
    /// The mark's step under its word in such a button: the mark is what it is read by, so it takes a lone band mark's
    /// step (規約 §寸法).
    readonly property int stackStep: Theme.iconLg
    /// How deep the word over the mark is, the word given up or not: the word's line and the seat at the step it has
    /// under a word. Whoever holds the button to a depth of its own keeps it at least this (`TopBar.actionDepth`) —
    /// in a shallower frame the word stands on the frame's top line.
    readonly property real stackDepth: stackLabel.implicitHeight + stackSeat.depthAt(actionBtn.stackStep)
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
    /// The icon is a mark beside the word: drawn to its letters (`ActionButtonSeat.wordSize`, デザイン規約 §アイコンの席
    /// の文頭記号) in an `iconSm` box rather than `iconMd`, and the seat shrinks to the icon — the wide seat holds the
    /// icon and the hold mark side by side, a pairing a click-only button cannot have (デザイン規約 §回数押し). The
    /// toolbar keeps the wide seat: its buttons can pair.
    property bool besideWord: false
    /// The wording the label's box is measured for, and whether it is a command (the two families measure
    /// differently), so a button whose wording changes with its state does not move its neighbours.
    property string widestText: ""
    property bool widestCode: false
    /// A cell the operation panel lays out (`TopBar`), which can be narrower than its word. False, the default, is a
    /// button measured to its own content — every one outside the panel.
    property bool laidByBand: false
    /// Told from outside that the operation panel has reached the width where the words go, whether or not this cell
    /// is short of room yet (`TopBar.actionsFolded` / `findFolded`). The button also gives up on its own when the cell
    /// it was handed cannot hold its word whole.
    property bool foldRequested: false
    /// The room the word gets in this cell once the marks and the air either side are paid for. Read off the style's
    /// fixed `padding`: `leftPadding` / `rightPadding` are answered from here (through `slack`), and reading them back
    /// would close a loop.
    readonly property real wordRoom:
        !actionBtn.laidByBand ? btnLabel.box
        : Math.max(0, actionBtn.width - 2 * actionBtn.padding - seat.implicitWidth - btnRow.spacing)
    /// Down to the icon: the word no longer fits whole — it is never cut to a `…` (規約 §操作パネル) — or the panel
    /// has said so.
    readonly property bool folded: actionBtn.laidByBand
        && (actionBtn.foldRequested || actionBtn.wordRoom < btnLabel.wantWidth)
    /// The widest this button is ever drawn, and the narrowest it is drawn with its word on it. Neither moves with
    /// whether the word is shown — the row lays the cell out from these, so giving the word up cannot change the width
    /// that decides whether to give it up (`TopBar`).
    readonly property real naturalWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing + btnLabel.box
    /// The wording this state says, whole, with the air around it. The set folds on the widest of the three
    /// (`TopBar.actionFold`).
    readonly property real foldWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing + btnLabel.wantWidth
    /// Automation: the ink the word takes (`PGG_AUTO_ACT=band-actions`).
    readonly property real wordInk: btnLabel.inkWidth
    /// …and the width this state's wording wants, whole: what the set folds against (`foldWidth`).
    readonly property real wordWant: btnLabel.wantWidth
    /// Icon and word centred as a pair, for a button told to fill a pane's width — packed from the left, the word
    /// would sit adrift from its icon.
    property bool centred: false
    /// What the shared box (the longest wording of the set) hands this state past its own wording. Split between the
    /// two ends with the phrase moving whole: all of it behind the word stretches the wash, the frame and the fill past
    /// the last letter, and centring the word alone would open the icon-to-word step (デザイン規約 §余白).
    ///
    /// A cell narrower than the box has no slack, and one narrower than its word folds (`folded`). Measured against the
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
    /// size (規約 §寸法 の `buttonMinWidth`).
    readonly property real floorSlack:
        actionBtn.framed && !btnLabel.phrased && !actionBtn.folded
        ? Theme.buttonMinWidth - btnRow.implicitWidth - 2 * actionBtn.padding : 0
    /// The air each end already holds (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」): a lone mark
    /// centred in the two-mark seat, a chip reaching half a gap past its last letter. Taken off before the halves are
    /// cut, so the ink comes out even.
    readonly property real headInk: seat.visible ? (seat.implicitWidth - seat.step) / 2 : 0
    readonly property real tailInk: actionBtn.code ? Theme.spaceXs / 2 : 0
    /// The last press landed, raised at once: the answer is decided here, and the work starts under the full fill.
    signal held()
    /// The floor after it has passed (`HoldDriver.landed`). For an owner whose answer takes this button off the
    /// screen at once — the row leaving its list, the pane closing: it accepted the answer at `held`, its content
    /// taken and its target locked there, and gives it now, so the full fill is seen first (デザイン規約 §回数押し).
    /// Nothing is decided here, and nothing this button turned into meanwhile (busy, disabled) calls the answer off.
    signal landed()
    /// The ordinary action: a plain press let go. A confirmed button raises it on no release.
    signal activated()
    /// Over the fill the words lift to `textOnAccent`, the wait included: the full fill stays while the work runs.
    readonly property color fg: holdProgress > 0 ? Theme.textOnAccent
                                : actionBtn.busy ? Theme.textMuted : enabled ? tone : Theme.textMuted
    /// The marks' colour: while busy, the state a step down like the ring and the frame
    /// (デザイン規約 §進行中・回数押しの定数「リング・枠・印は、そのボタンが待つ前に着ていた色の 1 段下」) — over the fill, the
    /// words' lift. Apart from `fg` because a dimmed word has one colour only (§無効), while a mark keeps its hue so a
    /// wait says which button waits.
    readonly property color markFg: holdProgress > 0 ? Theme.textOnAccent
                                    : actionBtn.busy ? actionBtn.toneDim : actionBtn.fg
    /// Off (not busy — a wait has its own step down), the hold mark keeps the colour it wears when the button can be
    /// pressed ([`holdHue`]), a step down, while the words and an icon beside them grey: the grey says it cannot be
    /// pressed, the mark what pressing it would cost (`AppMenuItem.holdOff`). An asleep heading's grey mark stays grey
    /// (`HoldIcon.ink`).
    readonly property bool holdOff: !actionBtn.enabled && !actionBtn.busy && actionBtn.holdProgress <= 0
    /// Neither over the fill nor waiting: the hold mark wears [`holdHue`], which [`holdOff`] dims (`HoldIcon.off`).
    readonly property bool holdResting: actionBtn.holdProgress <= 0 && !actionBtn.busy
    /// The hold mark's colour in a seat: at rest [`holdHue`], else the marks'.
    readonly property color holdInk: actionBtn.holdResting ? actionBtn.holdHue : actionBtn.markFg
    /// A press lands only with git off the network for this button.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: where the word's baseline stands, in `item`'s y. Buttons side by side are read as one line of words,
    /// and two of different builds (one with the hold mark) are different depths (`DiffRowDelegate`'s hunk tools).
    function wordBase(item) {
        return (actionBtn.stacked ? stackLabel : btnLabel).wordBase(item)
    }

    /// Automation: every press at once without a hand, gated the way a hand's press is (`onDownChanged`). Answers
    /// whether it went in, so a run that met the button busy presses again later rather than counting presses nothing
    /// can land.
    function completeHold() {
        if (!actionBtn.live)
            return false
        holdDrive.runOut()
        return true
    }

    // Only confirmed buttons join the tab order (デザイン規約 §回数押し).
    activeFocusOnTab: actionBtn.holdPresses > 0
    Accessible.description: actionBtn.holdPresses > 0 ? Words.pressToActivate(actionBtn.holdPresses) : ""
    // The hand left part way through the presses: back to zero (デザイン規約 §回数押し).
    onHoveredChanged: if (!actionBtn.hovered) holdDrive.leave()

    /// Whether the press just let go was a plain one, judged before the latch opens — the click follows the release in
    /// the same delivery, when the count is live again. `clickJudged` says there was a press at all: `clicked()` is
    /// also raised with no press behind it (`TopBar.stashNow` / `fetchNow`, `TabStrip`, `WipBucketHeader`), and that
    /// one is answered from the live count.
    property bool clickWanted: false
    property bool clickJudged: false
    onClicked: {
        if (!actionBtn.live)
            return
        const plain = actionBtn.clickJudged ? actionBtn.clickWanted : actionBtn.holdPresses <= 0
        actionBtn.clickJudged = false
        if (plain)
            actionBtn.activated()
    }
    /// Gone deaf under the hand (git went onto the network, or the state it acts on went): the count blanks and no
    /// signal is raised. The press that sent the work keeps its full fill (`HoldDriver.blank`).
    onLiveChanged: if (!actionBtn.live) {
        // Both, so a following press-less `clicked()` is judged from the live count.
        actionBtn.clickWanted = false
        actionBtn.clickJudged = false
        holdDrive.blank()
    }
    onDownChanged: {
        // Only the press is gated: the release has nothing left to start.
        if (actionBtn.down && !actionBtn.live)
            return
        if (actionBtn.down) {
            // Whatever the count: a plain press is answered from the latch too, and a press that never closed it can
            // still change meaning under the hand.
            actionBtn.clickJudged = false
            holdDrive.begin()
            return
        }
        // A confirmed button reports no click, and a press whose premise went reports nothing (`HoldDriver.stale` —
        // デザイン規約 §回数押し).
        actionBtn.clickWanted = holdDrive.armedPresses <= 0 && !holdDrive.stale
        actionBtn.clickJudged = true
        holdDrive.letUp()
    }
    // The press's other hand (`HoldDriver.pressKey`), offered only while `live`: a button waiting on git answers no
    // key either.
    Keys.onPressed: event => {
        if (actionBtn.live)
            holdDrive.pressKey(event)
    }
    Keys.onReleased: event => {
        if (actionBtn.live)
            holdDrive.releaseKey(event)
    }
    // Keys counted elsewhere are another gesture: the focus moved away drops them (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!actionBtn.activeFocus) holdDrive.focusLost()
    // The work goes out at the last press; the full fill waits on it past the floor, and the ring takes over from
    // there. An answer that takes the button away is given at the floor's end (`landed`).
    HoldDriver {
        id: holdDrive
        holdPresses: actionBtn.holdPresses
        waiting: actionBtn.busy
        onFinished: actionBtn.held()
        onLanded: actionBtn.landed()
    }
    /// The frame's height once folded: the depth the button had with its word, handed down by whoever sets that depth.
    /// Folding gives up the word, not the depth (デザイン規約 §操作パネル).
    property real foldedDepth: 0
    /// How far in from the cell's top and bottom the frame is drawn. A folded cell fills the band's height like the ☰
    /// and the window's three, and the frame keeps `foldedDepth` — a line along the band's top edge reads as a box
    /// glued to the window. The wash still fills the whole cell: that is the target.
    readonly property real frameInset:
        actionBtn.folded ? Math.max(0, (actionBtn.height - actionBtn.foldedDepth) / 2) : 0
    /// Half a gap inside the frame's top and right lines, in the button's coordinates: as far as a `!` on a word's
    /// shoulder may reach (`ActionButtonLabel.alertCeiling`).
    readonly property real alertTopEdge: actionBtn.frameInset + Theme.borderWidth + Theme.spaceXs / 2
    readonly property real alertRightEdge: actionBtn.width - Theme.borderWidth - Theme.spaceXs / 2
    /// The fill inside the frame, for a button that has to read as a box laid on its ground (the operation panel's).
    /// Under the wash, so the hand still lights it.
    property color faceColor: "transparent"

    background: Rectangle {
        id: btnGround
        color: "transparent"
        radius: Theme.radiusSm
        // The frame and the presses' fill: its own item, so a cell taller than the box washes edge to edge with the
        // frame still round the box (`frameInset`).
        Rectangle {
            anchors.fill: parent
            anchors.topMargin: actionBtn.frameInset
            anchors.bottomMargin: actionBtn.frameInset
            color: actionBtn.faceColor
            // While busy, or once the last press has filled it, the frame goes a step down rather than to grey (grey
            // would take its warning back for the whole wait), and a button with no frame grows none
            // (デザイン規約 §進行中・回数押しの定数).
            border.color: (actionBtn.busy || holdDrive.dimmed) && actionBtn.framed ? actionBtn.toneDim
                                                                                      : actionBtn.frameColor
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            HoldFill {
                progress: actionBtn.holdProgress
                tone: actionBtn.holdTone
                dimmed: holdDrive.dimmed
                inset: actionBtn.framed ? Theme.borderWidth : 0
            }
        }
        // `HoverToolButton`'s wash (`washColor`), redrawn because a background handed in replaces the one that carried
        // it. Over the face (an opaque face would hide it) and under the focus ring, across the whole cell so the
        // target matches its neighbours'. Only while `live` (デザイン規約 §進行中・回数押しの定数).
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusSm
            color: actionBtn.live ? actionBtn.washColor : "transparent"
        }
        // Outside the frame: the frame's colour is saying this button is the dangerous one (デザイン規約 §回数押し).
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
        when: actionBtn.laidByBand
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
                // A phrase carries the hold's mark itself (`ActionButtonLabel.phraseHoldPresses`): this seat sits at
                // the row's left edge, apart from the centred phrase.
                holdPresses: btnLabel.phrased ? 0 : actionBtn.armedPresses
                besideWord: actionBtn.besideWord
                holdProgress: actionBtn.holdProgress
                busy: actionBtn.ringing
                tint: actionBtn.markFg
                holdTint: actionBtn.holdInk
                holdOff: actionBtn.holdOff
                // Folded, the `!` comes to the mark's corner: the warning survives the fold
                // (規約 §回数押し「警告の色は枠と印が持つ」).
                cornerAlert: actionBtn.folded && actionBtn.alert
                cornerAlertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                // Beside its word, the head of it (デザイン規約 §アイコンの席 の文頭記号); folded, it stands alone.
                wordSize: actionBtn.folded || btnLabel.phrased || btnLabel.head === "" ? 0 : btnLabel.fontSize
                wordBase: btnLabel.y + btnLabel.headBase - seat.y
                wordHang: btnLabel.code ? Theme.spaceXs / 2 : 0
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
                folded: actionBtn.folded
                tint: actionBtn.fg
                alert: actionBtn.alert
                // A step down while busy, like every mark (デザイン規約 §進行中・回数押しの定数).
                alertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                alertCeiling: actionBtn.alertTopEdge - actionBtn.topPadding - btnRow.y - btnLabel.y
                alertWall: actionBtn.alertRightEdge - actionBtn.leftPadding - btnRow.x - btnLabel.x
                fontSize: actionBtn.font.pixelSize
                phraseHead: actionBtn.phraseHead
                phraseHoldPresses: actionBtn.armedPresses
                phraseHoldProgress: actionBtn.holdProgress
                phraseHoldTint: actionBtn.holdResting ? actionBtn.holdHue : actionBtn.fg
                phraseHoldOff: actionBtn.holdOff
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
                folded: actionBtn.folded
                tint: actionBtn.fg
                alert: actionBtn.alert
                alertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                alertCeiling: actionBtn.alertTopEdge - actionBtn.topPadding - stackCol.y - stackLabel.y
                alertWall: actionBtn.alertRightEdge - actionBtn.leftPadding - stackCol.x - stackLabel.x
                // A step under the band's: two lines in a two-line row, and the word is the half a reader already
                // knows.
                fontSize: Theme.fontSm
            }
            ActionButtonSeat {
                id: stackSeat
                // Folded, on the button's middle: held to the top it would sit in the upper half of a two-line box.
                Layout.alignment: actionBtn.folded ? Qt.AlignHCenter | Qt.AlignVCenter
                                                   : Qt.AlignHCenter | Qt.AlignTop
                kind: actionBtn.kind
                holdPresses: actionBtn.armedPresses
                // Folded, the mark grows into the word's room.
                step: actionBtn.folded ? Theme.iconXl : actionBtn.stackStep
                besideWord: false
                holdProgress: actionBtn.holdProgress
                busy: actionBtn.ringing
                tint: actionBtn.markFg
                holdTint: actionBtn.holdInk
                holdOff: actionBtn.holdOff
                cornerAlert: actionBtn.folded && actionBtn.alert
                cornerAlertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
            }
        }
    }
}
