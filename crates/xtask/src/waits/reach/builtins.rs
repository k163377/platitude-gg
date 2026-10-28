//! What the language's own functions answer, and what they put inside the
//! values they are handed — each a contract the origin rule reads
//! (`super::origin`). A function the tables do not hold answers a value of
//! no known origin, and a change through that value is not passed as
//! safe.

/// What a method of the language's own values answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answers {
    /// A value that holds nothing: a number, a string, a boolean.
    Plain,
    /// A new container of plain values (`split`).
    NewPlain,
    /// A new container holding what the receiver holds (`slice`).
    NewInside,
    /// A new container holding what the receiver holds, each argument, and
    /// what each argument holds (`concat`).
    NewJoined,
    /// A new container of what a function it is handed answers: not known.
    NewUnknown,
    /// A value the receiver holds (`find`).
    Inside,
    /// The receiver itself (`sort`).
    Receiver,
}

/// The methods of arrays and strings, by what they answer.
pub(super) const METHODS: [(&str, Answers); 53] = [
    ("join", Answers::Plain),
    ("toString", Answers::Plain),
    ("valueOf", Answers::Plain),
    ("substring", Answers::Plain),
    ("substr", Answers::Plain),
    ("toLowerCase", Answers::Plain),
    ("toUpperCase", Answers::Plain),
    ("trim", Answers::Plain),
    ("trimStart", Answers::Plain),
    ("trimEnd", Answers::Plain),
    ("replace", Answers::Plain),
    ("replaceAll", Answers::Plain),
    ("padStart", Answers::Plain),
    ("padEnd", Answers::Plain),
    ("repeat", Answers::Plain),
    ("toFixed", Answers::Plain),
    ("toPrecision", Answers::Plain),
    ("indexOf", Answers::Plain),
    ("lastIndexOf", Answers::Plain),
    ("includes", Answers::Plain),
    ("some", Answers::Plain),
    ("every", Answers::Plain),
    ("findIndex", Answers::Plain),
    ("findLastIndex", Answers::Plain),
    ("localeCompare", Answers::Plain),
    ("charAt", Answers::Plain),
    ("charCodeAt", Answers::Plain),
    ("codePointAt", Answers::Plain),
    ("startsWith", Answers::Plain),
    ("endsWith", Answers::Plain),
    ("test", Answers::Plain),
    ("hasOwnProperty", Answers::Plain),
    ("forEach", Answers::Plain),
    // The length the receiver has after the put.
    ("push", Answers::Plain),
    ("unshift", Answers::Plain),
    ("split", Answers::NewPlain),
    ("match", Answers::NewPlain),
    ("exec", Answers::NewPlain),
    ("slice", Answers::NewInside),
    ("filter", Answers::NewInside),
    ("flat", Answers::NewInside),
    // What it took out.
    ("splice", Answers::NewInside),
    ("concat", Answers::NewJoined),
    ("map", Answers::NewUnknown),
    ("flatMap", Answers::NewUnknown),
    ("find", Answers::Inside),
    ("findLast", Answers::Inside),
    ("at", Answers::Inside),
    ("pop", Answers::Inside),
    ("shift", Answers::Inside),
    ("sort", Answers::Receiver),
    ("reverse", Answers::Receiver),
    ("fill", Answers::Receiver),
];

/// What the method `name` answers; none for one the table does not hold.
pub(super) fn method(name: &str) -> Option<Answers> {
    METHODS
        .iter()
        .find(|(method, _)| *method == name)
        .map(|(_, answers)| *answers)
}

/// The members of arrays and strings that are plain values.
pub(super) const PLAIN_MEMBERS: [&str; 1] = ["length"];

/// The methods that put their arguments inside their receiver, from the
/// argument at the index on (`splice` puts what follows its start and its
/// count). `fill` answers its receiver as well.
pub(super) const PUTS: [(&str, usize); 4] =
    [("push", 0), ("unshift", 0), ("splice", 2), ("fill", 0)];

/// The index of the first argument `name` puts inside its receiver, where
/// it puts any.
pub(super) fn puts(name: &str) -> Option<usize> {
    PUTS.iter()
        .find(|(method, _)| *method == name)
        .map(|(_, from)| *from)
}

/// What a function of a global object answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GlobalAnswers {
    Plain,
    /// A new container of plain values (`Object.keys`).
    NewPlain,
    /// A new container holding what its first argument holds
    /// (`Object.values`, `Array.from`).
    NewInsideOfFirst,
    /// A new container of its arguments (`Array.of`).
    NewOfArguments,
    /// A new container whose insides are new containers alike: a value read
    /// afresh from text (`JSON.parse`).
    NewDeep,
    /// Its first argument (`Object.assign`, `Object.freeze`).
    First,
}

/// What `object.method` answers for a global object; none for a function
/// the table does not hold.
pub(super) fn global(object: &str, method: &str) -> Option<GlobalAnswers> {
    Some(match (object, method) {
        ("Object", "keys") => GlobalAnswers::NewPlain,
        ("Object", "values" | "entries" | "create") | ("Array", "from") => {
            GlobalAnswers::NewInsideOfFirst
        }
        ("Object", "assign" | "freeze" | "seal" | "defineProperty" | "setPrototypeOf") => {
            GlobalAnswers::First
        }
        ("Object", "is") | ("Array", "isArray") | ("JSON", "stringify") | ("Date", "now") => {
            GlobalAnswers::Plain
        }
        ("Array", "of") => GlobalAnswers::NewOfArguments,
        ("JSON", "parse") => GlobalAnswers::NewDeep,
        ("Math" | "Number" | "String", _) => GlobalAnswers::Plain,
        // Values Qt makes: points, rectangles, colours, formatted text.
        (
            "Qt",
            "point" | "rect" | "size" | "rgba" | "hsla" | "hsva" | "color" | "tint" | "lighter",
        )
        | ("Qt", "darker" | "formatDate" | "formatTime" | "formatDateTime" | "md5" | "btoa")
        | ("Qt", "atob" | "fontFamilies" | "platform" | "resolvedUrl" | "url") => {
            GlobalAnswers::Plain
        }
        _ => return None,
    })
}

/// The global functions that put what their arguments hold, from the one at
/// the index on, inside their first argument.
pub(super) const GLOBAL_PUTS: [((&str, &str), usize); 3] = [
    (("Object", "assign"), 1),
    (("Object", "defineProperty"), 2),
    (("Object", "setPrototypeOf"), 1),
];

/// Where `object.method` puts what its arguments hold inside its first
/// argument: the index of the first it takes from.
pub(super) fn global_puts(object: &str, method: &str) -> Option<usize> {
    GLOBAL_PUTS
        .iter()
        .find(|((o, m), _)| *o == object && *m == method)
        .map(|(_, from)| *from)
}

/// The global functions, called bare, that answer a plain value.
pub(super) const PLAIN_FUNCTIONS: [&str; 11] = [
    "Number",
    "String",
    "Boolean",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "Date",
    "qsTr",
    "qsTranslate",
    "qsTrId",
];

/// The global functions, called bare, that answer a new value holding
/// nothing known.
pub(super) const NEW_FUNCTIONS: [&str; 6] =
    ["Array", "Error", "TypeError", "RangeError", "Map", "Set"];
