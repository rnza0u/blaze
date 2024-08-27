use blaze_core::time::{now, set_current_time};
use blaze_core::SelectorSource;
use blaze_core::{common::selector::ProjectSelector, run, GlobalOptions, RunOptions};
use std::time::Duration;
use testing::{with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration};

use serde_json::json;

mod testing;

#[test]
fn after_duration() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace-name",
                "projects": {
                    "project-name": "project-root"
                }
            }),
            [(
                "project-root",
                json!({
                    "targets": {
                        "build": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'some compiled code' >> '{{ project.root }}/artifact.bin'"
                                ],
                                "shell": true
                            },
                            "cache": {
                                "invalidateWhen": {
                                    "expired": {
                                        "unit": "Days",
                                        "amount": 1
                                    }
                                }
                            }
                        }
                    }
                }),
            )],
            [],
        ),
        |root| {
            let run_cached_target = || -> Executions {
                let results = run(
                    root,
                    RunOptions::new("build").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project-name"]),
                    )),
                    GlobalOptions::default(),
                );
                Executions::from_run_result(results)
            };

            let artifact_path = root.join("project-root/artifact.bin");

            let first_execution = run_cached_target();
            first_execution.assert_targets([("project-name:build", ExpectedExecution::success())]);

            assert_eq!(
                "some compiled code\n",
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );

            // 12 hours later
            set_current_time(now() + Duration::from_secs(60 * 60 * 12));

            let second_execution = run_cached_target();
            second_execution.assert_targets([("project-name:build", ExpectedExecution::cached())]);

            assert_eq!(
                "some compiled code\n",
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );

            // 24 hours later
            set_current_time(now() + Duration::from_secs(60 * 60 * 24));

            let last_execution = run_cached_target();
            last_execution.assert_targets([("project-name:build", ExpectedExecution::success())]);

            assert_eq!(
                "some compiled code\nsome compiled code\n",
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );
        },
    )
}
