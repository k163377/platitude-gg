//! A paragraph's joined text cut at the places a line may end.
//!
//! A line end of the source, and a space, stand between the characters
//! drawn on either side of them, which are not always the ones beside them
//! in the source: emphasis delimiters, a link's brackets and destination
//! and a code span's backticks are not drawn, and CSS judges adjacency
//! across inline boxes (CSS Text, §1.5). Where the drawn pair joins tight
//! (`width::joins_tight`) a line end is removed and a space is drawn; any
//! other line end draws as a space. So a line end of the source goes
//! between such a pair and is a space anywhere else, and a line may end at
//! a space only where the pair does not join tight. Where the pair cannot
//! be told — a delimiter that may be literal, a tag, a bracket, an entity —
//! the source's line end stays where it is ([`Piece::Keep`]) and a space is
//! no place to end a line.
//!
//! Taken whole: code spans (a line end in one draws as a space), tags,
//! comments and autolinks, links (a name split across two lines reads as
//! two words), bracketed text (a reference label must keep its spelling)
//! and bare URLs (a URL ends where a line would).

use super::width::{breaks_between, joins_tight};

/// The paragraph as text between the places a line may end.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Piece {
    Text(String),
    /// Spaces: kept as they are between two pieces on a line, dropped
    /// where the line ends.
    Space(String),
    /// A place between two Japanese characters, which takes nothing.
    Tight,
    /// The source's own line end, kept where it stands.
    Keep,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Code,
    /// `[text](destination)` as CommonMark reads one; `text_end` is the `]`.
    Link {
        text_end: usize,
    },
    /// An inline image, `![alt](source)`: drawn as an object, which no
    /// line end beside joins tight (CSS draws an atomic inline as U+FFFC).
    Image,
    /// Any other bracketed text, and an image by reference.
    Bracket,
    /// A tag or a comment.
    Html,
    /// `<scheme:…>` or `<name@host>`, which draws its inside.
    Autolink,
    Url,
    Escape,
}

#[derive(Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
    kind: Kind,
}

/// What an image draws as, beside a line end.
const OBJECT: char = '\u{FFFC}';

/// The characters drawn either side of a place.
enum Drawn {
    /// The character, and whether a delimiter stood between it and the place.
    Char(char, bool),
    Unsure,
}

/// How a line end at a place between two drawn characters is drawn.
enum Between {
    /// Removed.
    Tight,
    /// As a space.
    Loose,
    Unsure,
}

/// The joined text cut into pieces, or `None` where a span taken whole
/// holds a line end that joining it would change.
pub(super) fn pieces(text: &str) -> Option<Vec<Piece>> {
    let chars: Vec<char> = text.chars().collect();
    let spans = spans(&chars, 0, chars.len());
    let mut out = Vec::new();
    let mut word = String::new();
    let mut next_span = 0;
    let mut at = 0;
    while at < chars.len() {
        if let Some(span) = spans.get(next_span).filter(|span| span.start == at) {
            word.push_str(&spelled(&chars, span)?);
            at = span.end;
            next_span += 1;
            continue;
        }
        let c = chars[at];
        if c != ' ' && c != '\n' {
            if let Some(before) = word.chars().last()
                && breaks_between(before, c)
            {
                out.push(Piece::Text(std::mem::take(&mut word)));
                out.push(Piece::Tight);
            }
            word.push(c);
            at += 1;
            continue;
        }
        let start = at;
        while at < chars.len() && matches!(chars[at], ' ' | '\n') {
            at += 1;
        }
        let run: String = chars[start..at].iter().collect();
        let line_end = run.contains('\n');
        if backslashes_before(&chars, start) % 2 == 1 {
            // A break here would end the line on the backslash: a hard break.
            word.push_str(if line_end { " " } else { &run });
            continue;
        }
        let after_url = spans
            .iter()
            .any(|span| span.end == start && span.kind == Kind::Url);
        match (line_end, between(&chars, &spans, start, at, 0, chars.len())) {
            // A URL takes in what a tight join would put after it.
            (true, Between::Tight) if !after_url => {}
            (true, Between::Loose) | (false, Between::Loose) => {
                if !word.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut word)));
                }
                out.push(Piece::Space(if line_end { " ".to_string() } else { run }));
            }
            (true, _) => {
                if !word.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut word)));
                }
                out.push(Piece::Keep);
            }
            (false, _) => word.push_str(&run),
        }
    }
    if !word.is_empty() {
        out.push(Piece::Text(word));
    }
    Some(out)
}

/// A span as it goes into the joined text.
fn spelled(chars: &[char], span: &Span) -> Option<String> {
    let own = &chars[span.start..span.end];
    match span.kind {
        // A code span draws a line end inside it as a space.
        Kind::Code => Some(
            own.iter()
                .map(|&c| if c == '\n' { ' ' } else { c })
                .collect(),
        ),
        Kind::Link { text_end } => {
            let destination = &chars[text_end..span.end];
            if destination.contains(&'\n') {
                return None;
            }
            let mut out = String::from("[");
            out.push_str(&link_text(chars, span.start + 1, text_end)?);
            out.extend(destination);
            Some(out)
        }
        _ if own.contains(&'\n') => None,
        _ => Some(own.iter().collect()),
    }
}

/// A link's text with its line ends joined as they are drawn.
fn link_text(chars: &[char], from: usize, to: usize) -> Option<String> {
    let inner = spans(chars, from, to);
    let mut out = String::new();
    let mut next_span = 0;
    let mut at = from;
    while at < to {
        if let Some(span) = inner.get(next_span).filter(|span| span.start == at) {
            out.push_str(&spelled(chars, span)?);
            at = span.end;
            next_span += 1;
            continue;
        }
        if chars[at] != ' ' && chars[at] != '\n' {
            out.push(chars[at]);
            at += 1;
            continue;
        }
        let start = at;
        while at < to && matches!(chars[at], ' ' | '\n') {
            at += 1;
        }
        if !chars[start..at].contains(&'\n') {
            out.extend(&chars[start..at]);
            continue;
        }
        match between(chars, &inner, start, at, from, to) {
            Between::Tight => {}
            Between::Loose => out.push(' '),
            Between::Unsure => return None,
        }
    }
    Some(out)
}

/// How a line end in `chars[start..end]` is drawn, read in `floor..ceil`.
fn between(
    chars: &[char],
    spans: &[Span],
    start: usize,
    end: usize,
    floor: usize,
    ceil: usize,
) -> Between {
    match (
        drawn_before(chars, spans, start, floor),
        drawn_after(chars, spans, end, ceil),
    ) {
        (Drawn::Char(before, through_before), Drawn::Char(after, through_after)) => {
            if !joins_tight(before, after) {
                Between::Loose
            } else if through_before || through_after {
                // Read as literal, the delimiter would be what is drawn.
                Between::Unsure
            } else {
                Between::Tight
            }
        }
        _ => Between::Unsure,
    }
}

fn drawn_before(chars: &[char], spans: &[Span], mut at: usize, floor: usize) -> Drawn {
    let mut through = false;
    loop {
        if at <= floor {
            return Drawn::Unsure;
        }
        if let Some(span) = spans.iter().find(|span| span.end == at) {
            let drawn = match span.kind {
                Kind::Code => code_text(chars, span).last().copied(),
                Kind::Link { text_end } => {
                    let inner = self::spans(chars, span.start + 1, text_end);
                    return match drawn_before(chars, &inner, text_end, span.start + 1) {
                        Drawn::Char(c, inside) => Drawn::Char(c, through || inside),
                        Drawn::Unsure => Drawn::Unsure,
                    };
                }
                Kind::Escape | Kind::Url => Some(chars[span.end - 1]),
                Kind::Autolink => Some(chars[span.end - 2]),
                Kind::Image => Some(OBJECT),
                Kind::Html | Kind::Bracket => None,
            };
            return drawn.map_or(Drawn::Unsure, |c| Drawn::Char(c, through));
        }
        match chars[at - 1] {
            '*' | '_' | '~' => {
                through = true;
                at -= 1;
            }
            ';' if entity_ends(chars, at - 1, floor) => return Drawn::Unsure,
            c => return Drawn::Char(c, through),
        }
    }
}

fn drawn_after(chars: &[char], spans: &[Span], mut at: usize, ceil: usize) -> Drawn {
    let mut through = false;
    loop {
        if at >= ceil {
            return Drawn::Unsure;
        }
        if let Some(span) = spans.iter().find(|span| span.start == at) {
            let drawn = match span.kind {
                Kind::Code => code_text(chars, span).first().copied(),
                Kind::Link { text_end } => {
                    let inner = self::spans(chars, span.start + 1, text_end);
                    return match drawn_after(chars, &inner, span.start + 1, text_end) {
                        Drawn::Char(c, inside) => Drawn::Char(c, through || inside),
                        Drawn::Unsure => Drawn::Unsure,
                    };
                }
                Kind::Escape => Some(chars[span.start + 1]),
                Kind::Url => Some(chars[span.start]),
                Kind::Autolink => Some(chars[span.start + 1]),
                Kind::Image => Some(OBJECT),
                Kind::Html | Kind::Bracket => None,
            };
            return drawn.map_or(Drawn::Unsure, |c| Drawn::Char(c, through));
        }
        match chars[at] {
            '*' | '_' | '~' => {
                through = true;
                at += 1;
            }
            '&' if entity_starts(chars, at, ceil) => return Drawn::Unsure,
            c => return Drawn::Char(c, through),
        }
    }
}

/// A code span's drawn text: its line ends as spaces, and one space off
/// each end when both ends have one and it is not all spaces.
fn code_text(chars: &[char], span: &Span) -> Vec<char> {
    let ticks = chars[span.start..span.end]
        .iter()
        .take_while(|&&c| c == '`')
        .count();
    let mut text: Vec<char> = chars[span.start + ticks..span.end - ticks]
        .iter()
        .map(|&c| if c == '\n' { ' ' } else { c })
        .collect();
    if text.len() >= 2
        && text[0] == ' '
        && text[text.len() - 1] == ' '
        && text.iter().any(|&c| c != ' ')
    {
        text.remove(0);
        text.pop();
    }
    text
}

fn backslashes_before(chars: &[char], at: usize) -> usize {
    chars[..at].iter().rev().take_while(|&&c| c == '\\').count()
}

/// An entity reference (`&amp;`, `&#12354;`) ending at the `;` at `at`.
fn entity_ends(chars: &[char], at: usize, floor: usize) -> bool {
    let name = chars[floor..at]
        .iter()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '#')
        .count();
    name > 0 && at > floor + name && chars[at - name - 1] == '&'
}

fn entity_starts(chars: &[char], at: usize, ceil: usize) -> bool {
    let name = chars[at + 1..ceil]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '#')
        .count();
    name > 0 && chars.get(at + 1 + name) == Some(&';')
}

/// The spans taken whole in `chars[from..to]`, left to right.
fn spans(chars: &[char], from: usize, to: usize) -> Vec<Span> {
    let mut out = Vec::new();
    let mut at = from;
    while at < to {
        let start = at;
        let whole = move |end: usize, kind: Kind| Span { start, end, kind };
        let found = match chars[at] {
            '\\' if chars.get(at + 1).is_some_and(char::is_ascii_punctuation) && at + 1 < to => {
                Some(whole(at + 2, Kind::Escape))
            }
            '`' => {
                let ticks = chars[at..to].iter().take_while(|&&c| c == '`').count();
                match code_span_end(chars, at, to) {
                    Some(end) => Some(whole(end, Kind::Code)),
                    None => {
                        // Literal backticks: the run as a whole is no opener.
                        at += ticks;
                        continue;
                    }
                }
            }
            '<' => autolink_end(chars, at, to)
                .map(|end| whole(end, Kind::Autolink))
                .or_else(|| html_end(chars, at, to).map(|end| whole(end, Kind::Html))),
            '!' if chars.get(at + 1) == Some(&'[') => bracket(chars, at + 1, to).map(|span| {
                let kind = if matches!(span.kind, Kind::Link { .. }) {
                    Kind::Image
                } else {
                    Kind::Bracket
                };
                whole(span.end, kind)
            }),
            '[' => bracket(chars, at, to),
            _ if bare_url_starts(chars, at, from) => Some(whole(url_end(chars, at, to), Kind::Url)),
            _ => None,
        };
        match found {
            Some(span) => {
                at = span.end;
                out.push(span);
            }
            None => at += 1,
        }
    }
    out
}

/// A code span closes on a run of exactly as many backticks as opened it.
fn code_span_end(chars: &[char], at: usize, to: usize) -> Option<usize> {
    let count = chars[at..to].iter().take_while(|&&c| c == '`').count();
    let mut look = at + count;
    while look < to {
        if chars[look] == '`' {
            let run = chars[look..to].iter().take_while(|&&c| c == '`').count();
            if run == count {
                return Some(look + run);
            }
            look += run;
        } else {
            look += 1;
        }
    }
    None
}

/// CommonMark's autolink from the `<` at `at`: a scheme of 2 to 32
/// letters, digits, `+`, `.`, `-` after a letter, then `:` and no white
/// space or `<` to the `>`; or an address with one `@`.
fn autolink_end(chars: &[char], at: usize, to: usize) -> Option<usize> {
    let close = (at + 1..to).find(|&look| matches!(chars[look], '>' | '<' | ' ' | '\t' | '\n'))?;
    if chars[close] != '>' || close == at + 1 {
        return None;
    }
    let inside = &chars[at + 1..close];
    let scheme = inside.iter().position(|&c| c == ':').filter(|&colon| {
        (2..=32).contains(&colon)
            && inside[0].is_ascii_alphabetic()
            && inside[..colon]
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    let address = inside.iter().filter(|&&c| c == '@').count() == 1
        && inside.first() != Some(&'@')
        && inside.last() != Some(&'@');
    (scheme.is_some() || address).then_some(close + 1)
}

/// Taken widely: anything from `<` and a letter, `/`, `!` or `?` to the
/// next `>` outside quotes, and a comment to its `-->`. A `<` that opens
/// nothing is text.
fn html_end(chars: &[char], at: usize, to: usize) -> Option<usize> {
    let rest = &chars[at + 1..to];
    if rest.starts_with(&['!', '-', '-']) {
        return (at + 2..to.saturating_sub(2))
            .find(|&look| chars[look..to].starts_with(&['-', '-', '>']))
            .map(|look| look + 3);
    }
    let opens = rest
        .first()
        .is_some_and(|&c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?'));
    if !opens {
        return None;
    }
    let mut quote: Option<char> = None;
    for (offset, &c) in rest.iter().enumerate() {
        match (quote, c) {
            (Some(open), _) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(at + 1 + offset + 1),
            (None, '<') => return None,
            (None, _) => {}
        }
    }
    None
}

/// Bracketed text from the `[` at `at`: a link when CommonMark reads one —
/// a destination it takes after the `]` that balances it, and no link in
/// its text — and otherwise the brackets themselves, with a second pair
/// right after them (`[text][label]`).
fn bracket(chars: &[char], at: usize, to: usize) -> Option<Span> {
    let close = balanced(chars, at, to)?;
    if chars.get(close + 1) == Some(&'(')
        && close + 1 < to
        && let Some(end) = destination_end(chars, close + 1, to)
        && destination_reads(&chars[close + 2..end - 1])
        && !spans(chars, at + 1, close)
            .iter()
            .any(|span| matches!(span.kind, Kind::Link { .. }))
    {
        return Some(Span {
            start: at,
            end,
            kind: Kind::Link { text_end: close },
        });
    }
    let end = if chars.get(close + 1) == Some(&'[') && close + 1 < to {
        balanced(chars, close + 1, to).map_or(close + 1, |second| second + 1)
    } else {
        close + 1
    };
    Some(Span {
        start: at,
        end,
        kind: Kind::Bracket,
    })
}

/// What stands between a link's parentheses, read as CommonMark reads it:
/// a destination (`<…>`, or one run with no white space), then, after
/// white space, a title in `"…"`, `'…'` or `(…)`, each part optional.
fn destination_reads(inside: &[char]) -> bool {
    let white = |c: &char| matches!(c, ' ' | '\t' | '\n');
    let mut at = inside.iter().take_while(|c| white(c)).count();
    if inside.get(at) == Some(&'<') {
        let Some(close) = inside[at..].iter().position(|&c| c == '>' || c == '\n') else {
            return false;
        };
        if inside[at + close] != '>' {
            return false;
        }
        at += close + 1;
    } else {
        while at < inside.len() && !white(&inside[at]) {
            at += if inside[at] == '\\' { 2 } else { 1 };
        }
    }
    let gap = inside[at.min(inside.len())..]
        .iter()
        .take_while(|c| white(c))
        .count();
    let rest = &inside[(at + gap).min(inside.len())..];
    let Some(&open) = rest.first() else {
        return true;
    };
    let close = match open {
        '"' | '\'' => open,
        '(' => ')',
        _ => return false,
    };
    if gap == 0 {
        return false;
    }
    let mut look = 1;
    while look < rest.len() && rest[look] != close {
        look += if rest[look] == '\\' { 2 } else { 1 };
    }
    look < rest.len() && rest[look + 1..].iter().all(white)
}

/// The `]` that balances the `[` at `at`, read past escapes and code spans.
fn balanced(chars: &[char], at: usize, to: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut look = at;
    while look < to {
        match chars[look] {
            '\\' => look += 1,
            '`' => {
                let ticks = chars[look..to].iter().take_while(|&&c| c == '`').count();
                look = code_span_end(chars, look, to).unwrap_or(look + ticks);
                continue;
            }
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(look);
                }
            }
            _ => {}
        }
        look += 1;
    }
    None
}

/// A link destination and title, `(` to the `)` that balances it.
fn destination_end(chars: &[char], open: usize, to: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut look = open;
    while look < to {
        match chars[look] {
            '\\' => look += 1,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(look + 1);
                }
            }
            _ => {}
        }
        look += 1;
    }
    None
}

/// GFM's extended autolink, as cmark-gfm finds one: `http://` or
/// `https://` after anything but an ASCII letter (Japanese included), and
/// `www.` where a word begins.
fn bare_url_starts(chars: &[char], at: usize, from: usize) -> bool {
    let before = (at > from).then(|| chars[at - 1]);
    let rest: String = chars[at..chars.len().min(at + 8)]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    if rest.starts_with("http://") || rest.starts_with("https://") {
        return before.is_none_or(|c| !c.is_ascii_alphabetic());
    }
    rest.starts_with("www.")
        && before.is_none_or(|c| matches!(c, ' ' | '\t' | '\n' | '*' | '_' | '~' | '('))
}

/// Where such a URL ends: at a space, a tab, a line end or `<` — U+3000
/// and Japanese run on — less the trailing punctuation cmark-gfm leaves
/// out of the link, and a `)` that closes nothing in it.
fn url_end(chars: &[char], at: usize, to: usize) -> usize {
    let mut end = (at..to)
        .find(|&look| matches!(chars[look], ' ' | '\t' | '\n' | '<'))
        .unwrap_or(to);
    loop {
        match chars[end - 1] {
            '?' | '!' | '.' | ',' | ':' | '*' | '_' | '~' | '\'' | '"' => end -= 1,
            ')' if chars[at..end].iter().filter(|&&c| c == ')').count()
                > chars[at..end].iter().filter(|&&c| c == '(').count() =>
            {
                end -= 1;
            }
            _ => return end,
        }
        if end == at {
            return end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Piece, pieces};

    fn text(piece: &str) -> Piece {
        Piece::Text(piece.to_string())
    }

    fn space() -> Piece {
        Piece::Space(" ".to_string())
    }

    /// An autolink draws its inside: the line may end before it, which a
    /// tag would not allow.
    #[test]
    fn an_autolink_is_judged_by_its_inside() {
        assert_eq!(
            pieces("see <https://example.com/x> and <a@b.example>\n次"),
            Some(vec![
                text("see"),
                space(),
                text("<https://example.com/x>"),
                space(),
                text("and"),
                space(),
                text("<a@b.example>"),
                space(),
                text("次"),
            ])
        );
        assert_eq!(
            pieces("x <span>y</span>"),
            Some(vec![text("x <span>y</span>")])
        );
    }

    /// A README's badges: a line may end between two linked images, which
    /// draw as objects.
    #[test]
    fn a_space_between_two_images_is_a_place_to_end_a_line() {
        assert_eq!(
            pieces("[![a](x.svg)](y) [![b](z.svg)](w)\n次"),
            Some(vec![
                text("[![a](x.svg)](y)"),
                space(),
                text("[![b](z.svg)](w)"),
                space(),
                text("次"),
            ])
        );
        assert_eq!(
            pieces("![図](a.png)\n次の行"),
            Some(vec![
                text("![図](a.png)"),
                space(),
                text("次"),
                Piece::Tight,
                text("の"),
                Piece::Tight,
                text("行"),
            ])
        );
    }

    #[test]
    fn the_pieces_of_mixed_text() {
        assert_eq!(
            pieces("の `x` で\n日本"),
            Some(vec![
                text("の"),
                space(),
                text("`x`"),
                space(),
                text("で"),
                Piece::Tight,
                text("日"),
                Piece::Tight,
                text("本"),
            ])
        );
    }

    /// `**重要**` then a line end then `です` draws `重要です`: the emphasis
    /// is no box CSS stops at. Whether `**` is emphasis at all takes the
    /// delimiter rules, so the line end stays where it is.
    #[test]
    fn a_line_end_by_a_delimiter_between_japanese_stays_where_it_is() {
        assert_eq!(
            pieces("**重要**\nです"),
            Some(vec![
                text("**重"),
                Piece::Tight,
                text("要**"),
                Piece::Keep,
                text("で"),
                Piece::Tight,
                text("す")
            ])
        );
        assert_eq!(
            pieces("**強調** です"),
            Some(vec![
                text("**強"),
                Piece::Tight,
                text("調** で"),
                Piece::Tight,
                text("す")
            ])
        );
        assert_eq!(
            pieces("**bold**\nthen"),
            Some(vec![text("**bold**"), space(), text("then")])
        );
    }

    /// A link's text and a code span's are drawn: what stands at their edge
    /// is the character a line end is judged by.
    #[test]
    fn a_line_end_is_judged_by_the_characters_inside_a_link_or_code() {
        assert_eq!(
            pieces("[機械化](x.md)\nです"),
            Some(vec![text("[機械化](x.md)で"), Piece::Tight, text("す")])
        );
        assert_eq!(
            pieces("`日本`\nです"),
            Some(vec![text("`日本`で"), Piece::Tight, text("す")])
        );
        assert_eq!(
            pieces("`日本` です"),
            Some(vec![text("`日本` で"), Piece::Tight, text("す")])
        );
    }

    #[test]
    fn a_tag_or_a_bracket_by_a_line_end_keeps_it() {
        assert_eq!(
            pieces("<b>強</b>\nです"),
            Some(vec![
                text("<b>強</b>"),
                Piece::Keep,
                text("で"),
                Piece::Tight,
                text("す")
            ])
        );
        assert_eq!(
            pieces("[注]\nです"),
            Some(vec![
                text("[注]"),
                Piece::Keep,
                text("で"),
                Piece::Tight,
                text("す")
            ])
        );
    }

    /// The URL would take in what a tight join puts after it.
    #[test]
    fn a_bare_url_is_whole_and_keeps_the_line_end_after_it() {
        assert_eq!(
            pieces("見よ https://example.com/ページ名の一覧\nを"),
            Some(vec![
                text("見"),
                Piece::Tight,
                text("よ"),
                space(),
                text("https://example.com/ページ名の一覧"),
                Piece::Keep,
                text("を"),
            ])
        );
    }

    /// cmark-gfm links `https://` straight after Japanese, leaves trailing
    /// `**` out of the link, and runs the link through U+3000.
    #[test]
    fn a_bare_url_is_found_and_ended_as_cmark_gfm_does() {
        assert_eq!(
            pieces("詳細はhttps://example.com/日本\nです"),
            Some(vec![
                text("詳"),
                Piece::Tight,
                text("細"),
                Piece::Tight,
                text("はhttps://example.com/日本"),
                Piece::Keep,
                text("で"),
                Piece::Tight,
                text("す"),
            ])
        );
        assert_eq!(
            pieces("見よ **https://example.com/日本**\nです"),
            Some(vec![
                text("見"),
                Piece::Tight,
                text("よ"),
                space(),
                text("**https://example.com/日本**"),
                Piece::Keep,
                text("で"),
                Piece::Tight,
                text("す"),
            ])
        );
        assert_eq!(
            pieces("見よ https://example.com/日本　です"),
            Some(vec![
                text("見"),
                Piece::Tight,
                text("よ"),
                space(),
                text("https://example.com/日本　です"),
            ])
        );
    }

    /// A destination with a space, or a link inside the text, is no link:
    /// the brackets are drawn, and judge the line end beside them.
    #[test]
    fn brackets_that_make_no_link_are_drawn() {
        assert_eq!(
            pieces("[日本](x y)\nです"),
            Some(vec![
                text("[日本](x"),
                space(),
                text("y)"),
                space(),
                text("で"),
                Piece::Tight,
                text("す"),
            ])
        );
        assert_eq!(
            pieces("[a[日本](b)](c)\nです"),
            Some(vec![
                text("[a[日本](b)](c)"),
                space(),
                text("で"),
                Piece::Tight,
                text("す")
            ])
        );
        assert_eq!(
            pieces("[日本](x \"題\")\nです"),
            Some(vec![text("[日本](x \"題\")で"), Piece::Tight, text("す")])
        );
    }

    /// No space inside the attribute is a place to end a line; nor is one
    /// beside the tag, whose drawn neighbour cannot be told.
    #[test]
    fn a_quoted_attribute_is_part_of_its_tag() {
        assert_eq!(
            pieces("<span title=\"a>b c d\">x</span> tail"),
            Some(vec![text("<span title=\"a>b c d\">x</span> tail")])
        );
        assert_eq!(
            pieces("w <span title=\"a>b c d\">"),
            Some(vec![text("w <span title=\"a>b c d\">")])
        );
    }

    #[test]
    fn a_reference_label_is_never_split() {
        assert_eq!(
            pieces("[設定画面の説明]を読む"),
            Some(vec![
                text("[設定画面の説明]を"),
                Piece::Tight,
                text("読"),
                Piece::Tight,
                text("む")
            ])
        );
        assert_eq!(pieces("[設定\n画面]を"), None);
    }

    #[test]
    fn a_code_span_in_a_link_draws_its_line_end_as_a_space() {
        assert_eq!(
            pieces("見よ [`日本\n語`](x.md)"),
            Some(vec![text("見"), Piece::Tight, text("よ [`日本 語`](x.md)")])
        );
    }

    #[test]
    fn a_zero_width_space_removes_the_line_end_beside_it() {
        assert_eq!(pieces("a\u{200B}\nb"), Some(vec![text("a\u{200B}b")]));
        assert_eq!(pieces("a\u{200B} b"), Some(vec![text("a\u{200B} b")]));
    }

    /// `C:\ ` then a line end: the backslash is literal and the line end a
    /// space. Ending the line on the backslash would make a hard break.
    #[test]
    fn a_backslash_before_a_line_end_is_never_left_at_one() {
        assert_eq!(pieces("C:\\ \nnext"), Some(vec![text("C:\\ next")]));
    }

    #[test]
    fn an_entity_by_a_line_end_keeps_it() {
        assert_eq!(
            pieces("&#12354;\nい"),
            Some(vec![text("&#12354;"), Piece::Keep, text("い")])
        );
    }
}
