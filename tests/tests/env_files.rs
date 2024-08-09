mod testing;

use blaze_core::{common::selector::ProjectSelector, RunOptions};

use crate::testing::{Executions, ExpectedExecution};

use blaze_core::run;
use serde_json::json;
use testing::{with_test_workspace, TestWorkspaceConfiguration};

// test the behavior of .env files
#[cfg(unix)]
#[test]
fn env_files() {
    use blaze_core::SelectorSource;

    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace",
                "projects": {
                    "project": "project"
                }
            }),
            [(
                "project",
                json!({
                    "targets": {
                        "target": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo \"TEST_VAR=$TEST_VAR\" > {{ project.root }}/test.txt",
                                    "echo \"USER_TEST_VAR=$USER_TEST_VAR\" >> {{ project.root }}/test.txt"
                                ],
                                "shell": true
                            }
                        }
                    }
                }),
            )],
            [("env/*", "")],
        ),
        |root| {
            let results = run(
                &root,
                RunOptions::new("target").with_selector_source(SelectorSource::Provided(
                    ProjectSelector::array(["project"]),
                )),
                Default::default(),
            );

            Executions::from_run_result(results)
                .assert_targets([("project:target", ExpectedExecution::success())]);

            assert_eq!(
                "TEST_VAR=1\nUSER_TEST_VAR=1\n",
                std::fs::read_to_string(root.join("project/test.txt"))
                    .expect("could not read test file")
            );
        },
    );
}
