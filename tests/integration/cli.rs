use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_core_commands() {
    let mut command = Command::cargo_bin("cc-profile").expect("binary exists");

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("use"))
        .stdout(predicate::str::contains("show"));
}
