import QtQuick
import QtTest
import platitude.ui

// What a name box says of a name that cannot stand beside the branches or tags already there
// (`Words.nameClashed`, from what `NavSectionModel.nameClash` answered): each kind of clash is one sentence naming
// which of the two would have to be the folder, a tag's and a branch's differ in their noun, and nothing in the
// way is nothing said — the empty sentence is what lets the box take the name (デザイン規約 §左メニューの所作「名前の衝突」).
Item {
    TestCase {
        name: "NameWords"

        function clash(kind, withName) {
            return { kind: kind, with: withName === undefined ? "" : withName }
        }

        function test_each_kind_of_clash_says_its_own_sentence_data() {
            return [
                { tag: "taken branch", clash: clash("taken"), isTag: false,
                  said: "A branch called that already exists" },
                { tag: "taken tag", clash: clash("taken"), isTag: true,
                  said: "A tag called that already exists" },
                { tag: "holds branch", clash: clash("holds", "feature/topic-a"), isTag: false,
                  said: "feature/topic-a already uses this name as a folder" },
                { tag: "holds tag", clash: clash("holds", "v1.0/rc"), isTag: true,
                  said: "v1.0/rc already uses this name as a folder" },
                { tag: "under branch", clash: clash("under", "feature/topic-a"), isTag: false,
                  said: "feature/topic-a is already a branch and cannot also be a folder" },
                { tag: "under tag", clash: clash("under", "v1.0"), isTag: true,
                  said: "v1.0 is already a tag and cannot also be a folder" },
                { tag: "case branch", clash: clash("case", "Feature/Topic-A"), isTag: false,
                  said: "Feature/Topic-A differs only in letter case — some disks take it for the same name" },
                { tag: "case tag", clash: clash("case", "V1.0"), isTag: true,
                  said: "V1.0 differs only in letter case — some disks take it for the same name" },
            ]
        }
        function test_each_kind_of_clash_says_its_own_sentence(data) {
            compare(Words.nameClashed(data.clash, data.isTag), data.said)
        }

        /// Nothing in the way, and a kind the model does not answer with, say nothing: the box takes the name.
        function test_a_name_with_nothing_in_its_way_says_nothing() {
            compare(Words.nameClashed(undefined, false), "")
            compare(Words.nameClashed(undefined, true), "")
            compare(Words.nameClashed(clash("elsewhere", "x"), false), "")
        }
    }
}
