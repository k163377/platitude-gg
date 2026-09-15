//! Tests of [`crate::sequencer`]'s todo rendering and helpers, in a
//! file of their own (structure.md §分割: テストだけ巨大なら同ディレクトリの専用ファイルへ).

use std::path::Path;

use crate::sequencer::*;

#[test]
fn renders_the_actions_git_understands() {
    let lines = vec![
        TodoLine::Command {
            action: TodoAction::Pick,
            oid: "aaa".into(),
            subject: "first".into(),
        },
        TodoLine::Command {
            action: TodoAction::Fixup,
            oid: "bbb".into(),
            subject: "second".into(),
        },
        TodoLine::Exec {
            command: "git commit --amend".into(),
        },
    ];
    assert_eq!(
        render_todo(&lines),
        "pick aaa first\nfixup bbb second\nexec git commit --amend\n"
    );
}

#[test]
fn reword_renders_as_pick_because_the_exec_carries_the_message() {
    let lines = vec![TodoLine::Command {
        action: TodoAction::Reword,
        oid: "aaa".into(),
        subject: "s".into(),
    }];
    assert_eq!(render_todo(&lines), "pick aaa s\n");
}

#[test]
fn newlines_in_a_subject_cannot_forge_commands() {
    let lines = vec![TodoLine::Command {
        action: TodoAction::Pick,
        oid: "aaa".into(),
        subject: "innocent\ndrop bbb".into(),
    }];
    assert_eq!(render_todo(&lines), "pick aaa innocent drop bbb\n");
}

#[test]
fn parses_git_own_todo_including_short_forms() {
    let text = "\
# This is a combination of 2 commits.
pick 1111111 first subject
f 2222222 second subject

x echo hi
drop 3333333 gone
noop-command 4444444 ignored
";
    let lines = parse_todo(text);
    assert_eq!(lines.len(), 4);
    assert_eq!(
        lines[0],
        TodoLine::Command {
            action: TodoAction::Pick,
            oid: "1111111".into(),
            subject: "first subject".into()
        }
    );
    assert_eq!(
        lines[1],
        TodoLine::Command {
            action: TodoAction::Fixup,
            oid: "2222222".into(),
            subject: "second subject".into()
        }
    );
    assert_eq!(
        lines[2],
        TodoLine::Exec {
            command: "echo hi".into()
        }
    );
    assert_eq!(
        lines[3],
        TodoLine::Command {
            action: TodoAction::Drop,
            oid: "3333333".into(),
            subject: "gone".into()
        }
    );
}

#[test]
fn render_and_parse_round_trip() {
    let lines = parse_todo("pick aaa one\nsquash bbb two\n");
    assert_eq!(render_todo(&lines), "pick aaa one\nsquash bbb two\n");
}

/// The todo git writes for the same range under `--update-refs`, copied
/// from a run of it (2.51, measured): one line per local branch inside
/// the range, none for the tag, and a **comment** where another working
/// copy has the branch checked out.
const GENERATED: &str = "\
pick 987d296 # c2
update-ref refs/heads/inside-a

pick 87156cc # c3
# Ref refs/heads/held checked out at 'C:/tmp/ur-held'

pick 5e34748 # c4
update-ref refs/heads/inside-b

pick 3c37418 # c5

# Rebase d9df930..3c37418 onto d9df930 (6 commands)
# Commands:
# p, pick <commit> = use commit
";

fn full(short: &str) -> String {
    format!("{short}{}", "0".repeat(40 - short.len()))
}

/// **What decides which refs follow a rewrite is git's**, and the plan is
/// written over its todo rather than in place of it: the `update-ref`
/// lines it put there travel with the commit they came after. Written
/// whole the way this used to be, the flag was passed and nothing
/// followed (P3-確認事項 §A).
#[test]
fn the_plan_carries_over_the_ref_lines_git_wrote() {
    let plan = format!(
        "pick {} c2\nreword {} c3\nexec git commit --amend --file m\npick {} c4\npick {} c5\n",
        full("987d296"),
        full("87156cc"),
        full("5e34748"),
        full("3c37418")
    );
    let merged = merge_todo(&plan, GENERATED);
    assert_eq!(
        merged.text,
        format!(
            "pick {} c2\nupdate-ref refs/heads/inside-a\nreword {} c3\n\
             exec git commit --amend --file m\npick {} c4\nupdate-ref refs/heads/inside-b\n\
             pick {} c5\n",
            full("987d296"),
            full("87156cc"),
            full("5e34748"),
            full("3c37418")
        ),
        "each ref line follows the commit git wrote it after — and a reword's own \
         exec comes first, since that is what leaves HEAD where the ref lands"
    );
    assert!(merged.orphaned.is_empty());
    assert!(
        !merged.text.contains("checked out at"),
        "the comment goes: git reads none of them, and the branch it names is one \
         git deliberately gave no ref line"
    );
}

/// A row moved takes its ref with it, which is what a hand editing the
/// file would leave behind — and a dropped row keeps it, so the branch
/// lands where the commit it was on used to be.
#[test]
fn a_ref_line_travels_with_the_commit_it_was_written_after() {
    let plan = format!(
        "pick {} c4\npick {} c2\ndrop {} c3\npick {} c5\n",
        full("5e34748"),
        full("987d296"),
        full("87156cc"),
        full("3c37418")
    );
    let merged = merge_todo(&plan, GENERATED);
    let lines: Vec<&str> = merged.text.lines().collect();
    assert_eq!(lines[1], "update-ref refs/heads/inside-b");
    assert_eq!(lines[3], "update-ref refs/heads/inside-a");
    assert_eq!(lines.len(), 6, "and nothing else was added: {lines:?}");
}

/// A line git wrote for a commit the plan says nothing about is left out
/// and named, rather than carried to wherever the walk happened to end:
/// the range moved between the plan and the spawn, and a ref put at the
/// tip is one nobody asked to move.
#[test]
fn a_ref_line_with_no_commit_left_is_reported_rather_than_moved() {
    let plan = format!("pick {} c2\npick {} c5\n", full("987d296"), full("3c37418"));
    let merged = merge_todo(&plan, GENERATED);
    assert_eq!(merged.orphaned, vec!["update-ref refs/heads/inside-b"]);
    assert!(!merged.text.contains("inside-b"));
    assert!(
        merged.text.contains("update-ref refs/heads/inside-a"),
        "the one whose commit is still in the plan is kept: {}",
        merged.text
    );
}

#[test]
fn editor_command_is_shell_quoted_with_forward_slashes() {
    let cmd = sequence_editor_command(
        Path::new(r"C:\Program Files\pg\platitude-gg.exe"),
        Path::new(r"C:\repo\.git\platitude\rebase-todo-1"),
    );
    assert_eq!(
        cmd,
        "'C:/Program Files/pg/platitude-gg.exe' --todo-editor \
         'C:/repo/.git/platitude/rebase-todo-1'"
    );
}

#[test]
fn single_quotes_in_a_path_are_escaped() {
    assert_eq!(sh_quote("it's"), r"'it'\''s'");
}

#[test]
fn the_helper_must_sit_in_the_directory_it_is_looked_for_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let error = helper_in(dir.path()).expect_err("nothing there yet");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(
        error.to_string().contains(HELPER_NAME),
        "the message names what packaging must ship: {error}"
    );

    let placed = dir
        .path()
        .join(format!("{HELPER_NAME}{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&placed, b"").expect("place helper");
    assert_eq!(helper_in(dir.path()).expect("found"), placed);
}

#[test]
fn a_parent_header_is_read_out_of_the_headers_alone() {
    // A shallow clone's edge keeps in the stored object the header its
    // parsed `%P` has lost; the history's first commit never had one.
    assert!(has_parent_header(
        b"tree aaa\nparent bbb\nauthor a <a@e> 1 +0000\n\nsubject\n"
    ));
    assert!(!has_parent_header(
        b"tree aaa\nauthor a <a@e> 1 +0000\n\nsubject\n"
    ));
    // A message that opens with the word is past the blank line, so it is
    // no header — and reading one there would refuse a rebase from a
    // perfectly good first commit.
    assert!(!has_parent_header(
        b"tree aaa\nauthor a <a@e> 1 +0000\n\nparent process died\n"
    ));
    // git writes the object with LF even on Windows (measured), and a CR
    // does not hide the header either: taking an edge for the root is the
    // costly direction of this answer.
    assert!(has_parent_header(
        b"tree aaa\r\nparent bbb\r\n\r\nsubject\r\n"
    ));
    // A merge has two, and either says there is a parent.
    assert!(has_parent_header(
        b"tree aaa\nparent bbb\nparent ccc\n\ns\n"
    ));
}

#[test]
fn apply_plan_writes_the_plan_over_the_todo_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let plan = dir.path().join("plan");
    let todo = dir.path().join("todo");
    std::fs::write(&plan, "pick aaa one\n").expect("write plan");
    std::fs::write(&todo, "pick aaa one\npick bbb two\n").expect("write todo");
    apply_plan(&plan, &todo).expect("apply");
    assert_eq!(
        std::fs::read_to_string(&todo).expect("read"),
        "pick aaa one\n"
    );
}

/// **A plan the size of a history, reordered end to end.** The ids are
/// matched by the nine characters git abbreviates to, the lines it wrote
/// stay with their own commits wherever those went, and nothing is left
/// over. Both halves of that were a scan before — one over git's lines per
/// line of the plan, and one closing the gap each taken line left — so the
/// cost went with the square of the range (ci/baseline の
/// code-costs-windows-x64.md §todo の突き合わせ).
#[test]
fn a_reordered_plan_the_size_of_a_history_keeps_every_ref_with_its_commit() {
    // Leading characters that differ, the way a real abbreviation does: a
    // zero-padded counter shares its first nine with every other one.
    fn oid(i: usize) -> String {
        let h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        format!("{h:016x}{i:024x}")
    }
    const ROWS: usize = 20_000;
    const EVERY: usize = 10;

    let mut generated = String::new();
    for i in 0..ROWS {
        generated.push_str(&format!("pick {} subject {i}\n", &oid(i)[..9]));
        if i % EVERY == 0 {
            generated.push_str(&format!("update-ref refs/heads/b{i}\n"));
        }
    }
    // Back to front: the order the old scan paid most for, since every row
    // it took came from the far end of what was left.
    let mut plan = String::new();
    for i in (0..ROWS).rev() {
        plan.push_str(&format!("pick {} subject {i}\n", oid(i)));
    }

    let merged = merge_todo(&plan, &generated);
    assert!(merged.orphaned.is_empty(), "{:?}", merged.orphaned);
    let lines: Vec<&str> = merged.text.lines().collect();
    assert_eq!(lines.len(), ROWS + ROWS / EVERY);
    for (at, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("update-ref refs/heads/b") else {
            continue;
        };
        let owner: usize = rest.parse().expect("the branch names its commit");
        assert_eq!(
            lines[at - 1],
            format!("pick {} subject {owner}", oid(owner)),
            "the ref line follows its own commit wherever the plan put it"
        );
    }
}
