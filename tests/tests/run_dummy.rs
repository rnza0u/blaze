mod testing;

use blaze_core::{common::selector::ProjectSelector, run, RunOptions, SelectorSource};
use serde_json::json;

use crate::testing::{
    with_test_workspace, Executions, ExpectedExecution, TestWorkspaceConfiguration,
};

fn setup() -> TestWorkspaceConfiguration {
    TestWorkspaceConfiguration::new(
        json!(
            {
                "name": "dummy-workspace",
                "projects": {
                    "dummy-project": "dummy-project"
                }
            }
        ),
        [(
            "dummy-project",
            json!(
                {
                    "targets": {
                        "dummy-target": {}
                    }
                }
            ),
        )],
        [],
    )
}

#[test]
fn dummy_success() {
    with_test_workspace(setup(), |root| {
        let results = run(
            root,
            RunOptions::new("dummy-target").with_selector_source(SelectorSource::Provided(
                ProjectSelector::array(["dummy-project"]),
            )),
            Default::default(),
        );

        Executions::from_run_result(results)
            .assert_targets([("dummy-project:dummy-target", ExpectedExecution::success())]);
    });
}
