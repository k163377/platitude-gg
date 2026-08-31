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
fn apply_plan_overwrites_the_todo_file() {
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
