use blaze_core::time::{now, set_current_time};
use blaze_core::{common::selector::ProjectSelector, run, GlobalOptions, RunOptions};

use std::time::Duration;
use testing::{with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration};

use serde_json::json;

mod testing;

#[cfg(not(windows))]
#[test]
fn single_input_file() {
    use blaze_core::SelectorSource;
    use filetime::{set_file_mtime, FileTime};

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
                        "target-name": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> {{ project.root }}/file.txt"
                                ],
                                "shell": true
                            },
                            "cache": {
                                "invalidateWhen": {
                                    "inputChanges": [
                                        {
                                            "pattern": "source.txt"
                                        }
                                    ]
                                }
                            }
                        }
                    }
                }),
            )],
            [("files/source.txt", "project-root/")],
        ),
        |root| {
            let run_cached_target = || -> Executions {
                let results = run(
                    root,
                    RunOptions::new("target-name").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project-name"]),
                    )),
                    GlobalOptions::default(),
                );
                Executions::from_run_result(results)
            };

            let cached_file_path = root.join("project-root/source.txt");
            let test_file_path = root.join("project-root/file.txt");

            set_file_mtime(&cached_file_path, FileTime::from_system_time(now()))
                .expect("could not set modified date");

            let first_execution = run_cached_target();
            first_execution
                .assert_targets([("project-name:target-name", ExpectedExecution::success())]);

            let second_execution = run_cached_target();
            second_execution
                .assert_targets([("project-name:target-name", ExpectedExecution::cached())]);

            let change_time = now() + Duration::from_secs(1);
            set_current_time(change_time);
            std::fs::write(&cached_file_path, "Content has changed !\n")
                .expect("failure to write to test file");
            set_file_mtime(cached_file_path, FileTime::from_system_time(change_time))
                .expect("could not set modified date");

            let last_execution = run_cached_target();
            last_execution
                .assert_targets([("project-name:target-name", ExpectedExecution::success())]);

            assert_eq!(
                "Hello world!\nHello world!\n",
                std::fs::read_to_string(&test_file_path).expect("failed to read test file")
            );
        },
    )
}

#[cfg(not(windows))]
#[test]
fn exclude_input_files() {
    use blaze_core::SelectorSource;
    use filetime::{set_file_mtime, FileTime};

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
                        "target-name": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> {{ project.root }}/history.txt"
                                ],
                                "shell": true
                            },
                            "cache": {
                                "invalidateWhen": {
                                    "inputChanges": [
                                        {
                                            "pattern": "src/folder{1,3}/**/*",
                                            "exclude": [
                                                "**/folder3/**/file1.txt"
                                            ]
                                        }
                                    ]
                                }
                            }
                        }
                    }
                }),
            )],
            [("files/tree/**/*", "project-root/")],
        ),
        |root| {
            let run_cached_target = || -> Executions {
                let results = run(
                    root,
                    RunOptions::new("target-name").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project-name"]),
                    )),
                    GlobalOptions::default(),
                );
                Executions::from_run_result(results)
            };

            let not_included_path = root.join("project-root/src/folder2/file1.txt");

            let executions = run_cached_target();
            executions.assert_targets([("project-name:target-name", ExpectedExecution::success())]);

            set_current_time(now() + Duration::from_secs(10));
            std::fs::write(&not_included_path, "modified")
                .expect("could not write to non included file");
            set_file_mtime(&not_included_path, FileTime::from_system_time(now()))
                .expect("could not change mtime to non included file");

            let executions = run_cached_target();

            executions.assert_targets([("project-name:target-name", ExpectedExecution::cached())]);

            let excluded_path = root.join("project-root/src/folder3/file1.txt");

            set_current_time(now() + Duration::from_secs(10));
            std::fs::write(&excluded_path, "modified").expect("could not write to excluded file");
            set_file_mtime(&excluded_path, FileTime::from_system_time(now()))
                .expect("could not change mtime to excluded file");

            let executions = run_cached_target();
            executions.assert_targets([("project-name:target-name", ExpectedExecution::cached())]);

            let included_path = root.join("project-root/src/folder1/file1.txt");

            set_current_time(now() + Duration::from_secs(10));
            std::fs::write(&included_path, "modified").expect("could not write to included file");
            set_file_mtime(&included_path, FileTime::from_system_time(now()))
                .expect("could not change mtime to included file");

            let executions = run_cached_target();
            executions.assert_targets([("project-name:target-name", ExpectedExecution::success())]);

            assert_eq!(
                "Hello world!\n".repeat(2),
                std::fs::read_to_string(root.join("project-root/history.txt"))
                    .expect("could not read history file")
            );
        },
    )
}
