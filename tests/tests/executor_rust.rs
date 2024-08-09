use blaze_core::{common::selector::ProjectSelector, run, RunOptions};
use serde_json::json;
use testing::{
    get_fixtures_root, with_test_workspace, Executions, ExpectedExecution,
    TestWorkspaceConfiguration,
};

mod testing;

#[test]
#[cfg(not(target_env = "musl"))]
fn rust_checker() {
    use blaze_core::SelectorSource;

    let executor_root = get_fixtures_root().join("executors/rust-checker");
    let executor_root_str = executor_root.to_str().unwrap();

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
                            "executor": format!("file://{}", executor_root_str),
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
            let results = run(
                root,
                RunOptions::new("target").with_selector_source(SelectorSource::Provided(
                    ProjectSelector::array(["project"]),
                )),
                Default::default(),
            );

            Executions::from_run_result(results)
                .assert_targets([("project:target", ExpectedExecution::success())]);
        },
    );
}
