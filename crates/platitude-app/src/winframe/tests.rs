//! The window's icon is the class icon Qt finds in the exe's resource
//! (`assets/platitude.rc`). No headless run creates a native window, so
//! only the compiled resource can say whether Qt will find it.

/// `RT_GROUP_ICON` (winuser.h).
const GROUP_ICON: u16 = 14;

const RESOURCE: &[u8] = include_bytes!("../../assets/platitude.res");

/// A resource's type or name: a number, or a string.
#[derive(Debug, PartialEq)]
enum Id {
    Number(u16),
    Name(String),
}

/// The type and name of each entry of a compiled `.res`. An entry is its
/// data's size and its header's, the type and the name, a fixed tail, then
/// the data; the header and the entry each end on a DWORD.
fn entries(res: &[u8]) -> Vec<(Id, Id)> {
    let word = |at: usize| u16::from_le_bytes([res[at], res[at + 1]]);
    let dword = |at: usize| {
        let bytes = [res[at], res[at + 1], res[at + 2], res[at + 3]];
        usize::try_from(u32::from_le_bytes(bytes)).unwrap()
    };
    let id = |at: &mut usize| {
        if word(*at) == 0xFFFF {
            let number = word(*at + 2);
            *at += 4;
            return Id::Number(number);
        }
        let mut units = Vec::new();
        while word(*at) != 0 {
            units.push(word(*at));
            *at += 2;
        }
        *at += 2;
        Id::Name(String::from_utf16(&units).unwrap())
    };
    let mut found = Vec::new();
    let mut at = 0;
    while at < res.len() {
        let (data, header) = (dword(at), dword(at + 4));
        let mut field = at + 8;
        let kind = id(&mut field);
        let name = id(&mut field);
        found.push((kind, name));
        at = (at + header + data).next_multiple_of(4);
    }
    found
}

/// Under the numeric id the file still shows the icon, but every window
/// comes up wearing `IDI_APPLICATION`.
#[test]
fn the_icon_has_the_name_qt_registers_window_classes_with() {
    let groups: Vec<Id> = entries(RESOURCE)
        .into_iter()
        .filter(|(kind, _)| *kind == Id::Number(GROUP_ICON))
        .map(|(_, name)| name)
        .collect();
    assert_eq!(groups, [Id::Name("IDI_ICON1".to_string())]);
}
