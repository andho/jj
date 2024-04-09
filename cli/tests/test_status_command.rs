// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::common::TestEnvironment;

#[test]
fn test_status_merge() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file"), "base").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["new", "-m=left"]);
    test_env.jj_cmd_ok(&repo_path, &["branch", "create", "left"]);
    test_env.jj_cmd_ok(&repo_path, &["new", "@-", "-m=right"]);
    std::fs::write(repo_path.join("file"), "right").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["new", "left", "@"]);

    // The output should mention each parent, and the diff should be empty (compared
    // to the auto-merged parents)
    let stdout = test_env.jj_cmd_success(&repo_path, &["status"]);
    insta::assert_snapshot!(stdout, @r###"
    The working copy is clean
    Working copy : mzvwutvl c965365c (empty) (no description set)
    Parent commit: rlvkpnrz 9ae48ddb left | (empty) left
    Parent commit: zsuskuln 29b991e9 right
    "###);
}

// See https://github.com/martinvonz/jj/issues/2051.
#[test]
fn test_status_ignored_gitignore() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::create_dir(repo_path.join("untracked")).unwrap();
    std::fs::write(repo_path.join("untracked").join("inside_untracked"), "test").unwrap();
    std::fs::write(
        repo_path.join("untracked").join(".gitignore"),
        "!inside_untracked\n",
    )
    .unwrap();
    std::fs::write(repo_path.join(".gitignore"), "untracked/\n!dummy\n").unwrap();

    let stdout = test_env.jj_cmd_success(&repo_path, &["status"]);
    insta::assert_snapshot!(stdout, @r###"
    Working copy changes:
    A .gitignore
    Working copy : qpvuntsm 88a40909 (no description set)
    Parent commit: zzzzzzzz 00000000 (empty) (no description set)
    "###);
}

#[test]
fn test_status_filtered() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file_1"), "file_1").unwrap();
    std::fs::write(repo_path.join("file_2"), "file_2").unwrap();

    // The output filtered to file_1 should not list the addition of file_2.
    let stdout = test_env.jj_cmd_success(&repo_path, &["status", "file_1"]);
    insta::assert_snapshot!(stdout, @r###"
    Working copy changes:
    A file_1
    Working copy : qpvuntsm abcaaacd (no description set)
    Parent commit: zzzzzzzz 00000000 (empty) (no description set)
    "###);
}

// See <https://github.com/martinvonz/jj/issues/3108>
#[test]
fn test_status_display_rebase_instructions() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);

    let repo_path = test_env.env_root().join("repo");
    let conflicted_path = repo_path.join("conflicted.txt");

    // PARENT: Write the initial file
    std::fs::write(&conflicted_path, "initial contents").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["describe", "--message", "Initial contents"]);

    // CHILD1: New commit on top of <PARENT>
    test_env.jj_cmd_ok(
        &repo_path,
        &["new", "--message", "First part of conflicting change"],
    );
    std::fs::write(&conflicted_path, "Child 1").unwrap();

    // CHILD2: New commit also on top of <PARENT>
    test_env.jj_cmd_ok(
        &repo_path,
        &[
            "new",
            "--message",
            "Second part of conflicting change",
            "@-",
        ],
    );
    std::fs::write(&conflicted_path, "Child 2").unwrap();

    // CONFLICT: New commit that is conflicted by merging <CHILD1> and <CHILD2>
    test_env.jj_cmd_ok(&repo_path, &["new", "--message", "boom", "all:(@-)+"]);
    // Adding more descendants to ensure we correctly find the root ancestors with
    // conflicts, not just the parents.
    test_env.jj_cmd_ok(&repo_path, &["new", "--message", "boom-cont"]);
    test_env.jj_cmd_ok(&repo_path, &["new", "--message", "boom-cont-2"]);

    let stdout = test_env.jj_cmd_success(&repo_path, &["log", "-r", "::@"]);

    insta::assert_snapshot!(stdout, @r###"
    @  yqosqzyt test.user@example.com 2001-02-03 08:05:13 d41fb742 conflict
    │  (empty) boom-cont-2
    ◉  royxmykx test.user@example.com 2001-02-03 08:05:12 c2389cec conflict
    │  (empty) boom-cont
    ◉    mzvwutvl test.user@example.com 2001-02-03 08:05:11 c57f2050 conflict
    ├─╮  (empty) boom
    │ ◉  kkmpptxz test.user@example.com 2001-02-03 08:05:10 55ce6709
    │ │  First part of conflicting change
    ◉ │  zsuskuln test.user@example.com 2001-02-03 08:05:11 ba5f8773
    ├─╯  Second part of conflicting change
    ◉  qpvuntsm test.user@example.com 2001-02-03 08:05:08 98e0dcf8
    │  Initial contents
    ◉  zzzzzzzz root() 00000000
    "###);

    let stdout = test_env.jj_cmd_success(&repo_path, &["status"]);

    insta::assert_snapshot!(stdout, @r###"
    The working copy is clean
    There are unresolved conflicts at these paths:
    conflicted.txt    2-sided conflict
    Working copy : yqosqzyt d41fb742 (conflict) (empty) boom-cont-2
    Parent commit: royxmykx c2389cec (conflict) (empty) boom-cont
    To resolve the conflicts, start by updating to the first one:
      jj new mzvwutvlkqwt
    Then use `jj resolve`, or edit the conflict markers in the file directly.
    Once the conflicts are resolved, you may want inspect the result with `jj diff`.
    Then run `jj squash` to move the resolution into the conflicted commit.
    "###);
}
