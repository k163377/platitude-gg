import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the word to be sure of it. Both halves dim together
// when disabled.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    /// The state's colour taken a step down for the wait — the ring and the frame wear it while git is on the network
    /// (デザイン規約 §暗く落とした段).
    ///
    /// The state's: where the word stays plain and only the frame and the mark carry a warning, this
    /// follows the frame. A plain button has no darker step of its own and takes `textMuted`.
    property color toneDim: Theme.textMuted
    /// git is on the network for this button: the ring takes the icon's seat, the word stays and steps down to the
    /// disabled colour, and the whole of it goes as dim and as deaf as a disabled one (デザイン規約 §進行中・長押しの定数).
    ///
    /// The word stays: what a button is waiting on is the thing the button names, and a
    /// ring alone in a bare button leaves nothing on the band to say which action is out. The seat is where the ring
    /// goes because it is the one place already measured for a mark of exactly this size, so the button's width does
    /// not move.
    ///
    /// Dimmed, still enabled: `enabled: false` would take the focus away, and the press that started the
    /// network call is the very one that may have come from the keyboard.
    property bool busy: false
    /// How long this button has to be held to fire `held()`; zero for an ordinary button, where a click is the whole
    /// gesture (デザイン規約 §進行中・長押しの定数).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// **The length the press under way was given** (`HoldDriver.armedMs`), which is the live one while no press is
    /// under way. An owner whose wording, frame or tone is worked out from the same condition the length is reads this,
    /// so the button cannot change what it says under a hand that is already holding
    /// (デザイン規約 §長押し「長押しか否かは押した瞬間に確定」).
    readonly property alias armedMs: holdDrive.armedMs
    /// **A gesture is under way**, the fill's slide back included (`HoldDriver.gesturing`) — what an owner latches on
    /// when what it dresses itself from is an answer other than the length.
    readonly property alias gesturing: holdDrive.gesturing
    /// **What this button's press is aimed at** (`HoldDriver.premise`) — set it wherever the target can be swapped
    /// out from under the hand without the length moving: a band button re-pointed at another tab, a list delegate
    /// re-used for another row.
    property alias premise: holdDrive.premise
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at what this button does did not work. Drawn as a mark standing clear of the word's last letter, in
    /// the word's own colour.
    property bool alert: false
    /// The mark's colour, where it is not the word's.
    property color alertTone: actionBtn.fg
    /// Pull the mark back to a letter's distance from the word.
    ///
    /// The mark sits at the end of the label's **advance** width, which leaves the last glyph's right side bearing as
    /// clear air — a word ending in `)` carries a lot of it, so the mark reads as a separate thing. One gap back puts
    /// it where the letters are.
    property bool alertTight: false
    /// The colour the hold fills the button with — the frame's, since a framed button fills the frame it drew. A bare
    /// one names its own (the hunk heading's `Discard hunk`, which fills edge to edge the way a held menu row does —
    /// デザイン規約 §長押し).
    property color holdTone: actionBtn.frameColor
    readonly property bool framed: actionBtn.frameColor.a > 0
    /// The label is a git command said in git's own spelling, and wears the chip that says so — lowercase, mono, on a
    /// faint ground (デザイン規約 §git 用語のコード表記). The command is the whole label, so `text` itself is what the chip is drawn
    /// around — and that text is left untranslated: it is the command itself.
    property bool code: false
    /// **The word over the mark rather than beside it**. The operation panel's own shape:
    /// that row is two lines deep, and a button that set its mark beside its word would be the one thing in it
    /// written on one. **The height does not move with it** — what a stacked button gives up as it narrows is its
    /// word, and then the mark grows into the room the word was in (`foldGrow`), so the row's depth is the same
    /// whatever any button in it is saying.
    property bool stacked: false
    /// What the wording is set at (`ActionButtonLabel.wordWeight`). Normal by default, which is every button whose
    /// word is a command on a chip.
    property int wordWeight: Font.Normal
    /// The label is a phrase with a command at each end — a chip, `text` between them, and a second chip in a colour
    /// of its own (`ActionButtonLabel.phraseHead`). Empty is the ordinary single-wording button.
    property string phraseHead: ""
    property string phraseCount: ""
    property string phraseTail: ""
    property color phraseTailTint: actionBtn.fg
    /// A short warning said after the phrase's words, in the note's own amber — the tag a menu row wears for the same
    /// thing, brought inside the phrase (`ActionButtonLabel.phraseNote`).
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
    /// The icon is a mark standing next to the word.
    ///
    /// The step goes with the role (デザイン規約 §寸法「段は役割で選ぶ」 — `iconSm` is the mark beside a word, `iconMd` the icon a row
    /// or a band starts with), and the seat comes in to the icon itself: the wider seat is there to hold the icon and
    /// the hold mark side by side, and a button that is only ever clicked pays for a pairing it cannot have (デザイン規約
    /// §余白「印が自分で持っている余白は、隣の 詰めに数える」).
    ///
    /// The toolbar keeps the wide seat: its buttons can pair, and their width is measured into a box two of them share.
    property bool besideWord: false
    /// Text the label's box is measured for, and whether that wording is a command (the two families measure
    /// differently, so the box has to be told which one it is holding). A button whose wording changes with its state
    /// would otherwise move everything beside it in the toolbar every time the state changed.
    property string widestText: ""
    property bool widestCode: false
    /// The narrowest this button draws its word before giving it up for the icon alone (`BandWidest.wordFloor`).
    ///
    /// **Zero — the default — is the whole of the old behaviour**: a button measured to its own content, which is what
    /// every ActionButton outside the band is. Only a control the band lays out (`TopBar`) hands the row a cell that
    /// can be narrower than the word inside it, and only that one has a shape to fall back to (規約 §ウィンドウの縁).
    property real wordFloor: 0
    /// Told from outside that the band has come to the width where the words go, whether or not this cell is short of
    /// room yet (`TopBar.actionsFolded`). The button also gives up on its own when the cell it was handed cannot hold
    /// the floor, which is what happens when the tabs take the room before the window is done shrinking.
    property bool foldRequested: false
    /// The room the word is given inside this cell, once the marks and the air either side are paid for.
    ///
    /// Read off the control's own `padding`: `leftPadding` / `rightPadding` are answered from here (through
    /// `slack`), and reading them back would close the ring. `padding` is the style's own and never
    /// moves, so this is a question about the width the row handed over and nothing else.
    readonly property real wordRoom:
        actionBtn.wordFloor <= 0 ? btnLabel.box
        : Math.max(0, actionBtn.width - 2 * actionBtn.padding - seat.implicitWidth - btnRow.spacing)
    /// Down to the icon: nothing left to say the word in, or the band has said so.
    readonly property bool folded: actionBtn.wordFloor > 0
        && (actionBtn.foldRequested || actionBtn.wordRoom < actionBtn.wordFloor)
    /// The widest this button is ever drawn, and the narrowest it is drawn with a word still on it. **Neither moves
    /// with the shape the button is in** — the row lays the cell out from these, so giving the word up cannot
    /// change the width that decides whether to give it up (`TopBar`).
    readonly property real naturalWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing + btnLabel.box
    ///
    /// **The flag is counted into the floor**, because it is the part that never gives: without it the last step
    /// before the fold hands `push -f` a `…` and one letter, while its neighbours still have three
    /// (`ActionButtonLabel.flagRoom`). Which is also why the set reads the **widest** of the three floors
    /// (`TopBar.actionFoldW`) — a floor that only fits the shortest wording is not a floor for the set.
    readonly property real foldWidth:
        2 * actionBtn.padding + seat.implicitWidth + btnRow.spacing
        + actionBtn.wordFloor + btnLabel.flagRoom
    /// Automation: the word as it came out — the ink left after a cut, and whether there was one. A picture cannot be
    /// asked whether a wording ended in a `…` of its own or was elided into one (`PGG_AUTO_ACT=band-actions`).
    readonly property real wordInk: btnLabel.inkWidth
    readonly property bool wordCut: btnLabel.capped
    /// …what this state's wording would like to be, and the box the set was measured for — which a run aiming between
    /// the two shapes needs, because the cell that starts cutting a wording is that wording plus everything the cell
    /// holds around it (`naturalWidth - wordBox`).
    readonly property real wordWant: btnLabel.wantWidth
    readonly property real wordBox: btnLabel.box
    /// Icon and word centred as a pair. A toolbar button is measured to its content and never sees the difference;
    /// one told to fill a pane's width does — packed from the left the icon would sit against the far edge
    /// with the word adrift from it.
    property bool centred: false
    /// What the shared box hands this state past its own wording: the box is measured for the longest thing either of
    /// the pair ever says, so every shorter wording is given air it has no use for.
    ///
    /// Split between the button's two ends — packed from the left it all lands behind
    /// the word, and the wash, the warning frame and the hold's fill all reach well past the last letter.
    ///
    /// The phrase moves whole, so the step from the icon to the word is the same in every state (デザイン規約 §余白). Centring
    /// the word inside the box instead would open that step wider than the gap to the button beside it.
    ///
    /// **A cell narrower than the box has no slack at all** — what it has is a word too long for it, and the word
    /// elides into what there is (`ActionButtonLabel.cap`). Measured against the *wanted* width, for the same
    /// reason the box is measured off a hidden label: the painted width is the answer this
    /// arithmetic produces.
    readonly property real slack:
        actionBtn.folded ? 0
        : Math.max(0, Math.min(btnLabel.box, actionBtn.wordRoom) - btnLabel.wantWidth, actionBtn.floorSlack)
    /// What a framed button is short of `Theme.buttonMinWidth`, which it takes as slack like any other (so the phrase
    /// still moves whole and the ink still comes out even at the two ends).
    ///
    /// A frame is a box put round a word, and a short word draws a box the eye takes for a chip: nothing
    /// to press.
    ///
    /// Only where a frame is drawn. A bare button is a word among words — the hunk heading's two, a dialog's `Cancel` —
    /// and a floor there would stretch the hover wash and the hold's fill well past what the button names.
    ///
    /// **A phrase has no floor.** The floor exists so a short word's box still reads as a button, and a
    /// button told to fill a pane is already far past it. It cannot be measured here either: a phrased cell asks for
    /// no width at all (`ActionButtonLabel.phraseRoom`), so the row reads as a tiny button and the floor would charge
    /// the padding a whole `buttonMinWidth` — straight out of the room the phrase then has to elide itself into.
    ///
    /// **Nor does a button that has given its word up.** That floor is a box put round a *word*; a box put round a
    /// mark takes the size of the mark, which is the same ruling the state group's `…` is drawn under (規約 §ウィンドウの縁
    /// 「枠を着ていても `buttonMinWidth` の外」).
    readonly property real floorSlack:
        actionBtn.framed && !btnLabel.phrased && !actionBtn.folded
        ? Theme.buttonMinWidth - btnRow.implicitWidth - 2 * actionBtn.padding : 0
    /// The air each end is already holding before the slack is shared out (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」): a lone
    /// mark sits centred in the two-mark seat, and a command's chip reaches half a gap past its last letter. Taken off
    /// before the halves are cut, so what comes out even is the **ink** at the two
    /// ends.
    readonly property real headInk: seat.visible ? (seat.implicitWidth - seat.step) / 2 : 0
    readonly property real tailInk: actionBtn.code ? Theme.spaceXs / 2 : 0
    /// Held all the way down.
    signal held()
    /// Pressed and let go, meaning the button's ordinary action.
    ///
    /// A plain press alone raises it: on a hold button both releases — the one that completes the hold and the one
    /// that gives up on it part way — stop there.
    signal activated()
    readonly property color fg: actionBtn.busy ? Theme.textMuted
                                : holdProgress > 0 ? Theme.textOnAccent : enabled ? tone : Theme.textMuted
    /// What the marks on this button wear while the word wears the disabled step: the state a step down, the same one
    /// the ring and the frame take (デザイン規約 §進行中・長押しの定数「リングと枠は 1 段下」).
    ///
    /// A separate colour from `fg` because the two paths mean different things: dimming a **word** is the disabled
    /// signal and has one colour only (§無効 — `textMuted`), while a mark keeps its hue and drops a step (§暗く落とした段).
    /// Read off the state, so a wait says which button is waiting.
    readonly property color markFg: actionBtn.busy ? actionBtn.toneDim : actionBtn.fg
    /// A press lands only with git off the network for this button.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }

    // Tab reaches the buttons that need a second way in. The rest of the toolbar stays out of the tab order: a hold is
    // the only gesture here that a pointer alone can fail to make (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0
    Accessible.description: actionBtn.holdMs > 0 ? Words.holdToActivate : ""

    /// Whether the press that has just been let go was a plain one, worked out **before** the latch opens: the click
    /// arrives in the same delivery as the release, and by then the length is back to whatever is true now.
    ///
    /// `clickJudged` says whether there was a press to judge at all. **A `clicked()` can be raised with no press
    /// behind it** — the band's own doors do exactly that so a run presses what a hand presses
    /// (`TopBar.stashNow` / `fetchNow`, `TabStrip`, `WipBucketHeader`) — and one of those is answered from the live
    /// length, there being no gesture for a latch to be about.
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
    /// A button that goes deaf under the hand — git took it onto the network, or the state it acts on went — has
    /// nothing left for the gesture to land on. The fill blanks, and no signal is raised:
    /// the press was made on a button that is no longer there to press.
    onLiveChanged: if (!actionBtn.live) {
        // Both, so the next `clicked()` — which may be one raised with no press behind it — is judged from what is
        // true then.
        actionBtn.clickWanted = false
        actionBtn.clickJudged = false
        holdDrive.blank()
    }
    onDownChanged: {
        // **Only the press is gated.** A release left unanswered leaves the fill running, and it runs
        // on to fire a hold under a hand that has already let go (and the latch below never opens again).
        if (actionBtn.down && !actionBtn.live)
            return
        if (actionBtn.down) {
            // Unconditionally, whatever the length is: the latch is what a plain press is answered from too, and a
            // press that never closed it is one whose meaning can still change under the hand.
            actionBtn.clickJudged = false
            holdDrive.begin()
            return
        }
        // A hold reports no click at all, and a press whose premise went reports nothing either
        // (`HoldDriver.stale` — デザイン規約 §長押し).
        actionBtn.clickWanted = holdDrive.armedMs <= 0 && !holdDrive.stale
        actionBtn.clickJudged = true
        holdDrive.letUp()
    }
    // The hold's other hand (`HoldDriver.pressKey`), offered only while the button is one a press can reach at all: a
    // button waiting on git answers no key any more than it answers a click.
    Keys.onPressed: event => {
        if (actionBtn.live)
            holdDrive.pressKey(event)
    }
    Keys.onReleased: event => {
        if (actionBtn.live)
            holdDrive.releaseKey(event)
    }
    // **A key press is answered wherever the focus is when the key comes up.** Move it and the release lands on
    // something else, leaving this fill to run out and fire a hold nobody was still making
    // (`HoldDriver.focusLost`).
    onActiveFocusChanged: if (!actionBtn.activeFocus) holdDrive.focusLost()
    HoldDriver {
        id: holdDrive
        holdMs: actionBtn.holdMs
        onFinished: actionBtn.held()
    }
    /// How far in from the cell's two ends the frame is drawn. Nothing at all for a button measured to its own
    /// content, where the cell **is** the box; in a folded cell, which fills the band's height the way the ☰ and the
    /// window's three do, the frame keeps the height the button's own box has — **a line drawn along the band's top
    /// edge reads as a box glued to the window** (measured). The wash still fills
    /// the whole cell, because that is the target, and the target is what the pointer is answering.
    ///
    /// Read only where the two can differ. A button measured to its own content **is** its implicit height, and a
    /// binding that reads both closes a ring the engine can see from the outside even where the arithmetic cannot
    /// (measured, `OpExitCard` / `DiffPaneHeader` reported a loop on this property).
    readonly property real frameInset:
        actionBtn.folded ? Math.max(0, (actionBtn.height - actionBtn.implicitHeight) / 2) : 0
    /// What the inside of the frame is filled with. Transparent everywhere the button is a word on the ground it
    /// stands on; a colour where the button has to read as a box laid on that ground instead — which is what a button
    /// on the operated row is. **Under the wash, not over it**: the hand still lights the box.
    property color faceColor: "transparent"

    background: Rectangle {
        id: btnGround
        color: "transparent"
        radius: Theme.radiusSm
        // The frame, and the fill a hold puts inside it. Its own item, so that a cell
        // taller than the button's box can wash edge to edge and still draw the frame round the box (`frameInset`).
        Rectangle {
            anchors.fill: parent
            anchors.topMargin: actionBtn.frameInset
            anchors.bottomMargin: actionBtn.frameInset
            color: actionBtn.faceColor
            // The frame goes a step down with the rest of the button while git is out on the network: the button is
            // still the one that overwrites a remote, and a frame that dropped to grey would take that back for as
            // long as the wait lasted.
            //
            // A button with no frame of its own grows none: the ring already says the wait has started, and a frame
            // drawn around a button that has never worn one reads as a box laid over the band.
            border.color: actionBtn.busy && actionBtn.framed ? actionBtn.toneDim : actionBtn.frameColor
            border.width: Theme.borderWidth
            radius: Theme.radiusSm
            HoldFill {
                progress: actionBtn.holdProgress
                tone: actionBtn.holdTone
                inset: actionBtn.framed ? Theme.borderWidth : 0
            }
        }
        // The same wash every other tool button answers with (`HoverToolButton.washColor`), drawn rather than left
        // out: a background handed in replaces the one that carries it. **Over the face and under the focus ring** —
        // the face is opaque where a button has one, and a wash beneath it would never be seen; and it covers the
        // whole cell rather than the frame's box, so the target is the same size as the cells beside it.
        //
        // Only while it is answering: a button with git out on the network is as deaf as a disabled one (`live`), and
        // the hand that started the fetch is still resting on it — the wash must not stay up for the whole call.
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusSm
            color: actionBtn.live ? actionBtn.washColor : "transparent"
        }
        // Drawn outside the frame rather than in it: the frame's colour is already saying this button is the dangerous
        // one, and focus must not be able to take that over.
        //
        // `visualFocus`, which is focus that arrived from the keyboard (a press gives `activeFocus` as well, and
        // `focusPolicy` is StrongFocus and nothing on this band takes it back, so an `activeFocus` ring would
        // come up on a click and stay).
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
    // The toolbar's size unless an instance says otherwise — the hunk heading's words take fontSm there, a step under
    // the file's own word (デザイン規約 §diff の中のステージ).
    font.pixelSize: Theme.fontMd

    // The slack goes into the button's own air, so what one end takes the other gives: the width stays the one the pair
    // was measured for and the toolbar's right-hand end does not move (デザイン規約 §リモートへ送る — 全状態が同じ幅).
    //
    // The trailing side is what is left: a wording measures in fractions of a pixel,
    // and `floor` beside `ceil` would round the pair of them up to a button a pixel wider than its box. The odd pixel
    // also lands on the side with a mark to stand clear of (`alert`). Only a button with slack to share reads the ink
    // off its two ends — one measured into a shared box (`widestText`) or one held open by the frame's floor
    // (`floorSlack`). A button sized to its own content is already as tight as the two ends can be, and taking the ink
    // off its padding would move every button in the app for the sake of the few that have room to give.
    readonly property real headAir:
        actionBtn.folded ? 0
        : (btnLabel.box > 0 || actionBtn.floorSlack > 0)
          ? Math.floor((actionBtn.slack - actionBtn.headInk - actionBtn.tailInk) / 2)
          : 0
    // A folded button pays no padding of its own: the cell it stands in is the band's own end-cell width and the mark
    // is centred in it, the way the ☰'s is and the window's three are (規約 §ウィンドウの縁「帯の両端は同じ組み方」).
    leftPadding: actionBtn.folded ? 0 : actionBtn.padding + actionBtn.headAir
    rightPadding: actionBtn.folded ? 0 : actionBtn.padding + (actionBtn.slack - actionBtn.headAir)
    // **Nothing over or under the pair**: the two lines are what the button
    // is, and the row around them already keeps its own step over and under (`opsBarHeight`). A button written on
    // one line keeps the style's own padding, which is what holds its word off the frame.
    topPadding: actionBtn.stacked ? 0 : actionBtn.padding
    bottomPadding: actionBtn.stacked ? 0 : actionBtn.padding

    /// **What a button laid out by the band asks for does not move with the shape it is in** — the same rule
    /// `naturalWidth` and `foldWidth` are written under, said on the one number the style answers for us.
    ///
    /// A Control's own `implicitWidth` is its content plus the two paddings, and both of those are answered from the
    /// width the row handed over (`wordRoom` → `folded` → `slack`). Until the row has set a width of its own — the
    /// frames on the way up, before the first rearrange — `setImplicitWidth` writes one, so the shape the button chose
    /// decides the cell that was supposed to decide the shape (measured, `folded` and `wordRoom` both reported a
    /// loop, on the two band buttons that wear no frame).
    ///
    /// Only where the row lays the cell out. A button measured to its own content **is** its content, and asking for a
    /// box it was never measured for would leave every wording adrift in it.
    Binding {
        target: actionBtn
        property: "implicitWidth"
        value: actionBtn.naturalWidth
        when: actionBtn.wordFloor > 0
    }

    // The row keeps its size while the network call runs — a pointer is still resting on the button — so nothing here
    // leaves the layout: the word stays where it is and only its colour steps
    // down, and the ring turns inside the seat the icon was already measured into.
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
                // A phrased label fills the row on its own and centres its phrase inside itself. A spacer either side
                // would split the slack three ways instead, leaving the phrase off the button's centre by an amount
                // its own parts change (the `+N` alone moves it).
                //
                // A folded button centres the same way: with no word beside it the mark is the whole content, and a
                // mark packed against the left of a cell would not line up with the ones either side of it.
                //
                visible: (actionBtn.centred || actionBtn.folded) && !btnLabel.phrased
            }
            ActionButtonSeat {
                id: seat
                kind: actionBtn.kind
                // A phrase carries the hold's mark itself, ahead of its first chip: this seat is at the row's left
                // edge and the phrase is centred, so the two would stand apart (`ActionButtonLabel.phraseHoldMs`).
                holdMs: btnLabel.phrased ? 0 : actionBtn.armedMs
                besideWord: actionBtn.besideWord
                holdProgress: actionBtn.holdProgress
                busy: actionBtn.busy
                tint: actionBtn.markFg
                // With the word gone the `!` has nowhere to stand after it, so it comes to the mark's own corner —
                // the warning has to survive the fold, being the one thing on this button that is news (規約 §長押し:
                // 警告の色は枠と印が持つ).
                cornerAlert: actionBtn.folded && actionBtn.alert
                cornerAlertTone: actionBtn.busy ? actionBtn.toneDim : actionBtn.alertTone
                Layout.alignment: Qt.AlignVCenter
            }
            ActionButtonLabel {
                id: btnLabel
                // **Out of the row once the word is given up.** A zero-width child still takes
                // the row's `spacing` on both sides of itself, and the two spacers either end share out what is
                // left — so the mark came to rest half a gap left of its cell's middle while the gap the word was
                // not in hung to the right of it (measured: 2px in a 36px cell, on all three of the band's
                // buttons; centred to the pixel on all three after). Everything read off this label is read off its
                // bindings, which an item out of the layout goes on answering.
                //
                // **This cannot unfold the button**: `wordRoom` is what decides the fold, and it is arithmetic on
                // the cell's width, the padding and the seat alone. A fold
                // answered by what the row came out at would give the word its room back by hiding it.
                visible: !actionBtn.folded
                text: actionBtn.text
                code: actionBtn.code
                wordWeight: actionBtn.wordWeight
                widestText: actionBtn.widestText
                widestCode: actionBtn.widestCode
                // What the cell left the word, and the two ends of the answer to that: -1 is "as much as it wants",
                // which is every button the band does not lay out.
                cap: actionBtn.wordFloor > 0 ? actionBtn.wordRoom : -1
                folded: actionBtn.folded
                tint: actionBtn.fg
                alert: actionBtn.alert
                // The mark goes a step down with the frame while the wait lasts, like every other mark on the button:
                // left at its own colour it would be the brightest thing on one that has just been made untouchable.
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

        // **The panel's own shape: the word over the mark**. Its own pair rather than the
        // row above stood on its side — a `RowLayout` cannot be turned, and the layout that can was measured and
        // took the window's floor down with it (observed: the window came out 1px wide).
        //
        // **What it gives up is the word, never the depth**: folded, the word goes and the mark grows into the room
        // it was in, so the panel is the same two lines deep whatever any button in it is saying.
        ColumnLayout {
            id: stackCol
            visible: actionBtn.stacked
            // **The pair is centred, not laid into the cell.** A layout given more room than its children want shares
            // the rest out between their cells, and what sits in these two is a word's box and a mark's seat — so
            // every button in the row drew its mark at a height of its own (measured: the magnifier 2px under the
            // three commands').
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.verticalCenter
            // The mark's seat carries air its ink never uses — a step either side of the icon
            // (`ActionButtonSeat.implicitHeight`), and the room the drawing leaves inside its own grid — while the
            // word above it is ink from the first row. Centred by their boxes the pair reads high (measured against
            // the frame's own middle).
            anchors.verticalCenterOffset: Metrics.opticalDrop
            // **No step of its own between the two lines.** The air is there twice over already: the word's box
            // keeps a descent under the baseline and the mark's seat keeps a step over its ink
            // (`ActionButtonSeat.implicitHeight`), and neither is drawn on. A step on top of those left the mark
            // half again as far from the word as the word is from the frame (measured, against the frame's own two).
            spacing: 0

            ActionButtonLabel {
                id: stackLabel
                Layout.alignment: Qt.AlignHCenter | Qt.AlignBottom
                visible: !actionBtn.folded
                text: actionBtn.text
                code: actionBtn.code
                // The line the step carries, rather than the one the family does: the mark under this word has to
                // stand on the same row as the marks beside it (`ActionButtonLabel.lineBox`).
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
                // **A step under the band's**, because this word shares its button with a mark under it rather than
                // standing beside one: two lines in a row that is two lines deep, and the word is the half a reader
                // already knows.
                fontSize: Theme.fontSm
            }
            ActionButtonSeat {
                id: stackSeat
                // Under the word while there is one, and **on the button's own middle once there is not**
                //: with nothing over it, a mark still held to the top would sit in the
                // upper half of a box that is two lines deep.
                Layout.alignment: actionBtn.folded ? Qt.AlignHCenter | Qt.AlignVCenter
                                                   : Qt.AlignHCenter | Qt.AlignTop
                // **A step up.** The seat's air is not even about its ink — a step over it, a step under it, and a
                // drawing that reaches the floor of its own grid more often than the ceiling — so a mark hung at the
                // seat's own middle comes to rest low in the button. The two margins cancel, so the cell keeps its
                // depth and nothing else on the button moves.
                Layout.topMargin: actionBtn.folded ? 0 : -Metrics.opticalDrop
                Layout.bottomMargin: actionBtn.folded ? 0 : Metrics.opticalDrop
                kind: actionBtn.kind
                holdMs: actionBtn.armedMs
                // **The mark is what this button is read by here**:
                // a word set under a mark is the half a reader already knows, so the mark takes the step a mark
                // standing alone in a band takes (規約 §寸法). **Into the room the word was in** once the word is
                // gone — the button's depth does not move, so that line is the mark's.
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
