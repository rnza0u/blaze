use blaze_core::time::now;
use blaze_core::{common::selector::ProjectSelector, run, GlobalOptions, RunOptions};
use once_cell::sync::Lazy;
use testing::{with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration};

use serde_json::json;

use std::{fs::File, time::UNIX_EPOCH};

mod testing;

const SCRIPT: &str = "set -e;
echo -n \"{\
\\\"project\\\":\\\"$BLAZE_PROJECT\\\",\
\\\"target\\\":\\\"$BLAZE_TARGET\\\",\
\\\"options\\\": $BLAZE_OPTIONS,\
\" > '{{ project.root }}/vars';
if [ -n \"$BLAZE_FRESH_EXECUTION\" ]; then echo -n \"\\\"freshExecution\\\":true\" >> '{{ project.root }}/vars'; fi;
if [ -n \"$BLAZE_LAST_EXECUTION_TIME\" ]; then echo -n \"\\\"lastExecutionAt\\\":$BLAZE_LAST_EXECUTION_TIME\" >> '{{ project.root }}/vars'; fi;
echo -n '}' >> '{{ project.root }}/vars';
test -f '{{ project.root }}/artifact.bin';";

const ARTEFACT_OUTPUT: &str = "some compiled code";

static OPTIONS: Lazy<serde_json::Value> = Lazy::new(|| {
    json!({
        "commands": [
            format!("echo -n '{ARTEFACT_OUTPUT}' >> artifact.bin")
        ],
        "shell": true
    })
});

#[cfg(not(windows))]
#[test]
fn command_fails() {
    use blaze_core::SelectorSource;

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
                            "options": *OPTIONS,
                            "cache": {
                                "invalidateWhen": {
                                    "commandFails": {
                                        "program": "/bin/sh",
                                        "arguments": [
                                            "-c",
                                            SCRIPT
                                        ]
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

            let read_vars = || {
                serde_json::from_reader::<_, serde_json::Value>(
                    File::open(root.join("project-root/vars"))
                        .expect("could not open vars test file"),
                )
                .expect("could not deserialize vars")
            };

            let check_common = |vars: &serde_json::Value| {
                assert_eq!(Some(&*OPTIONS), vars.pointer("/options"));
                assert_eq!(
                    Some(&serde_json::Value::String("build".into())),
                    vars.pointer("/target")
                );
                assert_eq!(
                    Some(&serde_json::Value::String("project-name".into())),
                    vars.pointer("/project")
                );
            };

            let vars = read_vars();
            check_common(&vars);
            assert!(vars.pointer("/freshExecution").is_some());
            assert!(vars.pointer("/lastExecutionAt").is_none());

            assert_eq!(
                ARTEFACT_OUTPUT,
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );

            let second_execution = run_cached_target();
            second_execution.assert_targets([("project-name:build", ExpectedExecution::cached())]);

            let vars = read_vars();
            check_common(&vars);
            assert!(vars.pointer("/freshExecution").is_none());
            assert_eq!(
                Some(&serde_json::Value::Number(serde_json::Number::from(
                    now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
                ))),
                vars.pointer("/lastExecutionAt")
            );

            assert_eq!(
                ARTEFACT_OUTPUT,
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );

            std::fs::remove_file(&artifact_path).expect("could not remove test artifact");

            let last_execution = run_cached_target();
            last_execution.assert_targets([("project-name:build", ExpectedExecution::success())]);

            let vars = read_vars();
            check_common(&vars);
            assert!(vars.pointer("/freshExecution").is_none());
            assert_eq!(
                Some(&serde_json::Value::Number(serde_json::Number::from(
                    now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
                ))),
                vars.pointer("/lastExecutionAt")
            );

            assert_eq!(
                ARTEFACT_OUTPUT,
                std::fs::read_to_string(&artifact_path).expect("failed to read test file")
            );
        },
    )
}
