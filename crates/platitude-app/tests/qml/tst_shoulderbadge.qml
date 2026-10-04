import QtQuick
import QtTest
import platitude.ui

// The shoulder rule (`ShoulderBadge`, デザイン規約 §寸法 の右肩の印): half the mark it sits on, never under the floor,
// its centre a `spaceXs` in from that mark's right edge and its top a `spaceXs` over that mark's top.
Item {
    id: root
    width: 100
    height: 100

    NavIcon {
        id: large
        x: 10
        y: 40
        width: Theme.iconXl
        height: Theme.iconXl
        kind: "tag"
        ShoulderBadge {
            id: onLarge
            kind: "eye"
        }
    }
    NavIcon {
        id: small
        x: 60
        y: 40
        width: Theme.iconSm
        height: Theme.iconSm
        kind: "tag"
        ShoulderBadge {
            id: onSmall
            kind: "remote"
        }
    }

    TestCase {
        name: "ShoulderBadge"

        function test_half_the_mark_it_sits_on() {
            compare(onLarge.width, Theme.iconXl / 2, "the folded rail's TAGS eye: 12 on 24")
            compare(onLarge.height, onLarge.width)
        }

        function test_never_under_the_floor() {
            verify(Theme.iconSm / 2 < Theme.iconBadgeMin, "the case this is about")
            compare(onSmall.width, Theme.iconBadgeMin)
            compare(onSmall.height, Theme.iconBadgeMin)
        }

        function test_its_centre_in_from_the_edge_and_its_top_over_it() {
            for (const [badge, mark] of [[onLarge, large], [onSmall, small]]) {
                compare(badge.x + badge.width / 2, mark.width - Theme.spaceXs, "centre in from the right edge")
                compare(badge.y, -Theme.spaceXs, "top over the mark's")
            }
        }
    }
}
