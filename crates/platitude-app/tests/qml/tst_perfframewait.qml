import QtQuick
import QtTest
import "../../src/auto" as Automation

// The wait the perf driver times its frames on (`PerfFrameWait`), fed a window's pass, sync and swap in the orders a
// render loop delivers them: the frame it answers to is the first swapped after the sync of a pass made after the
// ask, on a loop that says the three in one call and on one that queues the last two from another thread.
TestCase {
    id: testCase

    name: "PerfFrameWait"

    Component {
        id: maker

        Automation.PerfFrameWait {}
    }

    function fresh() {
        return createTemporaryObject(maker, testCase)
    }

    /// The basic and software loops: pass, sync and swap in one call, nothing in between.
    function test_a_loop_that_says_all_three_at_once_answers_with_its_first_frame() {
        const wait = testCase.fresh()
        wait.ask()
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 0)
        compare(wait.step, "")
    }

    /// The threaded loop with a frame on its way: its swap is delivered after the ask, and after the pass too.
    function test_a_frame_already_on_its_way_is_passed_over() {
        const wait = testCase.fresh()
        wait.passed()
        wait.synced()
        wait.ask()
        verify(!wait.swapped())
        wait.passed()
        verify(!wait.swapped())
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 2)
    }

    /// Frames a render thread draws for an animator carry no sync: any number of them go by.
    function test_frames_drawn_with_no_sync_are_passed_over() {
        const wait = testCase.fresh()
        wait.ask()
        for (let i = 0; i < 5; i++)
            verify(!wait.swapped())
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 5)
    }

    /// The pass before has not had its sync delivered when the next pass begins: that sync arrives first, and its
    /// frame with it.
    function test_the_sync_of_the_pass_before_is_not_this_pass_s() {
        const wait = testCase.fresh()
        wait.passed()
        wait.ask()
        wait.passed()
        wait.synced()
        verify(!wait.swapped())
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 1)
    }

    /// A pass the render thread made no sync for reads like one whose sync is still on its way: the wait after it
    /// lets its own frame go by and takes the next pass's.
    function test_a_pass_with_no_sync_costs_the_wait_after_it_a_frame() {
        const wait = testCase.fresh()
        wait.passed()
        wait.ask()
        wait.passed()
        wait.synced()
        verify(!wait.swapped())
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 1)
    }

    /// A sync with no pass made for the wait — one made before the ask, or a window grab's — answers nothing.
    function test_a_sync_without_a_pass_after_the_ask_answers_nothing() {
        const wait = testCase.fresh()
        wait.passed()
        wait.ask()
        wait.synced()
        wait.synced()
        verify(!wait.swapped())
        compare(wait.step, "asked")
        wait.passed()
        wait.synced()
        verify(wait.swapped())
    }

    /// The threaded loop's steady state: the sync wakes the GUI thread, which polishes the next pass before the
    /// swap of this one is delivered. The pass after the wait's sync is not the wait's, and takes nothing away.
    function test_a_pass_after_its_sync_does_not_take_the_frame_away() {
        const wait = testCase.fresh()
        wait.ask()
        wait.passed()
        wait.synced()
        wait.passed()
        verify(wait.swapped())
        compare(wait.passedOver, 0)
    }

    /// The same, one step earlier: a pass lands while the wait is polished and its sync still out. The wait keeps
    /// the count it took from its own pass, and the first sync after it is still the one it waits for.
    function test_a_pass_over_a_polished_wait_leaves_its_count_alone() {
        const wait = testCase.fresh()
        wait.ask()
        wait.passed()
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 0)
    }

    /// A swap nobody asked about is nobody's, and a wait answers once.
    function test_a_wait_answers_once() {
        const wait = testCase.fresh()
        verify(!wait.swapped())
        compare(wait.passedOver, 0)
        wait.ask()
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        verify(!wait.swapped())
        compare(wait.passedOver, 0)
    }

    /// A second ask starts over: what the first had seen is not the second's.
    function test_asking_again_starts_over() {
        const wait = testCase.fresh()
        wait.ask()
        wait.passed()
        wait.synced()
        wait.ask()
        verify(!wait.swapped())
        wait.passed()
        wait.synced()
        verify(wait.swapped())
        compare(wait.passedOver, 1)
    }
}
