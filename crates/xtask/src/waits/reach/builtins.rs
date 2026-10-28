//! What the language's own functions answer, what they put inside the
//! values they are handed, and which functions they call back — each a
//! contract for a shape of call, by the number of its arguments, that the
//! origin rule reads (`super::origin`). A call the tables do not hold in
//! that shape — another number of arguments, a spread where the count
//! matters — answers a value of no known origin, and a change through that
//! value is not passed as safe.

/// How many arguments a contract holds for: at least, and at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Arity(pub usize, pub usize);

/// Any number, a spread included.
const ANY: Arity = Arity(0, usize::MAX);

impl Arity {
    /// Whether a call of `count` arguments — `spread` where one is spread,
    /// so the count is not known — is of this shape.
    fn holds(self, count: usize, spread: bool) -> bool {
        if spread {
            return self == ANY;
        }
        self.0 <= count && count <= self.1
    }
}

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
    /// A new container of what the function at the argument answers
    /// (`map`): its answers are read (`Program::callback_answers`).
    NewOfCallback(usize),
    /// A new container of what the function at the argument answers, one
    /// level flattened: an answer that is a container gives what it holds
    /// (`flatMap`).
    NewOfCallbackFlat(usize),
    /// A value the receiver holds (`find`).
    Inside,
    /// The receiver itself (`sort`).
    Receiver,
}

use Answers::{
    Inside, NewInside, NewJoined, NewOfCallback, NewOfCallbackFlat, NewPlain, Plain, Receiver,
};

/// The methods of arrays and strings: what each answers, called with how
/// many arguments.
const METHODS: [(&str, Arity, Answers); 53] = [
    ("join", Arity(0, 1), Plain),
    ("toString", Arity(0, 1), Plain),
    ("valueOf", Arity(0, 0), Plain),
    ("substring", Arity(1, 2), Plain),
    ("substr", Arity(1, 2), Plain),
    ("toLowerCase", Arity(0, 0), Plain),
    ("toUpperCase", Arity(0, 0), Plain),
    ("trim", Arity(0, 0), Plain),
    ("trimStart", Arity(0, 0), Plain),
    ("trimEnd", Arity(0, 0), Plain),
    ("replace", Arity(2, 2), Plain),
    ("replaceAll", Arity(2, 2), Plain),
    ("padStart", Arity(1, 2), Plain),
    ("padEnd", Arity(1, 2), Plain),
    ("repeat", Arity(1, 1), Plain),
    ("toFixed", Arity(0, 1), Plain),
    ("toPrecision", Arity(0, 1), Plain),
    ("indexOf", Arity(1, 2), Plain),
    ("lastIndexOf", Arity(1, 2), Plain),
    ("includes", Arity(1, 2), Plain),
    ("some", Arity(1, 2), Plain),
    ("every", Arity(1, 2), Plain),
    ("findIndex", Arity(1, 2), Plain),
    ("findLastIndex", Arity(1, 2), Plain),
    ("localeCompare", Arity(1, 3), Plain),
    ("charAt", Arity(0, 1), Plain),
    ("charCodeAt", Arity(0, 1), Plain),
    ("codePointAt", Arity(0, 1), Plain),
    ("startsWith", Arity(1, 2), Plain),
    ("endsWith", Arity(1, 2), Plain),
    ("test", Arity(1, 1), Plain),
    ("hasOwnProperty", Arity(1, 1), Plain),
    ("forEach", Arity(1, 2), Plain),
    // The length the receiver has after the put.
    ("push", ANY, Plain),
    ("unshift", ANY, Plain),
    ("split", Arity(0, 2), NewPlain),
    ("match", Arity(1, 1), NewPlain),
    ("exec", Arity(1, 1), NewPlain),
    ("slice", Arity(0, 2), NewInside),
    ("filter", Arity(1, 2), NewInside),
    ("flat", Arity(0, 1), NewInside),
    // What it took out.
    ("splice", ANY, NewInside),
    ("concat", ANY, NewJoined),
    ("map", Arity(1, 2), NewOfCallback(0)),
    ("flatMap", Arity(1, 2), NewOfCallbackFlat(0)),
    ("find", Arity(1, 2), Inside),
    ("findLast", Arity(1, 2), Inside),
    ("at", Arity(1, 1), Inside),
    ("pop", Arity(0, 0), Inside),
    ("shift", Arity(0, 0), Inside),
    ("sort", Arity(0, 1), Receiver),
    ("reverse", Arity(0, 0), Receiver),
    ("fill", Arity(1, 3), Receiver),
];

/// What the method `name` answers called with `count` arguments (`spread`
/// where one is spread); none for a shape the table does not hold.
pub(super) fn method(name: &str, count: usize, spread: bool) -> Option<Answers> {
    METHODS
        .iter()
        .find(|(method, arity, _)| *method == name && arity.holds(count, spread))
        .map(|(_, _, answers)| *answers)
}

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

/// What a function handed to a call is called with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Given {
    /// Each element of the values it walks — the receiver's, or the first
    /// argument's (`Array.from`) — and its index and the values whole,
    /// which the elements stand for.
    Elements,
    /// What nothing says: an accumulator (`reduce`), a key and value read
    /// back from text (`JSON.parse`).
    Unknown,
}

/// A function a call calls back: the argument it is at, what it is called
/// with, and the argument the call binds its `this` to where it takes one
/// (`thisArg` — a `function`'s own `this`; an arrow's is its maker's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CallsBack {
    pub at: usize,
    pub given: Given,
    pub this_at: Option<usize>,
}

const fn back(at: usize, given: Given, this_at: Option<usize>) -> CallsBack {
    CallsBack { at, given, this_at }
}

/// The methods that call a function they are handed, and how. What the
/// function does is the caller's.
const CALLS_BACK: [(&str, CallsBack); 15] = [
    ("map", back(0, Given::Elements, Some(1))),
    ("flatMap", back(0, Given::Elements, Some(1))),
    ("filter", back(0, Given::Elements, Some(1))),
    ("find", back(0, Given::Elements, Some(1))),
    ("findLast", back(0, Given::Elements, Some(1))),
    ("findIndex", back(0, Given::Elements, Some(1))),
    ("findLastIndex", back(0, Given::Elements, Some(1))),
    ("some", back(0, Given::Elements, Some(1))),
    ("every", back(0, Given::Elements, Some(1))),
    ("forEach", back(0, Given::Elements, Some(1))),
    ("sort", back(0, Given::Elements, None)),
    ("reduce", back(0, Given::Unknown, None)),
    ("reduceRight", back(0, Given::Unknown, None)),
    ("replace", back(1, Given::Unknown, None)),
    ("replaceAll", back(1, Given::Unknown, None)),
];

/// How `name` calls back a function it is handed, where it takes one.
pub(super) fn calls_back(name: &str) -> Option<CallsBack> {
    CALLS_BACK
        .iter()
        .find(|(method, _)| *method == name)
        .map(|(_, how)| *how)
}

/// Whether `name` is a method of arrays and strings the tables hold: a call
/// of any other on a value the code holds calls a function the value holds,
/// which nothing here follows.
pub(super) fn is_method(name: &str) -> bool {
    METHODS.iter().any(|(method, _, _)| *method == name)
        || puts(name).is_some()
        || calls_back(name).is_some()
}

/// What a new container a contract answers is, which says whose its
/// members are (`super::members`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Made {
    /// An array: the language's members.
    Array,
    /// What JSON text reads back as: arrays and objects of plain values and
    /// of each other, no function among them.
    Json,
    /// A language constructor's value (`new Map`, `Error(…)`).
    Constructed,
    /// An object the code spells: its members are its keys'.
    Object,
    /// A value whose members are another's it is made from
    /// (`Object.create`), or a constructor's nothing here knows: no member
    /// of it is known to be the language's.
    Borrowing,
}

/// The constructors whose values are the language's own.
pub(super) const CONSTRUCTED: [&str; 11] = [
    "Array",
    "Map",
    "Set",
    "Date",
    "RegExp",
    "Error",
    "TypeError",
    "RangeError",
    "Object",
    "String",
    "Number",
];

/// What a function of a global object answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GlobalAnswers {
    Plain,
    /// A new container of plain values (`Object.keys`).
    NewPlain,
    /// A new container holding what its first argument holds
    /// (`Object.values`, `Array.from`).
    NewInsideOfFirst,
    /// A new container of what the function at the argument answers for
    /// each element of the first argument (`Array.from(values, f)`).
    NewOfCallback(usize),
    /// A new container of its arguments (`Array.of`).
    NewOfArguments,
    /// A new container whose insides are new containers alike: a value read
    /// afresh from text (`JSON.parse`).
    NewDeep,
    /// Its first argument (`Object.assign`, `Object.freeze`).
    First,
}

/// What `object.method` answers for a global object called with `count`
/// arguments (`spread` where one is spread), and what a new container it
/// answers is; none for a function or a shape the table does not hold.
pub(super) fn global(
    object: &str,
    method: &str,
    count: usize,
    spread: bool,
) -> Option<(GlobalAnswers, Made)> {
    let made = match (object, method) {
        ("JSON", "parse") => Made::Json,
        // Its members are the prototype's it is handed.
        ("Object", "create") => Made::Borrowing,
        _ => Made::Array,
    };
    let (arity, answers) = match (object, method) {
        ("Object", "keys") => (Arity(1, 1), GlobalAnswers::NewPlain),
        ("Object", "values" | "entries" | "create") => {
            (Arity(1, 1), GlobalAnswers::NewInsideOfFirst)
        }
        ("Array", "from") if count >= 2 => (Arity(2, 3), GlobalAnswers::NewOfCallback(1)),
        ("Array", "from") => (Arity(1, 1), GlobalAnswers::NewInsideOfFirst),
        ("Object", "assign") => (Arity(1, usize::MAX), GlobalAnswers::First),
        ("Object", "freeze" | "seal") => (Arity(1, 1), GlobalAnswers::First),
        ("Object", "defineProperty") => (Arity(3, 3), GlobalAnswers::First),
        ("Object", "setPrototypeOf") => (Arity(2, 2), GlobalAnswers::First),
        ("Object", "is") => (Arity(2, 2), GlobalAnswers::Plain),
        ("Array", "isArray") => (Arity(1, 1), GlobalAnswers::Plain),
        ("JSON", "stringify") => (Arity(1, 3), GlobalAnswers::Plain),
        ("Date", "now") => (Arity(0, 0), GlobalAnswers::Plain),
        ("Array", "of") => (ANY, GlobalAnswers::NewOfArguments),
        ("JSON", "parse") => (Arity(1, 1), GlobalAnswers::NewDeep),
        ("Math" | "Number" | "String", _) => (ANY, GlobalAnswers::Plain),
        // Values Qt makes: points, rectangles, colours, formatted text.
        (
            "Qt",
            "point" | "rect" | "size" | "rgba" | "hsla" | "hsva" | "color" | "tint" | "lighter",
        )
        | ("Qt", "darker" | "formatDate" | "formatTime" | "formatDateTime" | "md5" | "btoa")
        | ("Qt", "atob" | "fontFamilies" | "platform" | "resolvedUrl" | "url") => {
            (ANY, GlobalAnswers::Plain)
        }
        _ => return None,
    };
    arity.holds(count, spread).then_some((answers, made))
}

/// How `object.method` calls back a function it is handed: with the
/// elements of its first argument, for `Array.from`.
pub(super) fn global_calls_back(object: &str, method: &str) -> Option<CallsBack> {
    match (object, method) {
        ("Array", "from") => Some(back(1, Given::Elements, Some(2))),
        ("JSON", "parse" | "stringify") => Some(back(1, Given::Unknown, None)),
        _ => None,
    }
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

/// The members of arrays and strings that are plain values.
pub(super) const PLAIN_MEMBERS: [&str; 1] = ["length"];
