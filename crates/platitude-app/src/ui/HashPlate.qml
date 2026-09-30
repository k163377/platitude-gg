pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit's own hash over its parents', rows aligned right. Each hash is a control and a value: a press that lets go
// where it landed is the control's (copy the hash / go to that parent), one that travels drags over the digits
// (デザイン規約 §右のペインの字は掴める). A `TextEdit` takes the press wherever it is drawn, so the words take none
// (`grabbable: false`) and a hand over each face hands drags down to them (rules-refs/app-ui.md「右パネルの値は掴める字」).
// The parents share one line, cut to the room the row leaves it (デザイン規約 §右のペインの親).
ColumnLayout {
    id: plate

    property string sha8: ""
    property string fullSha: ""
    /// Every parent, full hex in git's order (`DetailsModel.parentHexes`); none draws no parent line.
    property var parents: []
    /// How wide the parent line may stand (`CommitAuthorRow.parentRoom`).
    property real parentRoom: 0
    /// How loud the hash is: full in the details pane, under the message in a list (デザイン規約 §複数のコミットを選ぶ).
    property color shaColor: Theme.textPrimary
    /// And how big: a list takes a step down, since mono at the words' size reads louder than they do.
    property real shaSize: Theme.fontMd

    signal copyRequested(string text)
    signal parentClicked(string oidHex)

    // ---- how the parent line fits its room (デザイン規約 §右のペインの親) ----
    /// The digits a parent shows whole, and the fewest it is cut to before the tail of the line gives way.
    readonly property int wholeDigits: 8
    readonly property int cutDigits: 3
    /// One digit, the cut mark and the count's characters, in the faces they are drawn in: measured, as the chip
    /// names' floor is (rules-refs/app-ui.md「字数の床の値付けは実測の字送り」). Properties, so the fit re-reads them.
    readonly property TextMetrics digitInk: TextMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        text: "0"
    }
    readonly property TextMetrics cutInk: TextMetrics {
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        text: "…"
    }
    // The count's own font: the label draws in whatever face the window gives a `Label`, which need not be the one
    // `Theme.uiFamily` names.
    readonly property TextMetrics plusInk: TextMetrics {
        font: moreLabel.font
        text: "+"
    }
    readonly property TextMetrics figureInk: TextMetrics {
        font: moreLabel.font
        text: "0"
    }
    /// The commas between the links are drawn in the same face as the count.
    readonly property TextMetrics commaInk: TextMetrics {
        font: moreLabel.font
        text: ", "
    }
    readonly property real commaWidth: Math.ceil(plate.commaInk.advanceWidth)
    /// Line scaled with the grid (rules-refs/app-ui.md「語の隣に立つ印は `iconSm`」).
    readonly property real backStroke: Metrics.iconStroke * Theme.iconSm / Theme.iconMd
    /// The line's head before the first hash: the arrow, set among the hashes as one more character — its box, and
    /// no step of the row's beside it (デザイン規約 §余白「語の列の中に組む印」).
    readonly property real leadWidth: Theme.iconSm
    /// One parent's digits: whole, or `digits` of them and the mark, which `LineText` lays over the next one. Every
    /// piece is priced at the whole pixel the layout hands it, which is the one above.
    function hashWidth(digits) {
        return Math.ceil(digits >= plate.wholeDigits ? plate.wholeDigits * plate.digitInk.advanceWidth
                                                     : digits * plate.digitInk.advanceWidth + plate.cutInk.advanceWidth)
    }
    /// The line with `shown` parents at `digits`, a comma after each but the last, and `more` said as `+N`.
    function lineWidth(shown, digits, more) {
        const count = more > 0 ? Theme.spaceXs + Math.ceil(plate.plusInk.advanceWidth
                                                           + String(more).length * plate.figureInk.advanceWidth) : 0
        return plate.leadWidth + shown * plate.hashWidth(digits) + (shown - 1) * plate.commaWidth + count
    }

    // ---- onto the holder's baselines (`CommitAuthorRow.firstBase`) ----
    /// Where the words beside this plate stand, in the plate's y — the first line's and the second's — or -1, where
    /// the plate stands alone (`ChosenCommitRow`).
    property real firstBase: -1
    property real secondBase: -1
    /// How far down the plate an item stands as laid out. Transforms are left out: they are what this feeds.
    function laidY(item) {
        let y = 0
        for (let at = item; at !== null && at !== plate; at = at.parent)
            y += at.y
        return y
    }
    /// The hash row moved onto the first baseline, and the parent line onto the second. The line is read off a
    /// hash-faced word that stands in it whatever the line holds (`lineRef`), so it does not move as the links step
    /// out for the card.
    readonly property real hashShift: plate.firstBase < 0 ? 0
        : plate.firstBase - plate.laidY(shaText) - shaText.baselineOffset
    readonly property real lineShift: plate.secondBase < 0 ? 0
        : plate.secondBase - plate.laidY(lineRef) - lineRef.baselineOffset

    // ---- while the count's card is out (デザイン規約 §右のペインの親) ----
    /// The card is out: the line stands as one parent's would — the arrow, then the room the card's first row fills,
    /// the card's frame and row air included, so the arrow stays clear of it.
    property bool unrolled: false
    /// The card's frame and row air left of its hashes (`ParentListCard.hashInset`), from the same two tokens.
    readonly property real unrollInset: Theme.borderWidth + Theme.spaceXs
    /// The arrow's box in `item`'s coordinates: a mark and no target, outside the card (automation reads its edge).
    function arrowBox(item) {
        const at = arrowSeat.mapToItem(item, 0, 0)
        return Qt.rect(at.x, at.y, arrowSeat.width, arrowSeat.height)
    }
    /// Every parent, cut evenly from whole down to `cutDigits`; past that the tail goes to `+N`. Two always stand.
    readonly property var parentFit: {
        const count = plate.parents.length
        if (count < 2)
            return { shown: count, digits: plate.wholeDigits }
        for (let digits = plate.wholeDigits; digits >= plate.cutDigits; digits--) {
            if (plate.lineWidth(count, digits, 0) <= plate.parentRoom)
                return { shown: count, digits: digits }
        }
        for (let shown = count - 1; shown > 2; shown--) {
            if (plate.lineWidth(shown, plate.cutDigits, count - shown) <= plate.parentRoom)
                return { shown: shown, digits: plate.cutDigits }
        }
        return { shown: 2, digits: plate.cutDigits }
    }
    /// Automation (verify-ui `details-parents`): how many links stand and how many digits each shows, and the count.
    readonly property int parentsShown: plate.parentFit.shown
    readonly property int parentDigits: plate.parentFit.digits
    readonly property int parentsLeft: plate.parents.length - plate.parentFit.shown
    readonly property alias moreSaid: moreLabel.text
    readonly property bool moreStands: moreLabel.visible
    /// The pointer rests on the count, which asks for the parents' card. The row owns the card
    /// (`CommitAuthorRow.parentsLit`).
    readonly property bool morePointed: moreHover.hovered
    /// Automation: where each word of the plate sits on the page, in `item`'s y (-1 for one not drawn).
    function baselines(item) {
        const link = plate.linkAt(0)
        const drawn = !!link && link.visible
        const comma = plate.parentsShown > 1 && plate.linkAt(0) ? plate.linkAt(0).comma : null
        return {
            hash: shaText.mapToItem(item, 0, shaText.baselineOffset).y,
            parent: drawn ? link.field.mapToItem(item, 0, link.field.baselineOffset).y : -1,
            comma: comma && comma.visible ? comma.mapToItem(item, 0, comma.baselineOffset).y : -1,
            count: moreLabel.visible ? moreLabel.mapToItem(item, 0, moreLabel.baselineOffset).y : -1,
            countSize: moreLabel.font.pixelSize
        }
    }
    /// The line in `item`'s coordinates.
    function parentLineRect(item) {
        const at = parentLine.mapToItem(item, 0, 0)
        return Qt.rect(at.x, at.y, parentLine.width, parentLine.height)
    }
    /// Where the parents' card opens: across, the line's span (its right end is a single parent's seat); down, the
    /// first hash's own box, so the card's first hash lands on the pixel row the line's stood on.
    function parentSeatRect(item) {
        const line = plate.parentLineRect(item)
        const link = plate.linkAt(0)
        if (!link)
            return line
        const hash = link.field.mapToItem(item, 0, 0)
        return Qt.rect(line.x, hash.y, line.width, link.field.height)
    }
    /// The line's left edge in `item`'s x, and its width as laid out.
    function parentLineLeft(item) {
        return parentLine.mapToItem(item, 0, 0).x
    }
    readonly property real parentLineWidth: parentLine.width

    /// The parent links, in git's order; a link past `parentsShown` is laid out nowhere.
    function linkAt(index) {
        return index < parentLinks.count ? parentLinks.itemAt(index) : null
    }
    function parentFields() {
        const fields = []
        for (let i = 0; i < plate.parentsShown; i++) {
            const link = plate.linkAt(i)
            if (link)
                fields.push(link.field)
        }
        return fields
    }
    /// The fields a sweep may select into, and must keep its hands off (`SweepRoom`).
    function valueFields() {
        return [shaText].concat(plate.parentFields())
    }
    /// Whether a point in `item`'s coordinates lands on a control of this plate, whose press is the control's whole.
    function claims(item, x, y) {
        if (plate.inside(hashCopy, item, x, y))
            return true
        for (let i = 0; i < plate.parentsShown; i++) {
            const link = plate.linkAt(i)
            if (link && link.visible && plate.inside(link.face, item, x, y))
                return true
        }
        return false
    }
    function inside(box, item, x, y) {
        const p = box.mapFromItem(item, x, y)
        return p.x >= 0 && p.y >= 0 && p.x < box.width && p.y < box.height
    }
    /// Where this plate begins, in `item`'s x.
    function leftEdge(item) {
        return plate.mapToItem(item, 0, 0).x
    }
    /// `"parent"` is the first parent's link, the one every commit but a root has.
    function fieldFor(which) {
        if (which !== "parent")
            return shaText
        const link = plate.linkAt(0)
        return link ? link.field : null
    }
    /// For the runs: the middle of that row's control, in `item`'s coordinates.
    function controlPoint(which, item) {
        const link = plate.linkAt(0)
        const box = which === "parent" ? (link ? link.face : null) : hashCopy
        return box ? box.mapToItem(item, box.width / 2, box.height / 2) : Qt.point(0, 0)
    }

    /// The controls' actions; handlers and runs both enter here (verify-ui §壊れない動詞の実装).
    function copyFullNow() {
        plate.copyRequested(plate.fullSha)
        hashCopy.copied = true
    }
    function goToParentNow(index) {
        if (index < plate.parents.length)
            plate.parentClicked(plate.parents[index])
    }

    function selectSha() { shaText.selectAll() }
    function selectParent() {
        const field = plate.fieldFor("parent")
        if (field)
            field.selectAll()
    }
    /// Clears every row's selection: a press on any of them starts a gesture on the whole plate.
    function dropValues() {
        const fields = plate.valueFields()
        for (let i = 0; i < fields.length; i++)
            fields[i].deselect()
    }
    readonly property alias shaSelected: shaText.selected
    /// What the first parent's field holds selected; read after a gesture, so a function (the links come and go).
    function parentSelected() {
        const field = plate.fieldFor("parent")
        return field ? field.selected : ""
    }

    /// Automation: the two gestures, through the hand's own three functions. The tap lands mid-face; the drag runs
    /// from the value's head off its far end.
    function tapAt(which) {
        const hand = plate.handFor(which)
        const at = plate.controlPoint(which, hand)
        hand.takeAt(at.x, at.y)
        hand.releaseNow()
    }
    function dragAcross(which) {
        const hand = plate.handFor(which)
        const field = plate.fieldFor(which)
        const near = field.mapToItem(hand, 0, field.height / 2)
        const far = field.mapToItem(hand, field.width, field.height / 2)
        hand.takeAt(near.x, near.y)
        hand.followAt(far.x, far.y)
        hand.releaseNow()
    }
    /// Automation: the word the copy control offers (hover cannot be injected; the tip itself is `tst_hashplate`'s).
    function tipWords() {
        return hashCopy.ToolTip.text
    }
    /// Automation: what the shared tip shows for this control, empty while none is up — differs from `tipWords` when
    /// a standing tip missed a new text.
    function tipSaid() {
        return plate.tipStanding ? hashCopy.ToolTip.toolTip.text : ""
    }
    /// Whether the window's one shared tip is up for this control, not merely up.
    readonly property bool tipStanding: {
        const tip = hashCopy.ToolTip.toolTip
        return !!tip && tip.visible && tip.parent === hashCopy
    }
    /// The attached tip snapshots its text when `visible` rises, missing a copy made during the delay; re-set as it
    /// comes up, which covers both orders (rules-refs/app-ui.md「右パネルの値は掴める字」).
    onTipStandingChanged: {
        if (plate.tipStanding)
            hashCopy.ToolTip.toolTip.text = hashCopy.ToolTip.text
    }
    function forceTip(on) {
        hashCopy.tipForced = on
    }
    /// Automation: the hand is there, enabled and full-face — runs enter its functions, so a missing one would pass.
    function handStands(which) {
        const hand = plate.handFor(which)
        const link = plate.linkAt(0)
        const face = which === "parent" ? (link ? link.face : null) : hashCopy
        return !!hand && !!face && hand.enabled && hand.width === face.width && hand.height === face.height
    }
    function handFor(which) {
        if (which !== "parent")
            return hashHand
        const link = plate.linkAt(0)
        return link ? link.hand : null
    }

    spacing: 0

    // The hash is the button (the 16px glyph alone is too small to aim at), lit as one control with its icon. Its own
    // flat wash: a HoverToolButton's panel would make it taller than one line, out of step with the author name.
    ToolButton {
        id: hashCopy
        Layout.alignment: Qt.AlignRight
        // On the name's baseline, wash and all.
        transform: Translate {
            y: plate.hashShift
        }
        hoverEnabled: true
        leftPadding: Theme.spaceXs
        // 0: the mark's ink ends on the line the parent hash and the message frame below end on.
        rightPadding: 0
        topPadding: 0
        bottomPadding: 0
        readonly property bool lit: hovered || visualFocus
        /// The hash was just copied, and the tip's one word says so (規約 §hover のツールチップ「結論を先頭に、1 行で書く」).
        property bool copied: false
        // Arriving clears it as well as leaving: a keyboard copy leaves it standing with no pointer near.
        onHoveredChanged: hashCopy.copied = false
        /// Automation: raise the tooltip with no pointer behind it (verify-ui §hover の絵の撮り方).
        property bool tipForced: false
        // The hand takes the pointer's press, so the face sinks from it until the press travels; `Space` sinks it too.
        down: hashCopy.pressed || (hashHand.containsPress && !hashHand.dragging)
        ToolTip.visible: hashCopy.hovered || hashCopy.tipForced
        ToolTip.delay: Metrics.tipDelayMs
        ToolTip.text: hashCopy.copied ? qsTr("Copied!") : qsTr("Copy full hash")
        // The keyboard's way in; the pointer's press goes to the hand.
        onClicked: plate.copyFullNow()
        background: Rectangle {
            radius: Theme.radiusSm
            color: hashCopy.down ? Theme.bgPressed : hashCopy.lit ? Theme.bgHover : "transparent"
            // Under the icon too: hash and icon are one target, with one line.
            Rectangle {
                visible: hashCopy.lit
                color: Theme.textPrimary
                height: Theme.borderWidth
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                anchors.leftMargin: hashCopy.leftPadding
                anchors.rightMargin: hashCopy.rightPadding
            }
        }
        contentItem: RowLayout {
            spacing: Theme.spaceXs
            LineText {
                id: shaText
                text: plate.sha8
                mono: true
                pixelSize: plate.shaSize
                color: plate.shaColor
                grabbable: false
                Layout.alignment: Qt.AlignVCenter
            }
            // Seated to the mark's ink, not its 16px box, so the air beside it is not spent twice (デザイン規約 §余白).
            Item {
                Layout.preferredWidth: copyMark.inkWidth
                Layout.preferredHeight: Theme.iconMd
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: copyMark
                    anchors.centerIn: parent
                    kind: "copyicon"
                    tint: hashCopy.lit ? Theme.textPrimary : Theme.textSecondary
                }
            }
        }
        // After the content, so it lies over the words.
        RowHand {
            id: hashHand
            field: shaText
            onTaken: plate.dropValues()
            onTapped: plate.copyFullNow()
        }
    }
    // The parents as git lists them, comma after comma, the arrow once at the head.
    RowLayout {
        id: parentLine
        visible: plate.parents.length > 0
        Layout.alignment: Qt.AlignRight
        // On the date's baseline, the arrow with it.
        transform: Translate {
            y: plate.lineShift
        }
        spacing: 0
        // A hash-faced word of no width that stands in the line whatever it holds: where the line's hashes sit, for
        // the shift above and the words below, and how tall the line is while the links step out for the card.
        LineText {
            id: lineRef
            text: "0"
            mono: true
            pixelSize: Theme.fontSm
            grabbable: false
            opacity: 0
            Layout.preferredWidth: 0
            Layout.alignment: Qt.AlignVCenter
        }
        // While the card is out the line stands as one parent's, narrower than it was: the rest of its width stays
        // held here, ahead of the arrow, so the plate keeps its width and the name beside it does not move.
        Item {
            visible: plate.unrolled
            Layout.preferredWidth: Math.max(0, plate.lineWidth(plate.parentsShown, plate.parentDigits,
                                                                plate.parentsLeft)
                                               - plate.leadWidth - plate.unrollInset
                                               - plate.hashWidth(plate.wholeDigits))
            Layout.preferredHeight: 1
        }
        // Drawn rather than `←` (デザイン規約 §寸法「印は描いて出す」), and set inline: its box is its seat, and what air
        // it has is the box's own (`leadWidth`). A mark, not a target: each hash is its own way to its parent, and the
        // arrow stays outside the card the count opens (デザイン規約 §右のペインの親).
        Item {
            id: arrowSeat
            Layout.preferredWidth: plate.leadWidth
            Layout.preferredHeight: Theme.iconSm
            Layout.alignment: Qt.AlignVCenter
            NavIcon {
                id: parentBack
                anchors.centerIn: parent
                kind: "arrow"
                // The family draws it leaving; turned to point back.
                rotation: 180
                tint: Theme.textLink
                width: Theme.iconSm
                height: Theme.iconSm
                stroke: plate.backStroke
            }
        }
        Repeater {
            id: parentLinks
            model: plate.parents
            delegate: Item {
                id: link

                required property string modelData
                required property int index
                readonly property alias field: parentText
                readonly property alias hand: parentHand
                /// The link itself — hash, underline and hand — without the comma after it.
                readonly property alias face: linkFace
                readonly property alias comma: commaLabel
                /// A comma follows every link but the last one standing.
                readonly property bool followed: link.index < plate.parentsShown - 1

                visible: !plate.unrolled && link.index < plate.parentsShown
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: linkFace.width + (link.followed ? plate.commaWidth : 0)
                implicitHeight: parentText.implicitHeight
                Item {
                    id: linkFace
                    width: plate.hashWidth(plate.parentDigits)
                    height: parent.height
                    ToolTip.visible: parentHand.containsMouse
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Go to parent commit")
                    // The whole short hash in the field, the cut drawn over it (`LineText`): a drag takes what shows,
                    // `Ctrl+A` all eight.
                    LineText {
                        id: parentText
                        width: parent.width
                        anchors.verticalCenter: parent.verticalCenter
                        text: link.modelData.substring(0, plate.wholeDigits)
                        mono: true
                        pixelSize: Theme.fontSm
                        color: Theme.textLink
                        grabbable: false
                    }
                    Rectangle {
                        visible: parentHand.containsMouse
                        color: Theme.textLink
                        height: Theme.borderWidth
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                    }
                    // Last, so it lies over the hash; also the link's only hover source (tooltip, underline).
                    RowHand {
                        id: parentHand
                        field: parentText
                        hoverEnabled: true
                        onTaken: plate.dropValues()
                        onTapped: plate.goToParentNow(link.index)
                    }
                }
                // Punctuation between two links, belonging to neither: a step down, like the count.
                Label {
                    id: commaLabel
                    visible: link.followed
                    x: linkFace.width
                    // On the hash's baseline: the two faces do not share one.
                    y: parentText.y + parentText.baselineOffset - commaLabel.baselineOffset
                    text: ", "
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
            }
        }
        // The parents the room could not hold, counted as the credit line counts its names (デザイン規約
        // §co-author の表示). Resting on it opens them all (`ParentListCard`).
        Label {
            id: moreLabel
            visible: !plate.unrolled && plate.parentsLeft > 0
            text: "+" + plate.parentsLeft
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
            Layout.leftMargin: Theme.spaceXs
            Layout.alignment: Qt.AlignVCenter
            // On the hashes' baseline, as the credit's count is on its name's.
            transform: Translate {
                y: lineRef.y + lineRef.baselineOffset - moreLabel.y - moreLabel.baselineOffset
            }
            // "Hover opens more", said before the hover as the credit line says it, a step down from the count.
            BandRule {
                color: Theme.borderStrong
            }
            // A handler is passive, so the card it opens keeps its own hover (規約 §hover のツールチップ).
            HoverHandler {
                id: moreHover
            }
        }
        // While the card is out, the room its first row stands in: where a single parent's hash would. As tall as the
        // line was with its count, or the plate would shrink under the card and the row it stands in would move.
        Item {
            visible: plate.unrolled
            Layout.preferredWidth: plate.unrollInset + plate.hashWidth(plate.wholeDigits)
            Layout.preferredHeight: Math.max(lineRef.implicitHeight, moreLabel.implicitHeight)
        }
    }
}
