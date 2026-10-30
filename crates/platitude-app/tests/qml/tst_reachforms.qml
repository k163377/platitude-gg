import QtQuick
import QtTest

// The forms the reach rule judges (`waits::reach::bindings` reads each body out
// of this file and judges it as a beat and as a report), run by Qt: each
// `staged_` and `unread_` form changes `acts.ticks` through what it reaches,
// each `own_` form changes only what it made. The prefix is what the rule
// answers — a stage (a read in a report), an unread change, nothing.
TestCase {
    id: forms

    name: "ReachForms"

    function named(prefixes) {
        const rows = []
        for (const key in forms) {
            if (prefixes.some(prefix => key.startsWith(prefix)) && typeof forms[key] === "function")
                rows.push({ tag: key })
        }
        return rows
    }

    function staged_thisArg() {
        [0].forEach(function() { this.push(true) }, acts.ticks)
    }

    function staged_aBlockDoesNotHideTheOuterObject() {
        { const acts = []; acts.push(false) }
        acts.ticks.push(true)
    }

    function staged_anArrowDoesNotHideTheOuterObject() {
        [0].forEach(acts => acts)
        acts.ticks.push(true)
    }

    function staged_aNestedIndexedWrite() {
        const kept = [acts.ticks]
        kept[0][0] = true
    }

    function staged_anAssignmentWithAlternative() {
        let list = null
        list = list || acts.ticks
        list.push(true)
    }

    function staged_aCompoundIndexedWrite() {
        acts.ticks[0] |= 1
    }

    function staged_aReassignedCallbackParameter() {
        [[]].forEach(list => {
            list = acts.ticks
            list.push(true)
        })
    }

    function own_independentBlockBindings() {
        const list = []
        { const list = []; list.push(false) }
        list.push(true)
    }

    function staged_thisArgOverLines() {
        [0].forEach(function(value) {
            this.push(value)
        }, acts.ticks)
    }

    function staged_thisOfAnArrowInside() {
        [0].forEach(function() {
            const add = () => this.push(true)
            add()
        }, acts.ticks)
    }

    function staged_thisReturned() {
        const kept = [0].map(function() { return this }, acts.ticks)
        kept[0].push(true)
    }

    function staged_thisOfAFunctionHandedByName() {
        const add = function() { this.push(true) }
        const zero = [0]
        zero.forEach(add, acts.ticks)
    }

    function staged_thisOfADeclaredFunctionHandedByName() {
        function add() { this.push(true) }
        const zero = [0]
        zero.forEach(add, acts.ticks)
    }

    function staged_aDeclaredFunctionsParameterTakenApart() {
        function take([a]) { a.push(true) }
        const all = [[acts.ticks]]
        all.forEach(take)
    }

    function staged_aLocalNamedMath() {
        const Math = [acts.ticks]
        const a = Math.slice()
        a[0].push(true)
    }

    function staged_aLocalNamedArray() {
        const Array = [[acts.ticks]]
        Array.forEach(([a]) => a.push(true))
    }

    function staged_theGlobalsOwnContract() {
        const a = Object.values({ s: acts.ticks })
        a[0].push(true)
    }

    function staged_anElement() {
        const [a] = [acts.ticks]
        a.push(true)
    }

    function staged_anElementOfAnElement() {
        const [[a]] = [[acts.ticks]]
        a.push(true)
    }

    function staged_aFieldOfAField() {
        const {a: {b}} = {a: {b: acts.ticks}}
        b.push(true)
    }

    function staged_aDefault() {
        const [a = acts.ticks] = []
        a.push(true)
    }

    function staged_insideADefault() {
        const [[a] = [acts.ticks]] = []
        a.push(true)
    }

    function staged_aRest() {
        const [...rest] = [acts.ticks]
        rest[0].push(true)
    }

    function staged_aParameterTakenApartByName() {
        const take = ([a]) => a.push(true)
        const all = [[acts.ticks]]
        all.forEach(take)
    }

    function staged_aParameterTakenApart() {
        [[acts.ticks]].forEach(([a]) => a.push(true))
    }

    function staged_aParametersDefault() {
        [undefined].forEach((a = acts.ticks) => a.push(true))
    }

    function staged_aDefaultEvaluated() {
        const [x = acts.ticks.push(true)] = []
    }

    function staged_aFieldsDefaultEvaluated() {
        const {x = acts.ticks.push(true)} = {}
    }

    function staged_aComputedKeyEvaluated() {
        const {[acts.ticks.push(true)]: x} = {}
    }

    function staged_aParametersDefaultEvaluated() {
        function sample(x = acts.ticks.push(true)) { return x }
        sample()
    }

    function staged_aCallbacksDefaultEvaluated() {
        [undefined].forEach((x = acts.ticks.push(true)) => {})
    }

    function staged_aLengthInheritedFromAPrototype() {
        const b = Object.create({length: acts.ticks})
        b.length.push(true)
    }

    function staged_aFieldNamedLikeTheLanguagesOwn() {
        const b = {length: acts.ticks}
        b.length.push(true)
    }

    function unread_aLocalsOwnSlice() {
        const b = {slice: () => acts.ticks}
        const a = b.slice()
        a.push(true)
    }

    function unread_aLocalsOwnSplit() {
        const b = {split: () => acts.ticks}
        const a = b.split()
        a.push(true)
    }

    function unread_aLocalsOwnSliceSpelledAsAMethod() {
        const b = {slice() { return acts.ticks }}
        b.slice().push(true)
    }

    function unread_aLocalsOwnSliceInACallback() {
        [{slice: () => acts.ticks}].forEach(b => b.slice().push(true))
    }

    function unread_aLocalsOwnSliceAsADefault() {
        const [b = {slice: () => acts.ticks}] = []
        b.slice().push(true)
    }

    function unread_aFunctionsOwnSplitGivenByName() {
        const f = () => 0
        f.split = () => acts.ticks
        f.split().push(true)
    }

    function unread_aFunctionsOwnSplitGivenByAssign() {
        const f = () => 0
        Object.assign(f, {split: () => acts.ticks})
        f.split().push(true)
    }

    function unread_aSliceInheritedFromAPrototype() {
        const b = Object.create({slice: () => acts.ticks})
        const a = b.slice()
        a.push(true)
    }

    function unread_aSliceInheritedFromAPrototypeByName() {
        const p = {slice: () => acts.ticks}
        const b = Object.create(p)
        b.slice().push(true)
    }

    function unread_aSliceInheritedFromAPrototypeSetLater() {
        const b = []
        Object.setPrototypeOf(b, {slice: () => acts.ticks})
        b.slice().push(true)
    }

    function unread_aSliceInheritedThroughProto() {
        const b = []
        b.__proto__ = {slice: () => acts.ticks}
        b.slice().push(true)
    }

    function unread_aLengthInheritedThroughAHelper() {
        function make(p) { return Object.create(p) }
        make({length: acts.ticks}).length.push(true)
    }

    function unread_aLocalNamedObject() {
        const Object = {values: () => acts.ticks}; const a = Object.values({}); a.push(true)
    }

    function unread_aLocalNamedJSON() {
        const JSON = {parse: () => acts.ticks}
        JSON.parse("[]").push(true)
    }

    function unread_aFunctionAValueHolds() {
        const box = {take: () => acts.ticks}
        box.take().push(true)
    }

    function unread_thisOfAFunctionCalledToo() {
        const add = function() { this.push(true) }
        const zero = [0]
        zero.forEach(add, [])
        add.call(acts.ticks)
    }

    function unread_aParameterOfAFunctionCalledToo() {
        const take = ([a]) => a.push(1)
        const all = [[[]]]
        all.forEach(take)
        take([acts.ticks])
    }

    function own_aPureDefault() {
        const [x = 1] = []
        const a = []
        a.push(x)
    }

    function own_aPureComputedKey() {
        const {["k"]: x = []} = {}
        x.push(acts.ticks.length)
    }

    function own_theLanguagesSliceOfAnArray() {
        const a = [acts.ticks.length].slice()
        a.push(true)
    }

    function own_theLanguagesSplitOfAString() {
        const parts = "a:b".split(":")
        parts.push("c")
    }

    function own_theLanguagesSliceOfAHarnessArray() {
        const a = acts.ticks.slice()
        a.push(true)
    }

    function own_anObjectMadeFromNothing() {
        const b = Object.create(null)
        b.list = []
        b.list.push(acts.ticks.length)
    }

    function own_theLanguagesSliceOfAnArrayItMade() {
        const a = Array.from([acts.ticks.length]).slice()
        a.push(true)
    }

    function own_theLanguagesSliceOfJson() {
        const a = JSON.parse("[1]").slice()
        a.push(true)
    }

    function own_thisBoundToWhatTheCodeMade() {
        [0].forEach(function() { this.push(true) }, [])
    }

    function own_partsOfWhatTheCodeMade() {
        const [a, b] = [[], acts.ticks.length]
        a.push(b)
    }

    function own_aFieldOfWhatTheCodeMade() {
        const {a} = {a: []}
        a.push(acts.ticks.length)
    }

    function own_aParameterTakenApart() {
        [[[]]].forEach(([a]) => a.push(1))
    }

    function own_aDefaultTheCodeMade() {
        const [a = []] = []
        a.push(acts.ticks.length)
    }

    function own_aRestOfWhatTheCodeMade() {
        const [...rest] = [[]]
        rest[0].push(true)
    }

    function test_reaches_data() {
        return named(["staged_", "unread_"])
    }

    function test_reaches(data) {
        acts.ticks = []
        forms[data.tag]()
        verify(acts.ticks.length > 0, `${data.tag} changes acts.ticks`)
    }

    function test_own_data() {
        return named(["own_"])
    }

    function test_own(data) {
        acts.ticks = []
        forms[data.tag]()
        compare(JSON.stringify(acts.ticks), "[]", `${data.tag} changes only what it made`)
    }

    function test_every_form_is_a_row() {
        compare(named(["staged_", "unread_"]).length, 49)
        compare(named(["own_"]).length, 15)
    }

    Item {
        id: acts

        property var ticks: []
    }
}
