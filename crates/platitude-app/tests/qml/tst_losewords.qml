import QtQuick
import QtTest
import platitude.ui

// What a branch loses, said of the branch (`Words.losesCommits`): the line under `Move <branch> here?` and under the
// first push that overwrites (デザイン規約 §長さ「失うもの 1 行」). One sentence for each count, and the name where the
// line's mark is seated.
Item {
    TestCase {
        name: "LoseWords"

        function test_one_commit_and_several_say_their_own_sentence() {
            compare(Words.losesCommits("main", 1), "main loses 1 commit.")
            compare(Words.losesCommits("origin/x", 2), "origin/x loses 2 commits.")
        }

        function test_the_name_is_seated_where_the_line_says_it() {
            const marks = Words.marksOf(name => Words.losesCommits(name, 3), [["feature/login", "branch"]])
            compare(marks.length, 1)
            compare(marks[0][0], 0)
            compare(marks[0][3], "feature/login".length)
        }
    }
}
