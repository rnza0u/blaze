mod testing;

use std::{collections::BTreeMap, path::Path};

use blaze_core::{common::selector::ProjectSelector, run, RunOptions, SelectorSource};
use serde_json::json;
use testing::{with_test_workspace, TestWorkspaceConfiguration};

use crate::testing::{Executions, ExpectedExecution};

fn setup() -> TestWorkspaceConfiguration {
    let projects = (0..100)
        .map(|i| format!("project-{i}"))
        .collect::<Vec<String>>();
    TestWorkspaceConfiguration::new(
        json!({
            "name": "workspace",
            "projects": projects
                .iter()
                .map(|name| (name.clone(), name.clone()))
                .collect::<BTreeMap<String, String>>(),
            "settings": {
                "defaultSelector": ["project-15", "project-25", "project-35"],
                "selectors": {
                    "all": "All",
                    "include-exclude": {
                        "include": [
                            "^project-1.*",
                            "^project-2.*"
                        ],
                        "exclude": [
                            "^project-1$",
                            "^project-2$"
                        ]
                    }
                }
            }
        }),
        projects
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    json!({
                        "targets": {
                            "dummy": {}
                        }
                    }),
                )
            })
            .collect::<Vec<_>>(),
        [],
    )
}

#[test]
fn array() {
    with_test_workspace(setup(), |root| {
        run_and_verify_selected_projects(
            &root,
            Some(SelectorSource::Provided(ProjectSelector::array([
                "project-1",
                "project-12",
                "project-56",
            ]))),
            ["project-1", "project-12", "project-56"],
        )
    })
}

#[test]
fn array_with_non_existing() {
    with_test_workspace(setup(), |root| {
        let result = run(
            &root,
            RunOptions::new("dummy").with_selector_source(SelectorSource::Provided(
                ProjectSelector::array(["project-1", "project-12", "project-56", "does-not-exist"]),
            )),
            Default::default(),
        );

        assert!(result.is_err())
    })
}

#[test]
fn default() {
    with_test_workspace(setup(), |root| {
        run_and_verify_selected_projects(&root, None, ["project-15", "project-25", "project-35"])
    })
}

#[test]
fn simple_include() {
    with_test_workspace(setup(), |root| {
        run_and_verify_selected_projects(
            &root,
            Some(SelectorSource::Provided(ProjectSelector::include_exclude(
                ["^project-(1|6|10)$"],
                [],
            ))),
            ["project-1", "project-6", "project-10"],
        )
    })
}

#[test]
fn include_exclude() {
    with_test_workspace(setup(), |root| {
        run_and_verify_selected_projects(
            &root,
            Some(SelectorSource::Provided(ProjectSelector::include_exclude(
                ["^project-1.*"],
                ["project-12", "project-13", "project-14", "project-15"],
            ))),
            [
                "project-1",
                "project-10",
                "project-11",
                "project-16",
                "project-17",
                "project-18",
                "project-19",
            ],
        )
    })
}

fn run_and_verify_selected_projects<const N: usize>(
    root: &Path,
    source: Option<SelectorSource>,
    expected: [&str; N],
) {
    let mut options = RunOptions::new("dummy");

    if let Some(source) = source {
        options = options.with_selector_source(source);
    }

    let results = run(root, options, Default::default());

    let doubles = expected
        .into_iter()
        .map(|name| format!("{name}:dummy"))
        .collect::<Vec<_>>();

    Executions::from_run_result(results).assert_targets(
        doubles
            .iter()
            .map(|double| (double.as_str(), ExpectedExecution::success())),
    );
}
