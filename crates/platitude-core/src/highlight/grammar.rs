//! The grammars the fast path knows: which languages, how their capture
//! names map onto the app's palette, and one lazily built configuration
//! per language. A path no entry claims falls back to the regex lexer.
//!
//! An entry lists its grammar's queries as the grammar's manifest
//! (`tree-sitter.json`, where a key left out names `queries/<key>.scm`)
//! lists them, in its order: the highlights, the injections, and the
//! locals through which a reference takes its definition's highlight.
//! Where tree-sitter-highlight reads a query otherwise than the query was
//! written for, one rule here answers it for every entry: [`Order`] for
//! the precedence a highlights file was ordered for, [`current_names`] and
//! [`with_regex_matches`] for the editors' vocabulary, [`CAPTURES`] for the
//! names the palette recognises.

use std::sync::OnceLock;

use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration;

use super::Rgb;
use super::theme::{
    self, ANNOTATION, COMMENT, CONSTANT, FUNCTION, KEYWORD, PLAIN_RGB, STRING, TYPE,
};

/// The colour a run gets when no capture below claims it — the same
/// `textPrimary` the regex lexer falls back to.
pub(super) const PLAIN: Rgb = PLAIN_RGB;

/// Capture name → colour: デザイン規約 §シンタックスハイライト's table in
/// tree-sitter's capture vocabulary (the values live in `theme`, shared
/// with `theme::PALETTE`).
///
/// tree-sitter resolves a capture to the listed name with the most
/// dot-separated parts that all appear among the capture's own, in any
/// position; a tie goes to the earlier row. So `keyword` covers
/// `keyword.control`, the narrow `constant.builtin` stands over the broad
/// `constant`, and `tag.attribute` takes `attribute`'s colour.
///
/// The plain text's names have rows of their own, in `theme::PLAIN`: the
/// ones tree-sitter's default theme lists, and `label`, which it leaves
/// out (rust's lifetimes, C's and Lua's goto labels). A name with no row
/// is no highlight at all: a reference to a local can then not take its
/// definition's highlight and reads as a non-local (QML's `require` would
/// be a builtin function where it is used and plain where it is defined),
/// and a run takes the colour of the capture around it (a name
/// interpolated into a string would read as the string). The names left
/// without a row are the ones meant to read as their surroundings
/// (`every_capture_name_has_a_row_or_reads_as_its_surroundings`).
///
/// The rows speak the current generation of the editors' vocabulary, and
/// only the names a shipped query uses; a crate still on the older one is
/// renamed on the way in ([`current_names`]). A narrower row in
/// `theme::PLAIN` keeps a name off a broader row's colour: swift's
/// wildcard `_` (`character.special`) is not a character literal.
const CAPTURES: [(&str, u32); 24] = [
    ("comment", COMMENT),
    ("keyword", KEYWORD),
    ("type", TYPE),
    ("constructor", TYPE),
    ("function", FUNCTION),
    ("string", STRING),
    ("character", STRING),
    ("character.special", theme::PLAIN),
    ("number", CONSTANT),
    ("boolean", CONSTANT),
    ("constant.builtin", CONSTANT),
    ("attribute", ANNOTATION),
    ("tag", KEYWORD),
    ("markup.heading", KEYWORD),
    ("markup.raw", STRING),
    ("markup.link", KEYWORD),
    ("variable", theme::PLAIN),
    ("property", theme::PLAIN),
    ("constant", theme::PLAIN),
    ("module", theme::PLAIN),
    ("label", theme::PLAIN),
    ("operator", theme::PLAIN),
    ("punctuation", theme::PLAIN),
    ("embedded", theme::PLAIN),
];

/// The older capture names some crates still ship, each with the name
/// nvim-treesitter renamed it to (nvim-treesitter#5831); a name that
/// starts with one (`method.call`) keeps its tail. `storageclass` and
/// `type.qualifier` go where the current crates put `static` and
/// `final`: `keyword.modifier`.
const RENAMED: [(&str, &str); 18] = [
    ("conditional", "keyword.conditional"),
    ("repeat", "keyword.repeat"),
    ("include", "keyword.import"),
    ("exception", "keyword.exception"),
    ("storageclass", "keyword.modifier"),
    ("type.qualifier", "keyword.modifier"),
    ("preproc", "keyword.directive"),
    ("method", "function.method"),
    ("field", "variable.member"),
    ("parameter", "variable.parameter"),
    ("float", "number.float"),
    ("namespace", "module"),
    ("text.title", "markup.heading"),
    ("text.literal", "markup.raw"),
    ("text.uri", "markup.link.url"),
    ("text.reference", "markup.link"),
    ("text.emphasis", "markup.italic"),
    ("text.strong", "markup.strong"),
];

/// `query` with every capture in the older vocabulary ([`RENAMED`])
/// written in the current one, the predicates' references to it with it.
/// Strings (objc's `"@try"`) and comments are copied as they stand. A
/// crate that moves to the current names leaves this nothing to do.
fn current_names(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    let mut chars = query.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                while let Some((_, s)) = chars.next() {
                    out.push(s);
                    match s {
                        '\\' => {
                            if let Some((_, e)) = chars.next() {
                                out.push(e);
                            }
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            ';' => {
                out.push(c);
                while let Some(&(_, s)) = chars.peek() {
                    if s == '\n' {
                        break;
                    }
                    out.push(s);
                    chars.next();
                }
            }
            '@' => {
                let start = at + 1;
                let mut end = start;
                while let Some(&(i, s)) = chars.peek() {
                    if !(s.is_ascii_alphanumeric() || s == '_' || s == '.') {
                        break;
                    }
                    end = i + s.len_utf8();
                    chars.next();
                }
                let name = &query[start..end];
                let renamed = RENAMED.iter().find_map(|(old, new)| {
                    let tail = name.strip_prefix(old)?;
                    (tail.is_empty() || tail.starts_with('.')).then(|| format!("{new}{tail}"))
                });
                out.push('@');
                out.push_str(renamed.as_deref().unwrap_or(name));
            }
            _ => out.push(c),
        }
    }
    out
}

/// `query` with each `#lua-match?` the editors evaluate written as the
/// `#match?` tree-sitter-highlight evaluates, where the Lua pattern has a
/// regular expression of the same meaning ([`lua_as_regex`]). One without
/// stays, and [`configured`] turns its pattern off.
fn with_regex_matches(query: &str) -> String {
    static LUA_MATCH: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let Some(lua_match) = LUA_MATCH
        .get_or_init(|| {
            regex::Regex::new(r#"#(not-)?lua-match\?(\s+@[\w.]+\s+)"((?:[^"\\]|\\.)*)""#).ok()
        })
        .as_ref()
    else {
        return query.to_owned();
    };
    lua_match
        .replace_all(query, |found: &regex::Captures<'_>| {
            let not = found.get(1).map_or("", |m| m.as_str());
            match lua_as_regex(&unquoted(&found[3])) {
                Some(regex) => format!("#{not}match?{}\"{}\"", &found[2], quoted(&regex)),
                None => found[0].to_owned(),
            }
        })
        .into_owned()
}

/// A query string's body as the text it stands for: tree-sitter reads
/// `\n`, `\t`, `\r` and `\0` as those characters, and any other escaped
/// character as itself.
fn unquoted(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Text as a query string's body ([`unquoted`] undone).
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

/// A Lua pattern as the regular expression of the same meaning, or `None`
/// where this reading has none: `%b`, `%f`, a back reference, a class's
/// complement inside a set. Lua's classes are the C locale's.
fn lua_as_regex(lua: &str) -> Option<String> {
    let chars: Vec<char> = lua.chars().collect();
    let mut out = String::new();
    // Whether the last item can take a quantifier: a Lua quantifier
    // follows one character class, and is a literal anywhere else.
    let mut quantifiable = false;
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        i += 1;
        match c {
            '%' => {
                let letter = *chars.get(i)?;
                i += 1;
                match lua_class(letter) {
                    Some((body, complement)) => {
                        out.push_str(if complement { "[^" } else { "[" });
                        out.push_str(body);
                        out.push(']');
                    }
                    None if letter.is_ascii_alphanumeric() => return None,
                    None => out.push_str(&regex::escape(&letter.to_string())),
                }
                quantifiable = true;
            }
            '[' => {
                out.push('[');
                if chars.get(i) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                // A `]` first in a Lua set is one of its members.
                let mut first = true;
                // Whether the last member is one character, which a `-`
                // after it makes the start of a range. An escaped one is
                // not: Lua reads a range only from a bare character.
                let mut single = false;
                loop {
                    let member = *chars.get(i)?;
                    i += 1;
                    match member {
                        ']' if !first => break,
                        '%' => {
                            let letter = *chars.get(i)?;
                            i += 1;
                            match lua_class(letter) {
                                Some((body, false)) => {
                                    out.push_str(body);
                                    single = false;
                                }
                                Some((_, true)) => return None,
                                None if letter.is_ascii_alphanumeric() => return None,
                                None => {
                                    out.push_str(&in_set(letter));
                                    single = false;
                                }
                            }
                        }
                        '-' if single && chars.get(i).is_some_and(|&n| n != ']' && n != '%') => {
                            out.push('-');
                            single = false;
                        }
                        _ => {
                            out.push_str(&in_set(member));
                            single = true;
                        }
                    }
                    first = false;
                }
                out.push(']');
                quantifiable = true;
            }
            '.' => {
                out.push_str("(?s:.)");
                quantifiable = true;
            }
            '*' | '+' | '?' if quantifiable => {
                out.push(c);
                quantifiable = false;
            }
            '-' if quantifiable => {
                out.push_str("*?");
                quantifiable = false;
            }
            '^' if i == 1 => out.push('^'),
            '$' if i == chars.len() => out.push('$'),
            '(' | ')' => {
                out.push(c);
                quantifiable = false;
            }
            _ => {
                out.push_str(&regex::escape(&c.to_string()));
                quantifiable = true;
            }
        }
    }
    regex::Regex::new(&out).is_ok().then_some(out)
}

/// A Lua class letter's members as a set's body, and whether the letter
/// (in capitals) names the complement.
fn lua_class(letter: char) -> Option<(&'static str, bool)> {
    let body = match letter.to_ascii_lowercase() {
        'a' => "A-Za-z",
        'd' => "0-9",
        'l' => "a-z",
        'u' => "A-Z",
        'w' => "A-Za-z0-9",
        'x' => "0-9A-Fa-f",
        's' => "\\t\\n\\x0B\\x0C\\r ",
        'p' => "!-/:-@\\[-`{-~",
        'c' => "\\x00-\\x1F\\x7F",
        _ => return None,
    };
    Some((body, letter.is_ascii_uppercase()))
}

/// One character as a literal member of a regular expression's set.
fn in_set(c: char) -> String {
    if matches!(c, '\\' | ']' | '[' | '^' | '-' | '&' | '~') {
        format!("\\{c}")
    } else {
        c.to_string()
    }
}

/// The colour behind one highlight index — the same order
/// [`HighlightConfiguration::configure`] was given.
pub(super) fn style(index: usize) -> Option<Rgb> {
    let (_, rgb) = CAPTURES.get(index)?;
    Some(Rgb::of(*rgb))
}

/// The precedence a highlights file was ordered for. Where two patterns
/// capture one node, tree-sitter-highlight paints the later, and a file
/// may be ordered for a highlighter that paints the earlier: what its
/// author meant is told by the file's own highlight tests where it has
/// them, and by the commits that ordered it where it has none.
#[derive(Clone, Copy)]
enum Order {
    /// For the later pattern: read as written.
    LastWins,
    /// For the earlier pattern: read backwards.
    FirstWins,
    /// For the later pattern, but with catch-alls — a lone node captured
    /// with no context or predicate — after the patterns they back: those
    /// read first.
    CatchAllsAfter,
}

/// One highlights file and how it is read.
#[derive(Clone, Copy)]
struct Highlights {
    query: &'static str,
    order: Order,
    /// Whole patterns this repository leaves out, as the file writes them.
    without: &'static [&'static str],
}

const fn last_wins(query: &'static str) -> Highlights {
    Highlights {
        query,
        order: Order::LastWins,
        without: &[],
    }
}

const fn first_wins(query: &'static str) -> Highlights {
    Highlights {
        query,
        order: Order::FirstWins,
        without: &[],
    }
}

const fn catch_alls_after(query: &'static str) -> Highlights {
    Highlights {
        query,
        order: Order::CatchAllsAfter,
        without: &[],
    }
}

impl Highlights {
    const fn without(self, patterns: &'static [&'static str]) -> Self {
        Self {
            without: patterns,
            ..self
        }
    }

    /// The file's patterns in the order tree-sitter-highlight paints as
    /// the file means, the ones left out gone.
    fn read(&self) -> String {
        if matches!(self.order, Order::LastWins) && self.without.is_empty() {
            return self.query.to_owned();
        }
        let mut patterns: Vec<&str> = patterns(self.query)
            .into_iter()
            .filter(|pattern| !self.without.iter().any(|left| same_pattern(pattern, left)))
            .collect();
        match self.order {
            Order::LastWins => {}
            Order::FirstWins => patterns.reverse(),
            Order::CatchAllsAfter => patterns.sort_by_key(|pattern| !is_catch_all(pattern)),
        }
        patterns.join("\n")
    }
}

/// A query's patterns, each as written, split off its top level as
/// tree-sitter splits it: a node, a bracketed alternation, a string or
/// `_`, with the quantifier and captures after it; the comments and
/// spacing between patterns belong to none. Read off the text, for
/// asking tree-sitter compiles the query against its grammar, which costs
/// as much again as building the configuration
/// (ci/baseline/code-costs-windows-x64.md §着色);
/// `a_query_splits_into_the_patterns_tree_sitter_reads` holds the two
/// readings together.
fn patterns(query: &str) -> Vec<&str> {
    let mut out = Vec::new();
    // The pattern being read: where it starts, and where its last
    // character so far ends.
    let mut current: Option<(usize, usize)> = None;
    let mut depth = 0usize;
    let mut chars = query.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            ';' => while chars.next_if(|&(_, c)| c != '\n').is_some() {},
            '"' => {
                let mut end = at + 1;
                while let Some((inside, c)) = chars.next() {
                    end = inside + c.len_utf8();
                    match c {
                        '\\' => {
                            if let Some((escaped, c)) = chars.next() {
                                end = escaped + c.len_utf8();
                            }
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                if depth == 0 {
                    out.extend(
                        current
                            .replace((at, end))
                            .and_then(|(s, e)| query.get(s..e)),
                    );
                }
            }
            '(' | '[' => {
                if depth == 0 {
                    out.extend(
                        current
                            .replace((at, at + 1))
                            .and_then(|(s, e)| query.get(s..e)),
                    );
                }
                depth += 1;
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                if let (0, Some((_, end))) = (depth, current.as_mut()) {
                    *end = at + 1;
                }
            }
            _ if depth > 0 || c.is_whitespace() => {}
            '@' | '*' | '+' | '?' => {
                let mut end = at + 1;
                if c == '@' {
                    while let Some((name, c)) =
                        chars.next_if(|&(_, c)| c.is_alphanumeric() || "_-.?!".contains(c))
                    {
                        end = name + c.len_utf8();
                    }
                }
                if let Some((_, last)) = current.as_mut() {
                    *last = end;
                }
            }
            // `_`, the wildcard: a node of its own.
            _ => {
                let mut end = at + c.len_utf8();
                while let Some((word, c)) = chars.next_if(|&(_, c)| c.is_alphanumeric() || c == '_')
                {
                    end = word + c.len_utf8();
                }
                out.extend(
                    current
                        .replace((at, end))
                        .and_then(|(s, e)| query.get(s..e)),
                );
            }
        }
    }
    out.extend(current.and_then(|(s, e)| query.get(s..e)));
    out
}

/// Whether two pattern texts are one pattern, spacing aside.
fn same_pattern(a: &str, b: &str) -> bool {
    a.split_whitespace().eq(b.split_whitespace())
}

/// A lone node, or an alternation of lone nodes, captured with no context
/// or predicate: `(identifier) @variable`.
fn is_catch_all(pattern: &str) -> bool {
    static CATCH_ALL: OnceLock<Option<regex::Regex>> = OnceLock::new();
    CATCH_ALL
        .get_or_init(|| {
            regex::Regex::new(
                r"^\s*(\(\s*[\w/]+\s*\)|\[(\s*\(\s*[\w/]+\s*\))+\s*\])\s*(@[\w.]+\s*)+$",
            )
            .ok()
        })
        .as_ref()
        .is_some_and(|catch_all| catch_all.is_match(pattern))
}

/// One language the fast path can read.
pub(super) struct Lang {
    pub(super) config: HighlightConfiguration,
}

struct Entry {
    /// Extensions — and whole file names (`CMakeLists.txt`) — this
    /// language claims, lower-case.
    matches: &'static [&'static str],
    /// The name injections address this language by (```rust fences,
    /// markdown's own inline grammar).
    name: &'static str,
    language: fn() -> Language,
    /// In the order tree-sitter-highlight wants: where one file is a
    /// delta over another language's (qmljs over JavaScript's, objc over
    /// C's), the base first, for a base's catch-all
    /// (`(identifier) @variable`) put last would take back every node the
    /// specific language names. A manifest lists a delta's bases, but for
    /// objc's, which only its file names (`; inherits: c`); a base left
    /// out leaves a language's sample uncoloured
    /// (`every_grammar_colours_a_sample_of_its_language`).
    ///
    /// Where a crate's query names a node otherwise than デザイン規約
    /// §シンタックスハイライト would (C++'s casts as the template calls they
    /// parse as, SQL's NULL as a type word), the crate's naming stands. A
    /// query of this repository's own laid last adds only what a crate's
    /// query leaves out (C#'s `extension`, C's `true` and `false`) or names
    /// by the broad `constant`, which reads plain for it also marks every
    /// all-caps name (C's `NULL`, Elixir's and OCaml's booleans). Laid
    /// last, such a pattern also takes the node where the crate names it
    /// otherwise in one context, so the query names those contexts again
    /// in the crate's own words (Elixir's `@doc false`).
    highlights: &'static [Highlights],
    injections: &'static [&'static str],
    locals: &'static [&'static str],
    cell: OnceLock<Option<Lang>>,
}

/// Builds one entry's configuration, or `None` for a query the runtime
/// refuses — that language then reads through the fallback lexer.
fn configured(entry: &Entry) -> Option<HighlightConfiguration> {
    let language = (entry.language)();
    let highlights = highlights_of(entry);
    let injections = with_regex_matches(&entry.injections.join("\n"));
    let locals = with_regex_matches(&entry.locals.join("\n"));
    let mut config =
        HighlightConfiguration::new(language, entry.name, &highlights, &injections, &locals)
            .ok()?;
    // tree-sitter-highlight evaluates the text predicates (`#eq?`,
    // `#match?`, `#any-of?`) and the property ones, and ignores the rest —
    // the editors' own with no regular expression (`#has-ancestor?`) — so
    // a pattern carrying one would claim every node of its shape.
    for pattern in 0..config.query.pattern_count() {
        if !config.query.general_predicates(pattern).is_empty() {
            config.query.disable_pattern(pattern);
        }
    }
    // Spell-check marks share a node with its colour (`@comment @spell`),
    // and the later capture on a node is the one painted.
    for mark in ["spell", "nospell"] {
        config.query.disable_capture(mark);
    }
    let names: Vec<&str> = CAPTURES.iter().map(|(n, _)| *n).collect();
    config.configure(&names);
    Some(config)
}

/// An entry's highlights files, each read in its order, as one query in
/// the vocabulary `CAPTURES` speaks.
fn highlights_of(entry: &Entry) -> String {
    let files: Vec<String> = entry.highlights.iter().map(Highlights::read).collect();
    with_regex_matches(&current_names(&files.join("\n")))
}

macro_rules! entry {
    (
        $matches:expr, $name:literal, $language:expr,
        [$($highlights:expr),* $(,)?], [$($injections:expr),* $(,)?], [$($locals:expr),* $(,)?] $(,)?
    ) => {
        Entry {
            matches: $matches,
            name: $name,
            language: || $language.into(),
            highlights: &[$($highlights),*],
            injections: &[$($injections),*],
            locals: &[$($locals),*],
            cell: OnceLock::new(),
        }
    };
}

/// C's own constants, laid after C's query in every entry built on it.
const C_CONSTANTS: &str = include_str!("queries/c-highlights.scm");

#[rustfmt::skip]
static ENTRIES: [Entry; 37] = [
    // `(self) @variable.builtin` follows the `self` keywords of `use`
    // paths it backs.
    entry!(&["rs"], "rust", tree_sitter_rust::LANGUAGE, [catch_alls_after(tree_sitter_rust::HIGHLIGHTS_QUERY)], [tree_sitter_rust::INJECTIONS_QUERY], []),
    entry!(&["qml"], "qml", tree_sitter_qmljs::LANGUAGE, [last_wins(tree_sitter_javascript::HIGHLIGHT_QUERY), last_wins(tree_sitter_typescript::HIGHLIGHTS_QUERY), last_wins(tree_sitter_qmljs::HIGHLIGHTS_QUERY)], [], [tree_sitter_qmljs::LOCALS_QUERY, tree_sitter_typescript::LOCALS_QUERY, tree_sitter_javascript::LOCALS_QUERY]),
    entry!(&["kt", "kts"], "kotlin", tree_sitter_kotlin_ng::LANGUAGE, [last_wins(include_str!("queries/kotlin-highlights.scm"))], [], []),
    entry!(&["md", "markdown"], "markdown", tree_sitter_md::LANGUAGE, [last_wins(tree_sitter_md::HIGHLIGHT_QUERY_BLOCK)], [include_str!("queries/markdown-injections.scm")], []),
    entry!(&[], "markdown_inline", tree_sitter_md::INLINE_LANGUAGE, [last_wins(tree_sitter_md::HIGHLIGHT_QUERY_INLINE)], [include_str!("queries/markdown_inline-injections.scm")], []),
    entry!(&["toml"], "toml", tree_sitter_toml_ng::LANGUAGE, [last_wins(tree_sitter_toml_ng::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["yaml", "yml"], "yaml", tree_sitter_yaml::LANGUAGE, [last_wins(tree_sitter_yaml::HIGHLIGHTS_QUERY)], [], []),
    // A key's pattern precedes the string's.
    entry!(&["json", "jsonc"], "json", tree_sitter_json::LANGUAGE, [first_wins(tree_sitter_json::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["js", "mjs", "cjs", "jsx"], "javascript", tree_sitter_javascript::LANGUAGE, [last_wins(tree_sitter_javascript::HIGHLIGHT_QUERY), last_wins(tree_sitter_javascript::JSX_HIGHLIGHT_QUERY), last_wins(include_str!("queries/javascript-highlights-params.scm"))], [tree_sitter_javascript::INJECTIONS_QUERY], [tree_sitter_javascript::LOCALS_QUERY]),
    // The manifest lists typescript's own file before javascript's, the
    // order for the earlier pattern.
    entry!(&["ts", "mts", "cts"], "typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT, [last_wins(tree_sitter_javascript::HIGHLIGHT_QUERY), last_wins(tree_sitter_typescript::HIGHLIGHTS_QUERY)], [tree_sitter_javascript::INJECTIONS_QUERY], [tree_sitter_typescript::LOCALS_QUERY, tree_sitter_javascript::LOCALS_QUERY]),
    // As typescript's: the manifest lists tsx's own file, jsx's and
    // javascript's, in that order.
    entry!(&["tsx"], "tsx", tree_sitter_typescript::LANGUAGE_TSX, [last_wins(tree_sitter_javascript::HIGHLIGHT_QUERY), last_wins(tree_sitter_javascript::JSX_HIGHLIGHT_QUERY), last_wins(tree_sitter_typescript::HIGHLIGHTS_QUERY)], [tree_sitter_javascript::INJECTIONS_QUERY], [tree_sitter_javascript::LOCALS_QUERY]),
    entry!(&["py", "pyi"], "python", tree_sitter_python::LANGUAGE, [last_wins(tree_sitter_python::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["java"], "java", tree_sitter_java::LANGUAGE, [last_wins(tree_sitter_java::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["c", "h"], "c", tree_sitter_c::LANGUAGE, [last_wins(tree_sitter_c::HIGHLIGHT_QUERY), last_wins(C_CONSTANTS)], [], []),
    // The manifest's one injection names a raw string's language by its
    // delimiter, a capture before the content's: tree-sitter-highlight
    // takes an injection at its first capture, as the match stands then,
    // so the content is never in it and the string stays a string.
    entry!(&["cpp", "cc", "cxx", "hpp", "hh", "hxx"], "cpp", tree_sitter_cpp::LANGUAGE, [last_wins(tree_sitter_c::HIGHLIGHT_QUERY), last_wins(tree_sitter_cpp::HIGHLIGHT_QUERY), last_wins(C_CONSTANTS)], [], []),
    entry!(&["cs"], "c_sharp", tree_sitter_c_sharp::LANGUAGE, [last_wins(tree_sitter_c_sharp::HIGHLIGHTS_QUERY), last_wins(include_str!("queries/c_sharp-highlights.scm"))], [], []),
    // The catch-alls follow the calls and declarations they back.
    entry!(&["go"], "go", tree_sitter_go::LANGUAGE, [first_wins(tree_sitter_go::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["rb"], "ruby", tree_sitter_ruby::LANGUAGE, [last_wins(tree_sitter_ruby::HIGHLIGHTS_QUERY)], [], [tree_sitter_ruby::LOCALS_QUERY]),
    entry!(&["php"], "php", tree_sitter_php::LANGUAGE_PHP, [last_wins(tree_sitter_php::HIGHLIGHTS_QUERY)], [tree_sitter_php::INJECTIONS_QUERY, include_str!("queries/php-injections-text.scm")], []),
    entry!(&["swift"], "swift", tree_sitter_swift::LANGUAGE, [last_wins(tree_sitter_swift::HIGHLIGHTS_QUERY)], [tree_sitter_swift::INJECTIONS_QUERY], [tree_sitter_swift::LOCALS_QUERY]),
    entry!(&["html", "htm"], "html", tree_sitter_html::LANGUAGE, [last_wins(tree_sitter_html::HIGHLIGHTS_QUERY)], [tree_sitter_html::INJECTIONS_QUERY], []),
    entry!(&["css"], "css", tree_sitter_css::LANGUAGE, [last_wins(tree_sitter_css::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["sh", "bash", "zsh"], "bash", tree_sitter_bash::LANGUAGE, [last_wins(tree_sitter_bash::HIGHLIGHT_QUERY)], [], []),
    entry!(&["scala", "sbt"], "scala", tree_sitter_scala::LANGUAGE, [last_wins(tree_sitter_scala::HIGHLIGHTS_QUERY)], [], [tree_sitter_scala::LOCALS_QUERY]),
    // `(variable) @type`, among the types and after every variable
    // pattern, would make every variable a type.
    entry!(&["hs"], "haskell", tree_sitter_haskell::LANGUAGE, [last_wins(tree_sitter_haskell::HIGHLIGHTS_QUERY).without(&["(variable) @type"])], [tree_sitter_haskell::INJECTIONS_QUERY], [tree_sitter_haskell::LOCALS_QUERY]),
    entry!(&["lua"], "lua", tree_sitter_lua::LANGUAGE, [last_wins(tree_sitter_lua::HIGHLIGHTS_QUERY)], [tree_sitter_lua::INJECTIONS_QUERY], [tree_sitter_lua::LOCALS_QUERY]),
    entry!(&["zig"], "zig", tree_sitter_zig::LANGUAGE, [last_wins(tree_sitter_zig::HIGHLIGHTS_QUERY)], [tree_sitter_zig::INJECTIONS_QUERY], []),
    entry!(&["ex", "exs"], "elixir", tree_sitter_elixir::LANGUAGE, [last_wins(tree_sitter_elixir::HIGHLIGHTS_QUERY), last_wins(include_str!("queries/elixir-highlights.scm"))], [tree_sitter_elixir::INJECTIONS_QUERY], []),
    // `.mli` stays with the fallback lexer: the crate's one highlight
    // query names nodes the interface grammar does not have.
    entry!(&["ml"], "ocaml", tree_sitter_ocaml::LANGUAGE_OCAML, [last_wins(tree_sitter_ocaml::HIGHLIGHTS_QUERY), last_wins(include_str!("queries/ocaml-highlights.scm"))], [], [tree_sitter_ocaml::LOCALS_QUERY]),
    // objc's query is a delta over C's (`; inherits: c`), which its
    // manifest does not list.
    entry!(&["m", "mm"], "objc", tree_sitter_objc::LANGUAGE, [last_wins(tree_sitter_c::HIGHLIGHT_QUERY), last_wins(tree_sitter_objc::HIGHLIGHTS_QUERY), last_wins(C_CONSTANTS)], [tree_sitter_objc::INJECTIONS_QUERY], [tree_sitter_objc::LOCALS_QUERY]),
    // Ordered as its own highlight tests expect; its locals.scm holds no
    // pattern.
    entry!(&["nix"], "nix", tree_sitter_nix::LANGUAGE, [first_wins(tree_sitter_nix::HIGHLIGHTS_QUERY)], [tree_sitter_nix::INJECTIONS_QUERY], []),
    entry!(&["sql"], "sql", tree_sitter_sequel::LANGUAGE, [last_wins(tree_sitter_sequel::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["diff", "patch"], "diff", tree_sitter_diff::LANGUAGE, [last_wins(tree_sitter_diff::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["ps1", "psm1", "psd1"], "powershell", tree_sitter_powershell::LANGUAGE, [last_wins(tree_sitter_powershell::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["properties"], "properties", tree_sitter_properties::LANGUAGE, [last_wins(tree_sitter_properties::HIGHLIGHTS_QUERY)], [], []),
    entry!(&["cmake", "cmakelists.txt"], "cmake", tree_sitter_cmake::LANGUAGE, [last_wins(tree_sitter_cmake::HIGHLIGHTS_QUERY)], [tree_sitter_cmake::INJECTIONS_QUERY], []),
    entry!(&["xml", "svg", "xsd", "plist"], "xml", tree_sitter_xml::LANGUAGE_XML, [last_wins(tree_sitter_xml::XML_HIGHLIGHT_QUERY)], [], []),
];

/// The grammar for a path, `None` where no entry claims it. An entry
/// matches by extension, or — for the entries that carry a dot
/// themselves (`cmakelists.txt`) — by the whole file name. A name with
/// no extension matches nothing: a wrapper script called `go` is not Go.
pub(super) fn for_path(path: &str) -> Option<&'static Lang> {
    entry_for(path).and_then(lang_of)
}

/// Whether an entry claims this path, without building anything.
pub(super) fn claims(path: &str) -> bool {
    entry_for(path).is_some()
}

fn entry_for(path: &str) -> Option<&'static Entry> {
    // git's paths use `/` alone; the fallback set splits the same way.
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e);
    ENTRIES.iter().find(|e| {
        e.matches.iter().any(|m| {
            ext.is_some_and(|ext| m.eq_ignore_ascii_case(ext))
                || (m.contains('.') && m.eq_ignore_ascii_case(name))
        })
    })
}

/// The configuration an injection names — markdown's inline grammar,
/// the language of a fenced code block. A name is a language's own
/// (`javascript`) or one of the extensions it claims (`js`), in any case,
/// as a fence's info string names it. Unknown names read as plain text,
/// which is what an unhighlightable fence should look like.
pub(super) fn injection(name: &str) -> Option<&'static HighlightConfiguration> {
    ENTRIES
        .iter()
        .find(|e| {
            e.name.eq_ignore_ascii_case(name)
                || e.matches.iter().any(|m| m.eq_ignore_ascii_case(name))
        })
        .and_then(lang_of)
        .map(|lang| &lang.config)
}

fn lang_of(entry: &'static Entry) -> Option<&'static Lang> {
    entry
        .cell
        .get_or_init(|| configured(entry).map(|config| Lang { config }))
        .as_ref()
}

#[cfg(test)]
mod tests;
