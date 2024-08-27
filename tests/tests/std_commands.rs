#![cfg(unix)]

mod testing;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    time::SystemTime,
};

use blaze_core::{common::selector::ProjectSelector, run, RunOptions, SelectorSource};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use testing::{with_test_workspace, TestWorkspaceConfiguration};

use crate::testing::{Executions, ExpectedExecution};

fn setup(commands_options: serde_json::Value) -> TestWorkspaceConfiguration {
    TestWorkspaceConfiguration::new(
        json!(
            {
                "name": "workspace",
                "projects": {
                    "project": "project"
                }
            }
        ),
        [(
            "project",
            json!(
                {
                    "targets": {
                        "commands": {
                            "executor": "std:commands",
                            "options": commands_options
                        }
                    }
                }
            ),
        )],
        [],
    )
}

#[test]
fn touch() {
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "touch",
                    "arguments": ["test"]
                }
            ]
        })),
        |root| {
            run_and_check_result(root, true);

            let created_file = std::fs::metadata(root.join("project/test"))
                .expect("could not read test file metadata");

            assert!(created_file.is_file());
        },
    );
}

#[test]
fn detached_fifo() {
    let fifo_path = std::env::temp_dir().join(format!(
        "blaze_test_fifo_{}",
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect::<String>()
    ));
    let fifo_path_string = fifo_path.to_str().unwrap().to_owned();
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "mkfifo",
                    "arguments": [fifo_path_string]
                },
                {
                    "program": "cat",
                    "arguments": [fifo_path_string, ">", "test"],
                    "detach": true
                },
                {
                    "program": "echo",
                    "arguments": ["-n", "Hello world!", ">", fifo_path_string]
                },
                {
                    "program": "rm",
                    "arguments": [fifo_path_string]
                }
            ],
            "shell": "sh"
        })),
        |root| {
            run_and_check_result(root, true);

            assert_eq!(
                b"Hello world!",
                &std::fs::read(root.join("project/test")).expect("could not read result file")[..]
            );
        },
    )
}

#[test]
fn command_failure() {
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "false"
                }
            ]
        })),
        |root| run_and_check_result(root, false),
    );
}

#[test]
fn ignore_failure() {
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "false",
                    "onFailure": "Ignore"
                }
            ]
        })),
        |root| run_and_check_result(root, true),
    );
}

#[test]
fn force_exit() {
    let start = SystemTime::now();
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "sleep",
                    "arguments": ["30"],
                    "detach": true
                },
                {
                    "program": "false",
                    "onFailure": "ForceExit"
                }
            ]
        })),
        |root| {
            run_and_check_result(root, false);
            assert!(
                SystemTime::now().duration_since(start).unwrap().as_secs() < 30,
                "sleep process should have been killed by the main process before exiting"
            );
        },
    );
}

#[test]
fn restart() {
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "sh",
                    "arguments": ["-c", "if test -f 'foo' ; then true; else touch 'foo'; false; fi"],
                    "onFailure": "Restart"
                }
            ]
        })),
        |root| run_and_check_result(root, true),
    );
}

#[test]
fn with_env() {
    with_test_workspace(
        setup(json!({
            "commands": [
                {
                    "program": "echo",
                    "arguments": [
                        "-n",
                        "$TEST_VAR",
                        ">",
                        "test.txt"
                    ],
                    "environment": {
                        "TEST_VAR": "Hello world!"
                    },
                    "cwd": "{{ project.root }}"
                }
            ],
            "shell": "sh"
        })),
        |root| {
            run_and_check_result(root, true);
            let output = std::fs::read_to_string(root.join("project/test.txt"))
                .expect("could not read test output");
            assert_eq!("Hello world!", output);
        },
    )
}

#[test]
fn default_env() {
    let commands: Vec<serde_json::Value> = [
        "BLAZE_WORKSPACE_NAME",
        "BLAZE_WORKSPACE_ROOT",
        "BLAZE_WORKSPACE_CONFIGURATION_FILE_PATH",
        "BLAZE_WORKSPACE_CONFIGURATION_FILE_FORMAT",
        "BLAZE_PROJECT_NAME",
        "BLAZE_PROJECT_ROOT",
        "BLAZE_TARGET",
    ]
    .into_iter()
    .map(|var| {
        json!({
            "program": "echo",
            "arguments": [
                format!("\"{var}=${var}\""),
                ">>",
                "test.txt",
            ]
        })
    })
    .collect();

    with_test_workspace(
        setup(json!({
            "commands": commands,
            "shell": "sh"
        })),
        |root| {
            run_and_check_result(root, true);
            let test_file =
                File::open(root.join("project/test.txt")).expect("could not open test file");
            let results = BufReader::new(test_file)
                .lines()
                .map(|line| {
                    line.expect("could not read line from test file")
                        .split_once("=")
                        .map(|(name, value)| (name.to_owned(), value.to_owned()))
                        .expect("could not parse line from test file")
                })
                .collect::<Vec<(String, String)>>();

            let test_var = |expected_name: &str, expected_value: &str| {
                assert!(
                    results.iter().any(|(name, value)| {
                        name.as_str() == expected_name && value.as_str() == expected_value
                    }),
                    "{expected_name} does not have value {expected_value}"
                );
            };

            test_var("BLAZE_WORKSPACE_NAME", "workspace");
            test_var("BLAZE_WORKSPACE_ROOT", root.to_str().unwrap());
            test_var(
                "BLAZE_WORKSPACE_CONFIGURATION_FILE_PATH",
                root.join("workspace.json").to_str().unwrap(),
            );
            test_var("BLAZE_WORKSPACE_CONFIGURATION_FILE_FORMAT", "Json");
            test_var("BLAZE_PROJECT_NAME", "project");
            test_var("BLAZE_PROJECT_ROOT", root.join("project").to_str().unwrap());
            test_var("BLAZE_TARGET", "commands");
        },
    )
}

fn run_and_check_result(root: &Path, expect_success: bool) {
    let results = run(
        root,
        RunOptions::new("commands").with_selector_source(SelectorSource::Provided(
            ProjectSelector::array(["project"]),
        )),
        Default::default(),
    );

    Executions::from_run_result(results).assert_targets([(
        "project:commands",
        if expect_success {
            ExpectedExecution::success()
        } else {
            ExpectedExecution::failure()
        },
    )]);
}
