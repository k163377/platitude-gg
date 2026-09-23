import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The word half of a toolbar action: the wording itself, a command's flag with drawn dashes, the chip a command
// wears, and the mark that says the last go did not work. The box it is measured into is the widest wording the button
// ever says, so the toolbar holds still when the state moves.
Item {
    id: btnLabel

    /// What this state says.
    property string text: ""
    /// The label is a git command said in git's own spelling, and wears the chip that says so (デザイン規約 §git 用語のコード表記).
    property bool code: false
    /// The box this word is measured into, where it is one line of a button written on two (`ActionButton.stacked`).
    /// **The two families answer a line box differently at the same step**, so a first line left to the family stands
    /// the mark under it a row off the marks beside it (measured: the find button's magnifier 2px under the three
    /// commands'). Zero takes the family's own, which is every word that has a line to itself.
    property int lineBox: 0
    /// What the wording is set at. Normal everywhere a word stands on a ground of its own; heavier where it has to
    /// hold its own beside the mono chips of a command (`TopBar`'s find button — a UI face at the same size reads
    /// lighter than a mono one, and the row is read as one set).
    property int wordWeight: Font.Normal
    /// Text the box is measured for, and whether that wording is a command (the two families measure differently, so
    /// the box has to be told which one it is holding).
    property string widestText: ""
    property bool widestCode: false
    /// How much room the button's cell left this word; -1 is "as much as it wants", which is every button that is
    /// measured to its own content (`ActionButton.wordRoom`).
    property real cap: -1
    /// The word is given up altogether and the button is down to its mark (`ActionButton.folded`). The cell keeps its
    /// **height** — the state group beside it borrows the button's box (規約 §ウィンドウの縁), and a box that lost its height
    /// here would take that mark down with it.
    property bool folded: false
    /// The ceiling on a single wording, whatever the cell allows.
    readonly property int wordCeiling: 240
    /// What this wording would like to be, before the cell has its say — and the one measurement here a cap leaves
    /// alone. Read off a hidden label of the whole wording: what is drawn
    /// depends on this answer (a capped wording gives its flag up, below), so reading the parts back would close the
    /// ring. Typed dashes, like the box the buttons share, which the drawn flag is designed to sit inside.
    readonly property real wantWidth: btnLabel.phrased ? phraseRow.implicitWidth
        : Math.min(wanted.implicitWidth, btnLabel.wordCeiling)
    /// Cut down to fit: what the cell left is less than the wording wants, so it elides into what there is.
    readonly property bool capped: btnLabel.cap >= 0 && btnLabel.cap < btnLabel.wantWidth
    /// What the chip is drawn round and what the `!` stands past — the **ink**, once a cut is in it. An elided word is
    /// painted narrower than the width it was given, and a ground stretched to the width leaves a tail with no letters
    /// in it (app-ui.md).
    readonly property real inkWidth: btnLabel.capped
        ? btnLabel.headRun + btnLabel.flagRoom : btnLabel.implicitWidth
    /// How far the head reaches: the width it was given, or — once a cut is in it — the ink it actually painted. What
    /// follows the head follows the letters, or the flag stands off in the air
    /// a `…` left behind (measured, the chip's ground ended before the flag did).
    readonly property real headRun: btnLabel.capped ? headText.paintedWidth : headText.width
    /// The colour the word is drawn in — the button's own `fg`.
    property color tint: Theme.textPrimary
    /// The last go at what this button does did not work.
    property bool alert: false
    property color alertTone: btnLabel.tint
    /// Pull the mark back to a letter's distance from the word.
    property bool alertTight: false
    property int fontSize: Theme.fontMd

    // The box is the widest wording's ink and nothing else. What stands between that ink and the frame is the button's
    // own air, shared out by one rule in every state (`ActionButton.slack`) — the widest included, whose slack is
    // nothing and whose air is therefore the padding itself. The two families are charged the same gap: an extra one
    // makes the box jump whenever the two wordings cross in width.
    //
    // Measured from the font even where the flag is drawn (see below) — the box is what holds the toolbar still, and it
    // stays put whatever rule is chosen for the flag.
    readonly property real box: widest.implicitWidth
    /// A command's flag, set apart from the command itself so its dashes can be drawn. Every dash the
    /// mono family carries is the same 7px rule in an 8px cell (measured over U+002D / 2010 / 2011 / 2212), and on the
    /// wording the shared box was measured for that is what leaves the mark no room past the word. Drawn, the rule's
    /// length and the air either side are ours to pick (デザイン規約 §git 用語のコード表記).
    ///
    /// **The flag stays whole.** What a cut takes off a wording is the end of
    /// it, and the end of this one is what the reader can be wrong about: `push -f` cut to `push …` is a push with an
    /// ellipsis after it, which everywhere else in the world means "asks first" (measured, it read as exactly
    /// that). So the command gives and the flag stays — `pu… -f` says a cut command *and* what it would do
    /// (規約 §長押し「警告の色は枠と印が持つ」, and the same order the commit phrase gives its parts up in).
    readonly property int flagAt: btnLabel.code ? btnLabel.text.indexOf(" -") : -1
    readonly property bool splitFlag: btnLabel.flagAt > 0
    /// What the flag holds, gap and drawn rules included. Read off the flag's own row, so a cut cannot move it.
    readonly property real flagRoom: btnLabel.splitFlag ? flagRow.implicitWidth : 0
    readonly property string head: btnLabel.splitFlag
        ? btnLabel.text.substring(0, btnLabel.flagAt) : btnLabel.text
    /// The flag with its leading dashes taken off, and how many of them there were.
    readonly property string flagRest: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).replace(/^-+/, "") : ""
    readonly property int dashCount: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).length - btnLabel.flagRest.length : 0

    /// The label is a phrase with a command at each end: a chip, the words between them, and
    /// a second chip in a colour of its own. The commit button is the one button that says one — it names the command
    /// *and* where the command will land, and neither half is a phrase about the other (デザイン規約 §git 用語のコード
    /// 表記). Empty leaves the ordinary single-wording path exactly as it was.
    property string phraseHead: ""
    /// The one number in the phrase, kept apart from the words around it so the words can give way without taking it
    /// (デザイン規約 §git 用語のコード表記).
    property string phraseCount: ""
    property string phraseTail: ""
    property color phraseTailTint: btnLabel.tint
    /// A short warning said after the phrase's words, in the note's own step and colour — the vocabulary the menu
    /// rows already use for the same thing (`AppMenuItem.note`).
    ///
    /// **Inside the phrase.** The phrase is what the button says, and the
    /// warning is said *about* what it says; a line of its own above a button that fills a pane reads as a
    /// heading for the pane, and it takes the pane's floor off the button
    /// (デザイン規約 §履歴を編集する — 実行ボタンの 3 状態). Empty says nothing.
    property string phraseNote: ""
    property color phraseNoteTint: Theme.warning
    /// The hold's mark, when the phrase is on a button that is held.
    ///
    /// **Inside the phrase.** The seat stands at the row's left edge, and a
    /// phrase is centred in a cell that fills the row — so a mark left out there ends up alone against the far edge,
    /// with the words it is meant to introduce adrift in the middle. §長押し puts the mark immediately ahead of the
    /// word; here that means ahead of the first chip (measured, the stranded mark).
    property int phraseHoldMs: 0
    property real phraseHoldProgress: 0
    /// Whom the phrase's action will be attributed to, at its end (`… by <face>`). -1 draws nothing at all — this is
    /// the commit button's own ending, and no other button has one (デザイン規約 §アバターを与える).
    property int phraseFace: -1
    property string phraseFaceUrl: ""
    /// How many more people the same action credits, as the `+N` the co-author line already uses.
    /// Zero says nothing: the ordinary case carries no mark.
    property int phraseMates: 0
    /// What is said about that person's signature, in the mark's own vocabulary (`SignatureMark.kind`), and the one
    /// line its tooltip carries. Empty puts nothing on the face.
    property string phraseSignature: ""
    property string phraseSignatureTip: ""
    /// Stands in for the pointer on that tick, so its one line can be photographed — hover cannot be injected.
    property bool phraseSignaturePointedAt: false
    /// Whether that line is on screen — the tooltip's own visible, so a cut binding cannot read as green.
    readonly property bool phraseSignatureTipShown: phraseAvatar.signatureTipShown
    readonly property bool phrased: btnLabel.phraseHead !== ""
    /// How wide the phrase may be: **the width this cell was given.** A phrased button
    /// fills a pane, and a request for the phrase's full width is a
    /// floor the pane cannot go under — the column would stand as wide as the longest branch name anyone has checked
    /// out, and every box sized to fill it paints past the window's edge (the accident `details-fit` catches, on this
    /// side of the app). Reading the given width closes no ring: nothing here feeds the request back.
    readonly property real phraseRoom: btnLabel.phrased ? btnLabel.width : 0
    // ---- what gives, and in what order ---------------------------------
    //
    // **The ref is what the reader can be wrong about, so it is the last thing to give**.
    // The parts that never give are the ones with no length of their own: the mark, the command, the count, the face,
    // and the note (a count and words of ours, both of them as long as they will ever be).
    // What gives, weakest first: the `+N`, then the app's own words, then — only if there is still nothing left — the
    // ref itself.
    /// What the parts that never give are holding, each with the gap that follows it. Counted per part: a `Row`
    /// charges a gap for every visible child, so a part that is not there does not owe one.
    readonly property real phraseFixed:
        (btnLabel.phraseHoldMs > 0 ? Theme.iconSm + phraseRow.spacing : 0)
        + headChip.implicitWidth + phraseRow.spacing
        + (countWord.visible ? countWord.implicitWidth + phraseRow.spacing : 0)
        + (noteWord.visible ? noteWord.implicitWidth + phraseRow.spacing : 0)
        + (btnLabel.phraseFace >= 0 ? byWord.implicitWidth + phraseAvatar.width + 2 * phraseRow.spacing : 0)
    /// What is left for the three that do.
    readonly property real phraseFree: Math.max(0, btnLabel.phraseRoom - btnLabel.phraseFixed)
    /// The ref, given everything the weaker two do not need. Down to a floor: elided past this a branch name says
    /// nothing at all, and at that point the words may as well go too.
    readonly property real tailCap:
        btnLabel.phraseRoom <= 0 ? 0 : Math.max(Theme.buttonMinWidth,
                                                btnLabel.phraseFree - btnLabel.midCap - btnLabel.mateRoom)
    /// **What gives leaves a `…` behind.** A cut has to show itself: a phrase that drops its words silently reads as
    /// a complete sentence that happens to be terse, and the reader has no way to know a word was taken out. So the
    /// floor for anything that gives is the mark itself.
    readonly property real cutMark: ellipsis.implicitWidth
    /// The app's own words: what the ref left them, down to the mark.
    readonly property real midCap:
        btnLabel.phraseRoom <= 0 || btnLabel.text === "" ? 0
        : Math.max(btnLabel.cutMark, Math.min(midWords.implicitWidth,
                                              btnLabel.phraseFree - tailWord.wantWidth - btnLabel.mateRoom
                                              - phraseRow.spacing))
    /// The `+N` goes first of all — it is the one part whose absence changes nothing about where the commit lands.
    /// Gone, it too leaves the mark: `by <face> …` still says there are more names here than the one drawn.
    readonly property bool mateFits:
        btnLabel.phraseFree - tailWord.wantWidth - midWords.implicitWidth
        >= mateFull.implicitWidth + phraseRow.spacing
    readonly property string mateWord: qsTr("+%1").arg(btnLabel.phraseMates)
    readonly property real mateRoom:
        btnLabel.phraseMates <= 0 ? 0
        : (btnLabel.mateFits ? mateFull.implicitWidth : btnLabel.cutMark) + phraseRow.spacing

    implicitWidth: btnLabel.phrased ? phraseRow.implicitWidth
                   : btnLabel.folded ? 0
                   : btnLabel.headRun + (btnLabel.splitFlag ? flagRow.width : 0)
    implicitHeight: btnLabel.phrased ? phraseRow.implicitHeight
                    : btnLabel.lineBox > 0 ? btnLabel.lineBox : headText.implicitHeight
    Layout.maximumWidth: btnLabel.phrased ? Number.POSITIVE_INFINITY : btnLabel.wordCeiling
    // Its own width: what the shared box asks for past this wording is held by the button's padding (`slack`), so the
    // chip and the mark, both measured off this cell, keep sitting on the word.
    //
    // A phrase asks for nothing and takes what the row has: it is the only wording with a part that can give (the
    // ref), and a request for its full width is a floor the pane cannot go under (see `phraseRoom`).
    Layout.fillWidth: btnLabel.phrased
    Layout.preferredWidth: btnLabel.phrased ? 0 : btnLabel.implicitWidth
    Layout.alignment: Qt.AlignVCenter

    // Measured only: a hidden item is left out of the layout, and a Label measures the way the visible one does
    // — TextMetrics reports a few pixels tighter, which is enough of a difference to shift the toolbar it is here to
    // hold still.
    Label {
        id: widest
        visible: false
        text: btnLabel.widestText
        font.family: btnLabel.widestCode ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: btnLabel.widestCode ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    // This wording at the length it would like to be, measured the same way, hidden (`wantWidth`).
    Label {
        id: wanted
        visible: false
        text: btnLabel.text
        font.family: btnLabel.code ? Theme.monoFamily : Theme.uiFamily
        // The weight the word is drawn at, or what it wants is measured in a face it is not set in
        // (rules-refs/app-ui.md: 測る側と描く側は同じ font で).
        font.weight: btnLabel.wordWeight
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    // Also measured only: the mark a cut leaves, and the `+N` at the length it would like to be. Both are read
    // while working out who gives what, so neither can be the item whose width the answer sets.
    Label {
        id: ellipsis
        visible: false
        text: "…"
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    Label {
        id: mateFull
        visible: false
        text: btnLabel.mateWord
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    // Measured only, and only where the word is one of two (`lineBox`): the line that box is a box for. The step's
    // line is the plain word's, and the two families put their glyphs at different heights inside the same box — so a
    // command left to its own box sits a row off the plain word beside it (measured: `fetch` one row over `Search`).
    Label {
        id: lineRef
        visible: false
        text: "Ag"
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    // The phrase, when there is one: chip, words, chip. A row, because what has to line up here is
    // three baselines and two gaps — and the chips hang their own ground half a gap past their glyphs, so the gaps are
    // the ordinary word spacing the sentence would have had bare.
    Row {
        id: phraseRow
        visible: btnLabel.phrased
        // Centred in the cell: the cell fills the button's row (see
        // `Layout.fillWidth` above), and the two spacers `centred` puts either side cannot centre a cell that fills.
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs
        HoldIcon {
            visible: btnLabel.phraseHoldMs > 0
            progress: btnLabel.phraseHoldProgress
            tint: btnLabel.tint
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: Metrics.opticalDrop
        }
        CodeChip {
            id: headChip
            word: btnLabel.phraseHead
            tint: btnLabel.tint
            size: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The count never gives: it is the one number in the phrase, and a `5` costs nothing to keep. Out entirely
        // when there is none — a `Row` charges a gap for a child of no width, and the
        // command would sit two gaps off the word after it.
        Label {
            id: countWord
            visible: btnLabel.phraseCount !== ""
            text: btnLabel.phraseCount
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The app's own words, which give before the ref does.
        Label {
            id: midWords
            visible: btnLabel.midCap > 0
            width: Math.min(implicitWidth, btnLabel.midCap)
            elide: Text.ElideRight
            text: btnLabel.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The warning said about what the phrase says, in the note's own step and colour — a step under the words it
        // follows, because it is meta about the button (デザイン規約 §タイポグラフィ),
        // and the words keep their own colour (§長押し — 警告の色は枠と印が持つ).
        Label {
            id: noteWord
            visible: btnLabel.phraseNote !== ""
            text: btnLabel.phraseNote
            color: btnLabel.phraseNoteTint
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
            anchors.verticalCenter: parent.verticalCenter
        }
        // **The ref is the one part that gives.** Everything else in this phrase is bounded by what it says — a
        // command, a count, a face, a `+N` — but **git takes a branch name of any length**, so this is the one
        // part that has to be capped: what is left for it is the room the button has, less what the fixed parts hold.
        CodeChip {
            id: tailWord
            word: btnLabel.phraseTail
            tint: btnLabel.phraseTailTint
            size: btnLabel.fontSize
            cap: btnLabel.tailCap
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            id: byWord
            visible: btnLabel.phraseFace >= 0
            text: qsTr("by")
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // The face, wearing the same corner mark it wears anywhere else (`AvatarButton`) — no address, so nothing here
        // is pressable: the phrase is the button, and a target inside a target is two things to aim at.
        //
        // `iconLg`, which is the step the graph draws a node at beside a `fontMd` subject: a face is read as a picture
        // and so wants the proportion the graph's rows already give
        // it.
        AvatarButton {
            id: phraseAvatar
            visible: btnLabel.phraseFace >= 0
            face: btnLabel.phraseFace
            faceUrl: btnLabel.phraseFaceUrl
            signatureKind: btnLabel.phraseSignature
            signatureTip: btnLabel.phraseSignatureTip
            signaturePointedAt: btnLabel.phraseSignaturePointedAt
            badgeInk: Theme.iconXs
            // Clear of the round face's corner by a step, one pixel short of the frame beside it, so what the eye
            // reads is a tick beside a face (§署名の表示).
            badgeTopOut: Theme.spaceXs - Theme.borderWidth
            badgeRightOut: Theme.spaceXs - Theme.borderWidth
            width: Theme.iconLg
            height: Theme.iconLg
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            id: mateCount
            visible: btnLabel.phraseMates > 0
            text: btnLabel.mateFits ? btnLabel.mateWord : ellipsis.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }
    Label {
        id: headText
        visible: !btnLabel.phrased && !btnLabel.folded
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // **On the row the plain word's baseline lands on, where this word is one of two.** Each family is centred
        // by a box of its own and the two are different heights at one step (`lineRef`), so a command came out a row
        // under the plain word beside it. Both sides are taken down to the row they are *drawn* on, which is the
        // only place the reader compares them.
        anchors.verticalCenterOffset: btnLabel.lineBox > 0
            ? Math.floor((btnLabel.lineBox - lineRef.height) / 2 + lineRef.baselineOffset)
              - Math.floor((btnLabel.lineBox - headText.height) / 2 + headText.baselineOffset)
            : 0
        // Bounded by the cell's own ceiling: the cell's width comes from this, so reading it back
        // would close a loop. The cap is what the *button* left this word
        // (`ActionButton.wordRoom`), which is arrived at from the width the row handed the button and nothing here.
        width: Math.min(implicitWidth, btnLabel.wordCeiling,
                        btnLabel.capped ? Math.max(0, btnLabel.cap - btnLabel.flagRoom)
                                        : Number.POSITIVE_INFINITY)
        text: btnLabel.head
        color: btnLabel.tint
        font.family: btnLabel.code ? Theme.monoFamily : Theme.uiFamily
        font.weight: btnLabel.wordWeight
        // A command and its flag are one thing said, and a mono space is far wider than the air the chip keeps at its
        // own ends — left alone, `-f` drifts away from the `push` it belongs to and the chip reads as two words on one
        // ground (デザイン規約 §git 用語のコード表記).
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
        elide: Text.ElideRight
    }
    // The flag: the air the mono space held, a drawn rule for each dash, and the letters after them in the font. The
    // rule is `borderWidth` thick because that is what the font's own dash measures at this size, and a hair of air
    // follows it so the letter does not touch.
    Row {
        id: flagRow
        visible: btnLabel.splitFlag && !btnLabel.folded
        anchors.left: headText.left
        anchors.leftMargin: btnLabel.headRun
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
    // The chip a command wears, behind the glyphs and only as wide as they are — the box around them is measured for
    // the longest wording of the pair, and a ground stretched to that would draw a chip the word does not fill. Half a
    // gap of tint hangs off either end, the same as a menu row's (デザイン規約 §git 用語のコード表記).
    Rectangle {
        z: -1
        visible: btnLabel.code && !btnLabel.folded
        x: -Theme.spaceXs / 2
        width: btnLabel.inkWidth + Theme.spaceXs
        // Off the word's own step: the cell is the mono family's line box, and that
        // box is the one thing about this dress each OS decides for itself (measured @14px: Cascadia Mono 16, Noto Sans
        // Mono CJK JP 21 — a CJK family carries half an em more leading than a Latin one). Left to the cell the same
        // command wore a wash on Windows and a tag on Ubuntu, while the glyphs themselves sat
        // on the same rows on both (デザイン規約 §git 用語のコード表記).
        height: btnLabel.fontSize + Theme.spaceXs / 2
        // The word's own middle, not the cell's: a word set to the line's baseline rather than to the cell
        // (`headText`) would leave its ground behind it.
        anchors.verticalCenter: headText.verticalCenter
        radius: Theme.radiusSm
        color: Theme.bgHover
    }
    // Past the word's end — past the chip's edge where there is one (a mark crossing that edge reads as stuck to the
    // chip). Closer still after a flag: that is the longest thing the button says and
    // the one wording whose right-hand side is short of room (デザイン規約 §リモートへ送る).
    //
    // **A phrase that ends in a face puts the mark at its head.** That corner of the face is the signature's
    // (§署名の表示), and a yellow `!` standing there would read as something said about the signature, which it is
    // never about. The head is the one end of this phrase that carries nothing.
    NavIcon {
        // The folded button wears this mark in the seat, where the icon's own corner is (`ActionButtonSeat`).
        visible: btnLabel.alert && !btnLabel.folded
        kind: "bang"
        tint: btnLabel.alertTone
        width: Theme.iconSm
        height: Theme.iconSm
        // Raised, and a half-gap off the command — the seat every other `!` in the app has (the toolbar's `push -f`),
        // so the mark reads the same wherever it is met. On a phrase it stands at the head, for
        // the reason above.
        x: btnLabel.phrased
           ? phraseRow.x - width + Theme.spaceXs / 2
           : btnLabel.inkWidth - (btnLabel.splitFlag ? Theme.spaceXs / 2 : 0)
             - (btnLabel.alertTight ? Theme.spaceXs : 0)
        // **On the line, where the word is the first of two.** There is nothing over that word but the button's own
        // frame, and the raised seat put the mark's stem on the frame's top line (measured). The mark's ink starts at
        // the top of its box, so it still stands over the word it follows.
        y: btnLabel.phrased
           ? phraseRow.y - Theme.spaceXs
           : btnLabel.lineBox > 0 ? headText.y : -Theme.spaceXs
    }
}
