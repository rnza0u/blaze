#[macro_use]
mod testing;

use blaze_core::{common::selector::ProjectSelector, run, RunOptions};
use serde_json::json;
use testing::{with_test_workspace, TestWorkspaceConfiguration};

#[cfg(windows)]
#[test]
fn dummy_powershell_script() {
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "dummy-powershell-script-test",
                "projects": {
                    "test-project": "test-project"
                }
            }),
            [(
                "test-project",
                json!({
                    "targets": {
                        "call-powershell-script": {
                            "executor": "std:exec",
                            "options": {
                                "program": "{{ project.root }}/dummy.ps1",
                                "shell": {
                                    "type": "powershell",
                                    "program": "powershell.exe"
                                },
                                "environment": {
                                    "TEST_VAR": "test message"
                                }
                            }
                        }
                    }
                }),
            )],
            [("scripts/dummy.ps1", "test-project")],
        ),
        |root| {
            let results = run(
                root,
                RunOptions {
                    target: "call-powershell-script".into(),
                    selector: Some(ProjectSelectorOption::Provided(ProjectSelector::named([
                        "test-project",
                    ]))),
                    ..Default::default()
                },
                Default::default(),
            );

            Executions::from_run_result(results).assert_targets([(
                "test-project:call-powershell-script",
                ExpectedExecution { result: Some(true) },
            )]);
        },
    )
}

#[cfg(not(windows))]
#[test]
fn dummy_shell_script() {
    use blaze_core::SelectorSource;
    use testing::cmd;

    use crate::testing::{Executions, ExpectedExecution};
    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "dummy-shell-script-test",
                "projects": {
                    "test-project": "test-project"
                }
            }),
            [(
                "test-project",
                json!({
                    "targets": {
                        "call-shell-script": {
                            "executor": "std:exec",
                            "options": {
                                "program": "{{ project.root }}/dummy.sh",
                                "shell": {
                                    "type": "sh",
                                    "program": "/bin/sh"
                                },
                                "environment": {
                                    "TEST_VAR": "test message"
                                }
                            }
                        }
                    }
                }),
            )],
            [("scripts/dummy.sh", "test-project")],
        ),
        |root| {
            cmd(format!(
                "chmod +x {}",
                root.join("test-project/dummy.sh").display()
            ));

            let results = run(
                root,
                RunOptions::new("call-shell-script").with_selector_source(
                    SelectorSource::Provided(ProjectSelector::array(["test-project"])),
                ),
                Default::default(),
            );

            Executions::from_run_result(results).assert_targets([(
                "test-project:call-shell-script",
                ExpectedExecution::success(),
            )]);
        },
    )
}
