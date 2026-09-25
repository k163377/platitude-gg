//! Tests of [`crate::mem`]'s footprints and the report it renders
//! (structure.md §分割).

use super::*;

#[test]
fn a_string_owns_its_buffer() {
    let mut s = String::with_capacity(64);
    s.push_str("abc");
    assert_eq!(s.heap_bytes(), 64);
    assert_eq!(String::new().heap_bytes(), 0);
}

#[test]
fn a_vec_counts_its_buffer_and_its_elements() {
    let items: Vec<String> = vec!["a".repeat(40), "b".repeat(40)];
    // Two Strings inline (capacity is exact for `repeat`) plus their
    // buffers.
    assert_eq!(
        items.heap_bytes(),
        items.capacity() * size_of::<String>() + 80
    );
}

#[test]
fn an_empty_container_owns_nothing() {
    assert_eq!(Vec::<String>::new().heap_bytes(), 0);
    assert_eq!(HashMap::<u32, String>::new().heap_bytes(), 0);
    assert_eq!(BTreeMap::<u32, String>::new().heap_bytes(), 0);
    assert_eq!(Option::<String>::None.heap_bytes(), 0);
}

#[test]
fn a_map_counts_its_table_and_its_entries() {
    let mut map: HashMap<u32, String> = HashMap::new();
    map.insert(1, "x".repeat(10));
    assert!(map.heap_bytes() >= table_bytes::<(u32, String)>(map.capacity()) + 10);
}

#[test]
fn nested_rows_add_up() {
    let row = crate::session::LogRow {
        row: 0,
        oid_hex: "0".repeat(40),
        short_sha: String::new(),
        author: String::new(),
        author_email: String::new(),
        co_authors: Vec::new(),
        time: 0,
        subject: "s".repeat(30),
        body: String::new(),
        node_lane: 0,
        node_color: 0,
        width: 1,
        segments: Vec::new(),
        labels: Vec::new(),
        stash_ref: String::new(),
        published: false,
        parents: Box::from([crate::Oid::from_hex_str(&"1".repeat(40)).unwrap()]),
        carried: None,
    };
    let nested = 70 + size_of::<crate::Oid>();
    assert_eq!(row.heap_bytes(), nested);
    let rows = vec![row];
    assert_eq!(
        rows.heap_bytes(),
        rows.capacity() * size_of::<crate::session::LogRow>() + nested
    );
}

#[test]
fn a_one_entry_btree_still_costs_a_whole_node() {
    let mut one: BTreeMap<u64, u64> = BTreeMap::new();
    one.insert(1, 1);
    let node = 2 * size_of::<usize>() + 11 * 16;
    assert_eq!(one.heap_bytes(), node);
    // Eleven fit in one real node; the estimate (about six per node)
    // charges two.
    let full: BTreeMap<u64, u64> = (0..11).map(|i| (i, i)).collect();
    assert_eq!(full.heap_bytes(), 2 * node);
}

#[test]
fn parts_render_and_total() {
    let parts = vec![Part::new("rows", 100, 2), Part::new("labels", 50, 3)];
    assert_eq!(total(&parts), 150);
    assert_eq!(render(&parts), "rows=100/2 labels=50/3");
}
