//! Every class there is, and the rows it has: the table of what decides a
//! menu's rows, written by the types — and held to what [`Class::on`]
//! makes of everything a menu is opened on, so a class the table leaves
//! out, or one nothing makes, fails here rather than in a reader's hands.

use super::render;
use crate::menu::{
    Below, BranchClass, Chosen, Class, On, RemoteClass, RemoteStand, Row, Seen, Shape, Stand,
    StepClass, TagClass, UpstreamShape, WorktreeClass, WorktreeKind, WorktreeShape,
};
use crate::offers::{MovedBranch, RefKind, TagSides};
use crate::stash::StashStanding;
use crate::worktrees::Keeper;

const BOTH: [bool; 2] = [false, true];

/// Every variant of a field-less enum, held to the enum: one added does not
/// compile until it is listed here, so no enumeration below leaves it out
/// unseen.
macro_rules! every {
    ($kind:ident: $($variant:ident),+ $(,)?) => {{
        let _listed = |value: $kind| match value {
            $($kind::$variant)|+ => (),
        };
        [$($kind::$variant),+]
    }};
}

/// Every class: each distinction taken every way it goes. The class's
/// fields are free of one another, so every combination is one something
/// makes (`the_table_is_every_class_a_shape_makes`). A kind of class added
/// does not compile until it is named here, beside the table it goes in.
fn every_class() -> Vec<Class> {
    let _kinds_listed = |class: Class| match class {
        Class::Branch(_)
        | Class::Remote(_)
        | Class::Tag(_)
        | Class::Stash
        | Class::Worktree(_)
        | Class::Commit { .. }
        | Class::Provisional
        | Class::Carried
        | Class::Files(_)
        | Class::Step(_)
        | Class::RemoteConfig => (),
    };
    let mut all = Vec::new();
    let stands = [
        Stand::Here,
        Stand::Kept,
        Stand::Apart { moves: false },
        Stand::Apart { moves: true },
    ];
    let _stands_listed = |stand: Stand| match stand {
        Stand::Here | Stand::Kept | Stand::Apart { .. } => (),
    };
    let worktree_kinds = every!(WorktreeKind: Main, Other);
    for stand in stands {
        for upstream in
            std::iter::once(None).chain(every!(UpstreamShape: Read, Here, Gone, Counted).map(Some))
        {
            for worktree in std::iter::once(None).chain(worktree_kinds.map(Some)) {
                for on_graph in BOTH {
                    all.push(Class::Branch(BranchClass {
                        stand,
                        upstream,
                        worktree,
                        on_graph,
                    }));
                }
            }
        }
    }
    for stand in every!(RemoteStand: Still, Moves, Compared) {
        for on_graph in BOTH {
            all.push(Class::Remote(RemoteClass { stand, on_graph }));
        }
    }
    for moves in BOTH {
        for sides in every!(TagSides: Here, Remote, Both) {
            for on_graph in BOTH {
                all.push(Class::Tag(TagClass {
                    moves,
                    sides,
                    on_graph,
                }));
            }
        }
    }
    all.push(Class::Stash);
    for here in BOTH {
        for moves in BOTH {
            for kind in worktree_kinds {
                for on_graph in BOTH {
                    all.push(Class::Worktree(WorktreeClass {
                        here,
                        moves,
                        kind,
                        on_graph,
                    }));
                }
            }
        }
    }
    for moves in BOTH {
        all.push(Class::Commit { moves });
    }
    all.push(Class::Provisional);
    all.push(Class::Carried);
    for chosen in every!(Chosen: Conflicts, Changes, Both) {
        all.push(Class::Files(chosen));
    }
    for below in BOTH {
        all.push(Class::Step(StepClass { below }));
    }
    all.push(Class::RemoteConfig);
    all
}

/// The classes made of everything a menu is opened on that tells two
/// apart: a ref's or a graph row's shape, each input taken through the
/// values that make a difference — a name that is the branch the tab moves,
/// the tab's upstream, or neither —, against every other's ([`Class::of`],
/// which [`Class::on`] hands a ref's to); and every other subject's facts
/// ([`apart_subjects`]).
fn classes_made() -> Vec<Class> {
    let seen = [
        Seen::Listed,
        Seen::Graph { provisional: false },
        Seen::Graph { provisional: true },
    ];
    let _seen_listed = |seen: Seen| match seen {
        Seen::Listed | Seen::Graph { .. } => (),
    };
    let kinds: Vec<Option<RefKind>> = std::iter::once(None)
        .chain(every!(RefKind: Branch, Remote, Tag, Stash, Worktree).map(Some))
        .collect();
    let names = ["main", "feat", "origin/main"];
    let moved = [
        None,
        Some(MovedBranch::Head("main")),
        Some(MovedBranch::Kept {
            name: "main",
            by: Keeper::Rebase,
        }),
    ];
    let _moved_listed = |moved: MovedBranch<'_>| match moved {
        MovedBranch::Head(_) | MovedBranch::Kept { .. } => (),
    };
    let upstreams: Vec<Option<UpstreamShape>> = std::iter::once(None)
        .chain(every!(UpstreamShape: Read, Here, Gone, Counted).map(Some))
        .collect();
    let tag_sides: Vec<Option<TagSides>> = std::iter::once(None)
        .chain(every!(TagSides: Here, Remote, Both).map(Some))
        .collect();
    let mut worktrees = vec![None];
    for main in BOTH {
        for here in BOTH {
            worktrees.push(Some(WorktreeShape { main, here }));
        }
    }
    let mut made: Vec<Class> = Vec::new();
    let mut add = |class: Option<Class>| {
        if let Some(class) = class
            && !made.contains(&class)
        {
            made.push(class);
        }
    };
    for seen in seen {
        for &kind in &kinds {
            for name in names {
                for moved in moved {
                    for tab_upstream in ["", "origin/main"] {
                        for &upstream in &upstreams {
                            for &tag_sides in &tag_sides {
                                for &worktree in &worktrees {
                                    add(Class::of(&Shape {
                                        seen,
                                        kind,
                                        name,
                                        moved,
                                        tab_upstream,
                                        upstream,
                                        tag_sides,
                                        worktree,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for on in apart_subjects() {
        add(Class::on(&on));
    }
    made
}

/// Every subject no [`Shape`] makes, off every fact it takes. A subject
/// added does not compile until it is named here, so the classes it makes
/// are made, and the table is asked for them.
fn apart_subjects() -> Vec<On<'static>> {
    // A ref's subject is made of every shape (`classes_made`).
    let _subjects_listed = |on: On<'_>| match on {
        On::Ref(_) | On::Carried { .. } | On::Files { .. } | On::Step(_) | On::Remote { .. } => (),
    };
    let mut all = Vec::new();
    for conflicts_here in BOTH {
        for conflicts_there in BOTH {
            all.push(On::Carried {
                conflicts_here,
                conflicts_there,
            });
        }
    }
    for chosen in every!(Chosen: Conflicts, Changes, Both) {
        for stash in every!(StashStanding: Unborn, Conflicts, Clean, Ready) {
            all.push(On::Files { chosen, stash });
        }
    }
    for below in every!(Below: Nothing, Dropped, Landing) {
        all.push(On::Step(below));
    }
    for marked in BOTH {
        all.push(On::Remote { marked });
    }
    all
}

/// The class in a few words: its kind, then each distinction it has.
fn describe(class: &Class) -> String {
    let worktree = |worktree: Option<WorktreeKind>| match worktree {
        None => "",
        Some(WorktreeKind::Main) => ", the repository's worktree",
        Some(WorktreeKind::Other) => ", another worktree",
    };
    // Whether the tab moves a branch is said once: beside the graph where
    // the row is drawn there, after the rest otherwise.
    let moving = |moves: bool, on_graph: bool| match (on_graph, moves) {
        (true, true) => " — on the graph, tab moving a branch",
        (true, false) => " — on the graph",
        (false, true) => ", tab moving a branch",
        (false, false) => "",
    };
    match *class {
        Class::Branch(BranchClass {
            stand,
            upstream,
            worktree: kind,
            on_graph,
        }) => format!(
            "branch {}{}{}{}",
            match stand {
                Stand::Here => "here",
                Stand::Kept => "kept by a stopped operation",
                Stand::Apart { .. } => "elsewhere",
            },
            match upstream {
                None => "",
                Some(UpstreamShape::Read) => ", upstream read",
                Some(UpstreamShape::Here) => ", upstream a branch here",
                Some(UpstreamShape::Gone) => ", upstream gone",
                Some(UpstreamShape::Counted) =>
                    ", upstream git counts and the listing has no row for",
            },
            worktree(kind),
            // Here and kept, the branch is the one the tab moves.
            moving(
                matches!(stand, Stand::Apart { moves: true }) || on_graph && stand.moves(),
                on_graph
            ),
        ),
        Class::Remote(RemoteClass { stand, on_graph }) => format!(
            "remote branch{}{}",
            if stand == RemoteStand::Compared {
                ", the tab's upstream"
            } else {
                ""
            },
            moving(stand != RemoteStand::Still, on_graph),
        ),
        Class::Tag(TagClass {
            moves,
            sides,
            on_graph,
        }) => format!("tag {}{}", sides.word(), moving(moves, on_graph)),
        Class::Stash => "stash".to_string(),
        Class::Worktree(WorktreeClass {
            here,
            moves,
            kind,
            on_graph,
        }) => format!(
            "worktree{}{}{}",
            if here { " shown here" } else { " elsewhere" },
            worktree(Some(kind)),
            moving(moves, on_graph),
        ),
        Class::Commit { moves } => format!("a graph row's commit{}", moving(moves, false)),
        Class::Provisional => "a graph row's commit, the discard log's entry".to_string(),
        Class::Carried => "another worktree's uncommitted row".to_string(),
        Class::Files(chosen) => format!(
            "chosen files: {}",
            match chosen {
                Chosen::Conflicts => "conflicted",
                Chosen::Changes => "not conflicted",
                Chosen::Both => "conflicted and not",
            }
        ),
        Class::Step(StepClass { below }) => {
            format!("a plan's step{}", if below { "" } else { ", the oldest" })
        }
        Class::RemoteConfig => "a remote's own row".to_string(),
    }
}

#[test]
fn every_class_has_the_rows_of_its_parts() {
    let table: Vec<String> = every_class()
        .iter()
        .map(|class| {
            let rows = class.rows().into_iter().map(|row| (row, true)).collect();
            format!("{}: {}", describe(class), render(&rows))
        })
        .collect();
    insta::assert_snapshot!(table.join("\n"));
}

/// The table is what is made: no class made of a shape or a subject's
/// facts is left out of it, and none in it is one nothing makes.
#[test]
fn the_table_is_every_class_a_shape_makes() {
    let table = every_class();
    let made = classes_made();
    let left_out: Vec<&Class> = made.iter().filter(|class| !table.contains(class)).collect();
    assert!(left_out.is_empty(), "made, not in the table: {left_out:?}");
    let unmade: Vec<&Class> = table.iter().filter(|class| !made.contains(class)).collect();
    assert!(
        unmade.is_empty(),
        "in the table, made by no shape: {unmade:?}"
    );
}

#[test]
fn every_row_stands_on_some_class() {
    let classes = every_class();
    for row in Row::ALL {
        assert!(
            classes.iter().any(|class| class.rows().contains(row)),
            "{row:?} belongs to a part no class has"
        );
    }
}
