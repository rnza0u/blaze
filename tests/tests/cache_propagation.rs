use blaze_core::{
    common::selector::ProjectSelector, run, GlobalOptions, RunOptions, SelectorSource,
};
use serde_json::json;
use testing::{with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration};

mod testing;

#[test]
fn always() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace-name",
                "projects": {
                    "project-name": "project-root",
                }
            }),
            [(
                "project-root",
                json!({
                    "targets": {
                        "a": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> '{{ project.root }}/a.txt'"
                                ],
                                "shell": true
                            },
                            "dependencies": [
                                {
                                    "target": "b"
                                }
                            ],
                            "cache": {}
                        },
                        "b": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> '{{ project.root }}/b.txt'"
                                ],
                                "shell": true
                            },
                            "cache": {
                                "invalidateWhen": {
                                    "commandFails": {
                                        "program": "/bin/sh",
                                        "arguments": [
                                            "{{ project.root }}/invalidate.sh"
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
            let invalidate = "#!/bin/sh
set -e
false
";

            let no_invalidate = "#!/bin/sh\n";

            let invalidation_script_path = root.join("project-root/invalidate.sh");

            std::fs::write(&invalidation_script_path, no_invalidate)
                .expect("could not write to invalidation script");

            let run = || {
                Executions::from_run_result(run(
                    root,
                    RunOptions::new("a").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project-name"]),
                    )),
                    GlobalOptions::default(),
                ))
            };

            run().assert_targets([
                ("project-name:a", ExpectedExecution::success()),
                ("project-name:b", ExpectedExecution::success()),
            ]);

            run().assert_targets([
                ("project-name:a", ExpectedExecution::cached()),
                ("project-name:b", ExpectedExecution::cached()),
            ]);

            std::fs::write(&invalidation_script_path, invalidate)
                .expect("could not write to invalidation script");

            run().assert_targets([
                ("project-name:a", ExpectedExecution::success()),
                ("project-name:b", ExpectedExecution::success()),
            ]);

            assert_eq!(
                "Hello world!\n".repeat(2),
                std::fs::read_to_string(root.join("project-root/a.txt"))
                    .expect("could not read target a history file")
            );

            assert_eq!(
                "Hello world!\n".repeat(2),
                std::fs::read_to_string(root.join("project-root/b.txt"))
                    .expect("could not read target b history file")
            );
        },
    )
}

#[test]
fn never() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace-name",
                "projects": {
                    "project-name": "project-root",
                }
            }),
            [(
                "project-root",
                json!({
                    "targets": {
                        "a": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> '{{ project.root }}/a.txt'"
                                ],
                                "shell": true
                            },
                            "dependencies": [
                                {
                                    "target": "b",
                                    "cachePropagation": "Never"
                                }
                            ],
                            "cache": {}
                        },
                        "b": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    "echo 'Hello world!' >> '{{ project.root }}/b.txt'"
                                ],
                                "shell": true
                            },
                            "cache": {
                                "invalidateWhen": {
                                    "commandFails": {
                                        "program": "/bin/sh",
                                        "arguments": [
                                            "{{ project.root }}/invalidate.sh"
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
            let invalidate = "#!/bin/sh
set -e
false
";

            let no_invalidate = "#!/bin/sh\n";

            let invalidation_script_path = root.join("project-root/invalidate.sh");

            std::fs::write(&invalidation_script_path, no_invalidate)
                .expect("could not write to invalidation script");

            let run = || {
                Executions::from_run_result(run(
                    root,
                    RunOptions::new("a").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project-name"]),
                    )),
                    GlobalOptions::default(),
                ))
            };

            run().assert_targets([
                ("project-name:a", ExpectedExecution::success()),
                ("project-name:b", ExpectedExecution::success()),
            ]);

            run().assert_targets([
                ("project-name:a", ExpectedExecution::cached()),
                ("project-name:b", ExpectedExecution::cached()),
            ]);

            std::fs::write(&invalidation_script_path, invalidate)
                .expect("could not write to invalidation script");

            run().assert_targets([
                ("project-name:a", ExpectedExecution::cached()),
                ("project-name:b", ExpectedExecution::success()),
            ]);

            assert_eq!(
                "Hello world!\n",
                std::fs::read_to_string(root.join("project-root/a.txt"))
                    .expect("could not read target a history file")
            );

            assert_eq!(
                "Hello world!\n".repeat(2),
                std::fs::read_to_string(root.join("project-root/b.txt"))
                    .expect("could not read target b history file")
            );
        },
    )
}
