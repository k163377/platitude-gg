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
    // %b carries the trailers too; the body is the writing without them.
    assert_eq!(&*commits[0].body, "Why it was done.");
}

#[test]
fn a_co_authored_by_in_the_prose_stays_in_the_body() {
    // Field 5 is empty: git did not call it a trailer.
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
    // The filter reads as many bytes as the key is long, so what matters
    // is a character *straddling* that byte (an em dash, a CJK character).
    // The guard below keeps the fixtures honest: hand-counted bytes stop
    // reproducing once reworded.
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

/// What git's fold makes of a real `.mailmap` is in `logparse::periodic`.
#[test]
fn the_author_fields_are_the_mailmap_spellings() {
    let fields: Vec<&str> = LOG_FORMAT_ARG
        .strip_prefix("--format=")
        .unwrap()
        .split("%x00")
        .collect();
    assert_eq!(fields.len(), LOG_FIELDS, "one token per field: {fields:?}");
    assert_eq!(fields[2], "%aN", "the author's name");
    assert_eq!(fields[3], "%aE", "the author's address");
}

#[test]
fn the_address_is_lowercased_and_interned() {
    // The key a picture is filed under must not depend on case.
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
    // git writes `<>` for no address; empty is valid and matches no
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
