use blaze_core::{common::selector::ProjectSelector, run, RunOptions, SelectorSource};
use serde_json::json;
use testing::{get_fixtures_root, with_test_workspace, TestWorkspaceConfiguration};

use crate::testing::{Executions, ExpectedExecution};

mod testing;

#[cfg(node)]
#[test]
fn node_checker_fs() {
    use testing::cmd;

    #[cfg(windows)]
    let executor_root = get_fixtures_root().join("executors\\node-checker");

    #[cfg(not(windows))]
    let executor_root = get_fixtures_root().join("executors/node-checker");

    let executor_root_str = executor_root.to_str().unwrap();

    cmd(format!(
        "cd {executor_root_str} && rm -rf node_modules package-lock.json dist build_hash"
    ));

    let executor_url = format!("file://{executor_root_str}");

    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "workspace",
                "projects": {
                    "project": "project"
                },
                "settings": {
                    "logLevel": "Debug"
                }
            }),
            [(
                "project",
                json!({
                    "targets": {
                        "target": {
                            "executor": executor_url,
                            "options": {
                                "number": 1,
                                "string": "hello",
                                "bool": true,
                                "array": [1, 2, 3],
                                "null": null,
                                "float": 1.0
                            }
                        }
                    }
                }),
            )],
            [],
        ),
        |root| {
            let execute = || {
                let results = run(
                    root,
                    RunOptions::new("target").with_selector_source(SelectorSource::Provided(
                        ProjectSelector::array(["project"]),
                    )),
                    Default::default(),
                );

                Executions::from_run_result(results)
                    .assert_targets([("project:target", ExpectedExecution::success())]);
            };

            let read_build_hash = || {
                std::fs::read_to_string(executor_root.join("build_hash"))
                    .expect("could not read build hash")
            };

            execute();

            let build_hash = read_build_hash();

            execute();

            assert_eq!(read_build_hash(), build_hash);
        },
    )
}
