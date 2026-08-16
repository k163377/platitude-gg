//! The NUL-record builder both config-parsing test modules use.

pub(super) fn z(records: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for r in records {
        v.extend_from_slice(r.as_bytes());
        v.push(0);
    }
    v
}
