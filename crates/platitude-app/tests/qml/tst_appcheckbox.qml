import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A check box handed its least (`AppCheckBox.leastWidth`): the words' box still holds the cut mark whole, so words
// cut to nothing say they were cut, and the style's right padding stands past the mark as it does past whole words.
Item {
    width: 400
    height: 60

    AppCheckBox {
        id: box
        text: "Amend the last commit"
        font.pixelSize: Theme.fontSm
    }

    TestCase {
        name: "AppCheckBox"
        when: windowShown

        /// The words after the box's seat (`CutName`).
        function words() {
            return box.contentItem.children[0]
        }

        function test_the_least_holds_the_mark() {
            box.width = box.leastWidth
            tryVerify(() => Math.abs(words().width - Math.max(0, box.availableWidth - words().x)) < 0.5)
            verify(words().width >= words().markWidth - 0.5,
                   "the words' box is " + words().width + " for a mark of " + words().markWidth)
            tryVerify(() => words().cutting, undefined, "the words never cut to the mark")
            let mark = null
            for (const child of words().children)
                if (child instanceof CutMark)
                    mark = child
            verify(mark !== null && mark.visible, "no mark drawn")
            const end = mark.mapToItem(box, mark.implicitWidth, 0).x
            verify(end <= box.width - box.rightPadding + 0.5,
                   "the mark runs " + (end - box.width + box.rightPadding) + "px into the box's right padding")
        }
    }
}
