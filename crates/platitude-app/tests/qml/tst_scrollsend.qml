import QtQuick
import QtTest
import platitude.ui

// A view's send (`ScrollSend`): a dash farther than its landing starts that far short of its aim (user decision), a
// nearer one is drawn the whole way, a move measures from where the one in flight is going, a halt leaves the view
// where it stands and a landing puts it where it was going. Over a bare flickable two hundred pixels wide and a hundred
// tall, so the landing is a quarter of the side the send goes along.
Item {
    id: root
    width: 200
    height: 100

    Flickable {
        id: flick
        width: root.width
        height: root.height
        contentWidth: 10000
        contentHeight: 10000
    }
    readonly property ScrollSend send: ScrollSend {
        id: send
        view: flick
        dashMs: 200
    }
    readonly property ScrollSend across: ScrollSend {
        id: across
        view: flick
        axis: "contentX"
        dashMs: 200
    }

    TestCase {
        name: "ScrollSend"
        when: windowShown

        function init() {
            send.halt()
            across.halt()
            flick.contentY = 0
            flick.contentX = 0
        }

        function test_a_far_dash_starts_a_landing_short_of_its_aim() {
            send.dashTo(1000)
            compare(flick.contentY, 1000 - Metrics.dashLandShare * flick.height, "the start is skipped at once")
            verify(send.dashing, "and the landing is drawn")
            compare(send.at, 1000)
            // The landing's end, then where it left the view: under load the aim was read while the send still ran.
            tryVerify(() => !send.sending, undefined, "the landing ends")
            compare(flick.contentY, 1000)
        }

        function test_a_far_dash_up_lands_from_below() {
            flick.contentY = 1000
            send.dashTo(0)
            compare(flick.contentY, Metrics.dashLandShare * flick.height)
            tryCompare(flick, "contentY", 0)
        }

        function test_a_sideways_dash_lands_over_the_width() {
            across.dashTo(1000)
            compare(flick.contentX, 1000 - Metrics.dashLandShare * flick.width)
            tryCompare(flick, "contentX", 1000)
        }

        function test_a_near_dash_is_drawn_the_whole_way() {
            send.dashTo(20)
            compare(flick.contentY, 0, "nothing is skipped")
            verify(send.dashing)
            tryCompare(flick, "contentY", 20)
        }

        function test_a_glide_is_drawn_the_whole_way() {
            send.glideTo(1000)
            compare(flick.contentY, 0, "a glide skips nothing")
            verify(send.gliding && !send.dashing)
            tryCompare(flick, "contentY", 1000)
        }

        function test_a_move_measures_from_where_the_one_in_flight_is_going() {
            send.dashTo(1000)
            send.glideTo(send.at + 50)
            compare(send.aim, 1050)
            verify(send.gliding && !send.dashing, "the glide took over from the dash")
            tryCompare(flick, "contentY", 1050)
        }

        function test_a_halt_leaves_the_view_where_it_stands() {
            send.dashTo(1000)
            send.halt()
            verify(!send.sending)
            const stood = flick.contentY
            // waits(paced): the dash's length twice over; a dash left running would have moved the view on.
            wait(send.dashMs * 2)
            compare(flick.contentY, stood)
        }

        function test_a_landing_puts_the_view_where_it_was_going() {
            send.dashTo(1000)
            send.land()
            compare(flick.contentY, 1000)
            verify(!send.sending)
        }

        function test_a_landing_at_rest_leaves_the_view() {
            flick.contentY = 30
            send.land()
            compare(flick.contentY, 30)
        }
    }
}
