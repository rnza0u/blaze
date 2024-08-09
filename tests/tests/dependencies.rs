use blaze_core::{common::selector::ProjectSelector, run, RunOptions, SelectorSource};
use serde_json::json;
use testing::{with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration};

mod testing;

/*
├── a
|   ├── b
|   |   ├── d
|   ├── c
*/
#[test]
fn three_levels() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace",
                "projects": {
                    "project-a": "project-a",
                    "project-b": "project-b",
                    "project-c": "project-c",
                    "project-d": "project-d"
                }
            }),
            [
                (
                    "project-a",
                    json!({
                        "targets": {
                            "build": {
                                "dependencies": [
                                    {
                                        "target": "build",
                                        "projects": ["project-b", "project-c"]
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "project-b",
                    json!({
                        "targets": {
                            "build": {
                                "dependencies": [
                                    {
                                        "target": "build",
                                        "projects": ["project-d"]
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "project-c",
                    json!({
                        "targets": {
                            "build": {}
                        }
                    }),
                ),
                (
                    "project-d".into(),
                    json!({
                        "targets": {
                            "build": {}
                        }
                    }),
                ),
            ],
            [],
        ),
        |root| {
            let result = run(
                root,
                RunOptions::new("build").with_selector_source(SelectorSource::Provided(
                    ProjectSelector::array(["project-a"]),
                )),
                Default::default(),
            );

            Executions::from_run_result(result).assert_targets([
                ("project-a:build", ExpectedExecution::success()),
                ("project-b:build", ExpectedExecution::success()),
                ("project-c:build", ExpectedExecution::success()),
                ("project-d:build", ExpectedExecution::success()),
            ]);
        },
    )
}

// for this test, the graph looks like that
//
//      a ----> b ----> e
//      ^       |
//      |       ⌄
//      d <---- c ----> f
//
// it is circular, so it cannot be executed.
#[test]
fn circular_dependencies() {
    with_test_workspace(
        TestWorkspaceConfiguration {
            workspace: json!({
                "project": {
                    "a": "a",
                    "b": "b",
                    "c": "c",
                    "d": "d",
                    "e": "e",
                    "f": "f"
                }
            }),
            projects: [
                (
                    "a".into(),
                    json!({
                        "targets": {
                            "t": {
                                "dependencies": [
                                    {
                                        "projects": ["b"],
                                        "target": "t"
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "b".into(),
                    json!({
                        "targets": {
                            "t": {
                                "dependencies": [
                                    {
                                        "projects": ["c", "e"],
                                        "target": "t"
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "c".into(),
                    json!({
                        "targets": {
                            "t": {
                                "dependencies": [
                                    {
                                        "projects": ["d", "f"],
                                        "target": "t"
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "d".into(),
                    json!({
                        "targets": {
                            "t": {
                                "dependencies": [
                                    {
                                        "projects": ["a"],
                                        "target": "t"
                                    }
                                ]
                            }
                        }
                    }),
                ),
                (
                    "e".into(),
                    json!({
                        "targets": {
                            "t": {}
                        }
                    }),
                ),
                (
                    "f".into(),
                    json!({
                        "targets": {
                            "t": {}
                        }
                    }),
                ),
            ]
            .into(),
            ..Default::default()
        },
        |root| {
            let results = run(
                root,
                RunOptions::new("t")
                    .with_selector_source(SelectorSource::Provided(ProjectSelector::array(["a"]))),
                Default::default(),
            );
            Executions::from_run_result(results).assert_global_failure();
        },
    )
}

#[test]
fn failed_dependency() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "a",
                "projects": {
                    "a": "a"
                }
            }),
            [(
                "a",
                json!({
                    "targets": {
                        "a": {
                            "dependencies": [
                                {
                                    "target": "b"
                                }
                            ]
                        },
                        "b": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    { "program": "false" }
                                ]
                            }
                        }
                    }
                }),
            )],
            [],
        ),
        |root| {
            let results = Executions::from_run_result(run(
                root,
                RunOptions::new("a")
                    .with_selector_source(SelectorSource::Provided(ProjectSelector::array(["a"]))),
                Default::default(),
            ));
            results.assert_targets([
                ("a:a", ExpectedExecution::not_executed()),
                ("a:b", ExpectedExecution::failure()),
            ])
        },
    )
}

#[test]
fn optional_dependency() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "a",
                "projects": {
                    "a": "a"
                }
            }),
            [(
                "a",
                json!({
                    "targets": {
                        "a": {
                            "dependencies": [
                                {
                                    "target": "b",
                                    "optional": true
                                }
                            ]
                        },
                        "b": {
                            "executor": "std:commands",
                            "options": {
                                "commands": [
                                    { "program": "false" }
                                ]
                            }
                        }
                    }
                }),
            )],
            [],
        ),
        |root| {
            let results = Executions::from_run_result(run(
                root,
                RunOptions::new("a")
                    .with_selector_source(SelectorSource::Provided(ProjectSelector::array(["a"]))),
                Default::default(),
            ));
            results.assert_targets([
                ("a:a", ExpectedExecution::success()),
                ("a:b", ExpectedExecution::failure()),
            ])
        },
    )
}
