//! `parse::log`'s tests, split out for length alone (structure.md §分割).

use super::*;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccccccccccc";

/// One record with an address derived from the name, for the tests
/// that are not about the address.
fn record(oid: &str, parents: &str, author: &str, time: &str, subject: &str) -> Vec<u8> {
    let email = format!("{}@example.com", author.to_lowercase());
    record_with_email(oid, parents, author, &email, time, subject)
}

fn record_with_email(
    oid: &str,
    parents: &str,
    author: &str,
    email: &str,
    time: &str,
    subject: &str,
) -> Vec<u8> {
    record_with_mates(oid, parents, author, email, time, "", "", subject)
}

#[expect(
    clippy::too_many_arguments,
    reason = "one parameter per field of the record it builds; naming \
              them at each call site is what makes the fixtures read"
)]
fn record_with_mates(
    oid: &str,
    parents: &str,
    author: &str,
    email: &str,
    time: &str,
    mates: &str,
    body: &str,
    subject: &str,
) -> Vec<u8> {
    let mut v = Vec::new();
    for field in [oid, parents, author, email, time, mates, body, subject] {
        v.extend_from_slice(field.as_bytes());
        v.push(0);
    }
    v
}

fn parse_all(bytes: &[u8], chunk_size: usize) -> (Vec<CommitMeta>, LogParser) {
    let mut parser = LogParser::new();
    let mut out = Vec::new();
    for chunk in bytes.chunks(chunk_size.max(1)) {
        parser.feed(chunk, &mut out).unwrap();
    }
    parser.finish().unwrap();
    (out, parser)
}

#[test]
fn co_author_trailers_ride_along_in_the_record() {
    let bytes = record_with_mates(
        A,
        "",
        "Alice",
        "alice@example.com",
        "1700000000",
        "Claude Opus 5 <noreply@anthropic.com>\u{1f}Nameless",
        "Why it was done.\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>\n\
         Co-authored-by: Nameless",
        "pair on it",
    );
    let (commits, parser) = parse_all(&bytes, bytes.len());
    assert_eq!(commits.len(), 1);
    let mates = &commits[0].co_authors;
    assert_eq!(mates.len(), 2);
    assert_eq!(parser.pool().get(mates[0].0), "Claude Opus 5");
    assert_eq!(parser.pool().get(mates[0].1), "noreply@anthropic.com");
    assert_eq!(parser.pool().get(mates[1].0), "Nameless");
    assert_eq!(parser.pool().get(mates[1].1), "");
    // git hands the trailers back inside %b as well; the body the
    // hover reads is the writing without them.
    assert_eq!(&*commits[0].body, "Why it was done.");
}

#[test]
fn a_co_authored_by_in_the_prose_stays_in_the_body() {
    // git did not call it a trailer (field 5 is empty), so neither
    // does the filter — it is a sentence somebody wrote.
    let bytes = record_with_mates(
        A,
        "",
        "Alice",
        "alice@example.com",
        "1700000000",
        "",
        "I wrote Co-authored-by: nobody <n@e.com> in the body.",
        "alone",
    );
    let (commits, _) = parse_all(&bytes, bytes.len());
    assert!(commits[0].body.contains("Co-authored-by: nobody"));
}

#[test]
fn a_body_line_that_is_not_ascii_where_the_key_ends_is_kept() {
    // The filter reads as many bytes as the key is long, so a line
    // whose character *straddles* that byte is the one that matters
    // — an em dash or a CJK character starting one or two bytes
    // short of the end. Both are everyday writing (this repository's
    // own history is full of the first), and slicing a `str` there
    // took the whole walk down with it: the graph never left its
    // loading ring (observed on platitude-gg itself).
    // The guard below is what keeps these honest: hand-counted bytes
    // stop reproducing the moment somebody rewords them.
    let bodies = [
        "The reason is—said plainly",
        "The reason is 版で書いてある",
        "co-authored-b—not the key",
    ];
    for body in bodies {
        assert!(
            !body.is_char_boundary(CO_AUTHOR_KEY.len()),
            "{body} does not reproduce: byte {} is a boundary",
            CO_AUTHOR_KEY.len()
        );
        let bytes = record_with_mates(
            A,
            "",
            "Alice",
            "alice@example.com",
            "1700000000",
            "",
            body,
            "wrote prose",
        );
        let (commits, _) = parse_all(&bytes, bytes.len());
        assert_eq!(commits.len(), 1, "{body}");
        assert_eq!(&*commits[0].body, body);
    }
}

#[test]
fn a_commit_with_no_trailer_credits_nobody() {
    let bytes = record(A, "", "Alice", "1700000000", "alone");
    let (commits, _) = parse_all(&bytes, bytes.len());
    assert!(commits[0].co_authors.is_empty());
}

#[test]
fn parses_multiple_records() {
    let mut bytes = record(A, &format!("{B} {C}"), "Alice", "1700000000", "merge two");
    bytes.extend(record(B, C, "Bob", "1699999940", "second"));
    bytes.extend(record(C, "", "Alice", "1699999880", "root"));

    let (commits, parser) = parse_all(&bytes, bytes.len());
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].oid.to_hex(), A);
    assert_eq!(commits[0].parents.len(), 2);
    assert_eq!(parser.pool().get(commits[0].author), "Alice");
    assert_eq!(commits[0].time, 1_700_000_000);
    assert_eq!(&*commits[0].subject, "merge two");
    assert_eq!(commits[2].parents.len(), 0);
    assert_eq!(commits[0].author, commits[2].author, "author interned");
}

#[test]
fn the_address_is_lowercased_and_interned() {
    // The same person, shouting on one commit and not the other. The
    // key a picture is filed under has to come out the same either
    // way, which is why case is dropped on the way in.
    let mut bytes = record_with_email(A, "", "Alice", "Alice@Example.COM", "1", "one");
    bytes.extend(record_with_email(
        B,
        A,
        "Alice",
        "alice@example.com",
        "2",
        "two",
    ));
    let (commits, parser) = parse_all(&bytes, 3);
    assert_eq!(
        parser.pool().get(commits[0].author_email),
        "alice@example.com"
    );
    assert_eq!(
        commits[0].author_email, commits[1].author_email,
        "one address, one pool entry"
    );
}

#[test]
fn an_empty_address_is_kept_as_one() {
    // git writes `<>` for an author with no address, and `%aE` comes
    // back empty. Nothing is a valid answer; it just matches no
    // picture.
    let bytes = record_with_email(A, "", "Nobody", "", "1", "one");
    let (commits, parser) = parse_all(&bytes, 7);
    assert_eq!(parser.pool().get(commits[0].author_email), "");
}

#[test]
fn arbitrary_chunk_boundaries_do_not_matter() {
    let mut bytes = record(A, B, "Alice", "1", "one");
    bytes.extend(record(B, "", "日本語 太郎", "2", "日本語サブジェクト 🎌"));
    for chunk_size in [1, 2, 3, 7, 16, 64] {
        let (commits, parser) = parse_all(&bytes, chunk_size);
        assert_eq!(commits.len(), 2, "chunk size {chunk_size}");
        assert_eq!(parser.pool().get(commits[1].author), "日本語 太郎");
        assert_eq!(&*commits[1].subject, "日本語サブジェクト 🎌");
    }
}

#[test]
fn empty_subject_is_preserved() {
    let bytes = record(A, "", "Alice", "1", "");
    let (commits, _) = parse_all(&bytes, 5);
    assert_eq!(&*commits[0].subject, "");
}

#[test]
fn truncated_stream_is_an_error() {
    let bytes = record(A, "", "Alice", "1", "subject");
    let cut = &bytes[..bytes.len() - 3];
    let mut parser = LogParser::new();
    let mut out = Vec::new();
    parser.feed(cut, &mut out).unwrap();
    assert_eq!(parser.finish(), Err(LogParseError::Truncated));
}

#[test]
fn trailing_newline_is_tolerated() {
    let mut bytes = record(A, "", "Alice", "1", "subject");
    bytes.push(b'\n');
    let mut parser = LogParser::new();
    let mut out = Vec::new();
    parser.feed(&bytes, &mut out).unwrap();
    parser.finish().unwrap();
    assert_eq!(out.len(), 1);
}

#[test]
fn invalid_oid_is_fatal() {
    let bytes = record("zzzz", "", "Alice", "1", "subject");
    let mut parser = LogParser::new();
    let mut out = Vec::new();
    assert!(matches!(
        parser.feed(&bytes, &mut out),
        Err(LogParseError::InvalidOid { record: 0 })
    ));
}

#[test]
fn invalid_timestamp_is_fatal() {
    let bytes = record(A, "", "Alice", "not-a-number", "subject");
    let mut parser = LogParser::new();
    let mut out = Vec::new();
    assert!(matches!(
        parser.feed(&bytes, &mut out),
        Err(LogParseError::InvalidTimestamp { record: 0 })
    ));
}

#[test]
fn negative_timestamp_parses() {
    // Commits before the epoch exist in the wild.
    let bytes = record(A, "", "Alice", "-100", "old");
    let (commits, _) = parse_all(&bytes, 4);
    assert_eq!(commits[0].time, -100);
}
