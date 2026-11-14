.pragma library

// The chip records the tests hand a row, a stack or a card: every field spelled out as `encode::chips_of` hands one,
// so a field it gains is one edit here.

/// A record of `kind` named `name`: every mark down and on this worktree (`here`, as `RefChip.noChip` reads), unless
/// `marks` sets them.
function chip(kind, name, marks) {
    return Object.assign({ "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false,
                           "here": true, "held": false, "locked": false, "home": false, "remote": "",
                           "key": kind + ":" + name },
                         marks === undefined ? {} : marks)
}
