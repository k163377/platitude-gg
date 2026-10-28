//! What a name stands for where it is spelled — a local named as a global,
//! `this` in a function a call calls back, the names a pattern takes apart,
//! a function's parameters, a function handed by its name — judged alike
//! in a beat and a report, and the forms Qt runs (`tst_reachforms.qml`)
//! judged as they stand there.

use super::tests::{driver, found, in_report, indented, rules, verbs};

/// `this` stands for what it is bound to where it is spelled: in a
/// `function` a call calls back, the `this` that call hands it (`thisArg`) —
/// through an arrow inside it too, whose `this` is its maker's; in a QML
/// function, nothing known. A change through it is judged by that, in a beat
/// and a report alike, the forms Qt runs (the reviewer's, on one line).
#[test]
fn a_change_through_this_is_judged_by_what_the_call_binds_it_to() {
    let members = "    property var ticks: []";
    let forms = [
        "[0].forEach(function() { this.push(true) }, acts.ticks)",
        "[0].forEach(function(value) {\n    this.push(value)\n}, acts.ticks)",
        "[0].forEach(function() {\n    const add = () => this.push(true)\n    add()\n}, acts.ticks)",
        "const kept = [0].map(function() { return this }, acts.ticks)\nkept[0].push(true)",
        // Handed by a local's name, the function is the one spelled.
        "const add = function() { this.push(true) }\nconst zero = [0]\nzero.forEach(add, acts.ticks)",
    ];
    for form in forms {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert_eq!(
            rules(&beat),
            ["stage"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert_eq!(
            rules(&report),
            ["read"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
    let unbound = found(
        &driver(""),
        &verbs(
            members,
            "            [0].forEach(function() { this.push(true) })",
        ),
    );
    assert_eq!(
        rules(&unbound),
        ["unread"].into_iter().collect(),
        "{unbound:?}"
    );
    let own = found(
        &driver(""),
        &verbs(
            members,
            "            [0].forEach(function() { this.push(true) }, [])",
        ),
    );
    assert!(own.is_empty(), "a `this` the code made is its own: {own:?}");
    // A function that goes anywhere else too is bound to what nothing here says.
    let elsewhere = found(
        &driver(""),
        &verbs(
            members,
            "            const add = function() { this.push(true) }\n            const zero = [0]\n            zero.forEach(add, [])\n            add.call(acts.ticks)",
        ),
    );
    assert!(rules(&elsewhere).contains("unread"), "{elsewhere:?}");
}

/// A name is what its binding is where it is spelled: a local named as a
/// global (`Object`, `JSON`) is that local, and no contract of the
/// global's answers for it; a function a value holds, called, is followed by
/// nothing here, in a beat and a report alike.
#[test]
fn a_local_named_as_a_global_is_the_local() {
    let members = "    property var ticks: []";
    for form in [
        "const Object = {values: () => acts.ticks}; const a = Object.values({}); a.push(true)",
        "const JSON = {parse: () => acts.ticks}\nJSON.parse(\"[]\").push(true)",
        "const box = {take: () => acts.ticks}\nbox.take().push(true)",
    ] {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert_eq!(
            rules(&beat),
            ["unread"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert_eq!(
            rules(&report),
            ["unread"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
    // A local named as a global answers by what it is: its methods' contracts, not the global's.
    for form in [
        "const Math = [acts.ticks]\nconst a = Math.slice()\na[0].push(true)",
        "const Array = [[acts.ticks]]\nArray.forEach(([a]) => a.push(true))",
    ] {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert_eq!(
            rules(&beat),
            ["stage"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert_eq!(
            rules(&report),
            ["read"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
    let real = found(
        &driver(""),
        &verbs(
            members,
            "            const a = Object.values({ s: acts.ticks })\n            a[0].push(true)",
        ),
    );
    assert_eq!(
        rules(&real),
        ["stage"].into_iter().collect(),
        "the global's own contract: {real:?}"
    );
}

#[test]
fn a_name_a_pattern_binds_is_what_the_value_holds_there() {
    let members = "    property var ticks: []";
    for form in [
        "const [a] = [acts.ticks]\na.push(true)",
        "const [[a]] = [[acts.ticks]]\na.push(true)",
        "const {a: {b}} = {a: {b: acts.ticks}}\nb.push(true)",
        "const [a = acts.ticks] = []\na.push(true)",
        "const [[a] = [acts.ticks]] = []\na.push(true)",
        "const [...rest] = acts.ticks\nrest[0] = true",
        "const take = ([a]) => a.push(true)\nconst all = [[acts.ticks]]\nall.forEach(take)",
        "[[acts.ticks]].forEach(([a]) => a.push(true))",
        "[[]].forEach((a = acts.ticks) => a.push(true))",
    ] {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert!(
            rules(&beat) == ["stage"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert!(
            rules(&report) == ["read"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
    // Taken apart, a local's own parts are its own.
    for form in [
        "const [a, b] = [[], acts.ticks.length]\na.push(b)",
        "const {a} = {a: []}\na.push(acts.ticks.length)",
        "[[[]]].forEach(([a]) => a.push(1))",
    ] {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert!(beat.is_empty(), "beat {form}: {beat:?}");
    }
    // A function handed on and called too is handed what nothing here says.
    let elsewhere = found(
        &driver(""),
        &verbs(
            members,
            "            const take = ([a]) => a.push(1)\n            const all = [[[]]]\n            all.forEach(take)\n            take([acts.ticks])",
        ),
    );
    assert!(rules(&elsewhere).contains("unread"), "{elsewhere:?}");
}

#[test]
fn a_pattern_binds_its_names_by_where_they_stand() {
    let tokens = |text: &str| crate::lex::tokens(text, crate::lex::Lang::Qml);
    let names = |text: &str| -> Vec<(String, Option<usize>, Vec<usize>)> {
        let tokens = tokens(text);
        super::parse::pattern_names(&tokens, 0)
            .bound
            .into_iter()
            .map(|b| {
                let defaults = b.defaults.iter().map(|(_, inside)| *inside).collect();
                (b.name, b.depth, defaults)
            })
            .collect()
    };
    let n = |name: &str, depth: Option<usize>, defaults: &[usize]| {
        (name.to_string(), depth, defaults.to_vec())
    };
    assert_eq!(
        names("a: int, b = c.d, ...rest"),
        [
            n("a", Some(0), &[]),
            n("b", Some(0), &[0]),
            n("rest", None, &[])
        ]
    );
    assert_eq!(
        names("[a, {b, c: [d], [e]: f}, ...g]"),
        [
            n("a", Some(1), &[]),
            n("b", Some(2), &[]),
            n("d", Some(3), &[]),
            n("f", Some(2), &[]),
            n("g", Some(0), &[]),
        ]
    );
    assert_eq!(
        names("[[a, b = x] = [y], c]"),
        [
            n("a", Some(2), &[1]),
            n("b", Some(2), &[0, 1]),
            n("c", Some(1), &[])
        ]
    );
}

/// A function's parameters are bound argument by argument: one taken apart
/// is what its argument holds there, whichever argument stands after it,
/// and a default joins what callers hand.
#[test]
fn a_functions_parameters_are_bound_from_their_own_arguments() {
    let members = "    property var ticks: []\n    function both([a, b], c) {\n        b.push(c)\n    }\n    \
                   function last([a, b], c) {\n        c.push(true)\n    }\n    \
                   function keep(a = acts.ticks) {\n        a.push(true)\n    }\n    \
                   function sample(x = acts.ticks.push(true)) {\n        return x\n    }\n    \
                   function apart({[acts.ticks.push(true)]: k}) {\n        return k\n    }";
    for form in [
        "acts.both([[], acts.ticks], [])",
        "acts.last([[], []], acts.ticks)",
        "acts.keep()",
        // What a parameter list evaluates as it binds is the function's code.
        "acts.sample()",
        "acts.apart({})",
    ] {
        let beat = found(
            &driver(""),
            &verbs(members, &indented(form, "            ")),
        );
        assert_eq!(
            rules(&beat),
            ["stage"].into_iter().collect(),
            "beat {form}: {beat:?}"
        );
        let report = found(&driver(""), &verbs(members, &in_report(form, "\"x\"")));
        assert_eq!(
            rules(&report),
            ["read"].into_iter().collect(),
            "report {form}: {report:?}"
        );
    }
    let own = found(
        &driver(""),
        &verbs(
            members,
            "            acts.both([[], []], acts.ticks.length)",
        ),
    );
    assert!(own.is_empty(), "{own:?}");
    let tokens = crate::lex::tokens("[a, b], c = d, ...rest", crate::lex::Lang::Qml);
    let super::parse::UnitParams {
        params,
        bound,
        evaluated,
    } = super::parse::unit_params(&tokens, 0);
    assert_eq!((evaluated.len(), evaluated.first()), (1, Some(&(8..9))));
    assert_eq!(params, ["", "c", ""]);
    let bound: Vec<_> = bound
        .iter()
        .map(|(arg, b)| (*arg, b.name.as_str(), b.depth, b.defaults.len()))
        .collect();
    assert_eq!(
        bound,
        [
            (0, "a", Some(1), 0),
            (0, "b", Some(1), 0),
            (1, "c", Some(0), 1),
            (2, "rest", None, 0)
        ]
    );
}

/// The forms Qt runs (`tst_reachforms.qml`, where each `staged_` and
/// `unread_` form changes `acts.ticks` and each `own_` form changes only what
/// it made), judged as they stand there: by their prefix, in a beat and a
/// report alike.
#[test]
fn the_forms_qt_runs_are_judged_by_what_they_change() {
    let file = include_str!("../../../../platitude-app/tests/qml/tst_reachforms.qml");
    let lines: Vec<&str> = file.lines().collect();
    let mut judged = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some(name) = line
            .strip_prefix("    function ")
            .and_then(|rest| rest.strip_suffix("() {"))
        else {
            continue;
        };
        let Some(prefix) = ["staged_", "unread_", "own_"]
            .into_iter()
            .find(|prefix| name.starts_with(prefix))
        else {
            continue;
        };
        let end = lines[at..]
            .iter()
            .position(|line| *line == "    }")
            .expect("each form closes at its own indent");
        let body = lines[at + 1..at + end]
            .iter()
            .map(|line| line.strip_prefix("        ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");
        judged.push((name, prefix, body));
    }
    let count = |prefix: &str| judged.iter().filter(|(_, p, _)| *p == prefix).count();
    assert_eq!(
        (count("staged_") + count("unread_"), count("own_")),
        (43, 14),
        "the forms tst_reachforms counts"
    );
    let members = "    property var ticks: []";
    for (name, prefix, body) in &judged {
        let (beat, report): (&[&str], &[&str]) = match *prefix {
            "staged_" => (&["stage"], &["read"]),
            "unread_" => (&["unread"], &["unread"]),
            _ => (&[], &[]),
        };
        let in_beat = found(
            &driver(""),
            &verbs(members, &indented(body, "            ")),
        );
        assert_eq!(
            rules(&in_beat),
            beat.iter().copied().collect(),
            "beat {name}: {in_beat:?}"
        );
        let in_report = found(&driver(""), &verbs(members, &in_report(body, "\"x\"")));
        assert_eq!(
            rules(&in_report),
            report.iter().copied().collect(),
            "report {name}: {in_report:?}"
        );
    }
}
