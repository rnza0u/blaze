mod testing;

use blaze_core::{
    common::{parallelism::Parallelism, selector::ProjectSelector},
    run, RunOptions, SelectorSource,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use testing::{Executions, ExpectedExecution};

use crate::testing::{cmd, with_test_workspace, TestWorkspaceConfiguration};

fn setup_client(id: usize, server_path: &str) -> (String, serde_json::Value) {
    (
        format!("client-{id}"),
        json!({
            "targets": {
                "parallel": {
                    "executor": "std:commands",
                    "options": {
                        "commands": [
                            {
                                "program": "cat",
                                "arguments": [server_path, ">", format!("parallel-test-output-{id}")],
                            },
                        ],
                        "shell": "sh"
                    }
                }
            }
        }),
    )
}

fn setup_server(id: usize, server_path: &str) -> (String, serde_json::Value) {
    (
        format!("server-{id}"),
        json!({
            "targets": {
                "parallel": {
                    "executor": "std:commands",
                    "options": {
                        "commands": [
                            {
                                "program": "echo",
                                "arguments": ["-n", "Hello world!", ">", server_path],
                            },
                        ],
                        "shell": "sh"
                    }
                }
            }
        }),
    )
}

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn run_parallel_targets_infinite() {
    let fifo_path = std::env::temp_dir().join(format!(
        "blaze_test_fifo_{}",
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect::<String>()
    ));
    let fifo_path_string = fifo_path.to_str().unwrap().to_owned();

    cmd(format!("mkfifo {fifo_path_string}_1"));
    cmd(format!("mkfifo {fifo_path_string}_2"));
    cmd(format!("mkfifo {fifo_path_string}_3"));

    with_test_workspace(
        TestWorkspaceConfiguration::new(
            json!({
                "name": "test",
                "projects": {
                    "server-1": "server-1",
                    "server-2": "server-2",
                    "server-3": "server-3",
                    "client-1": "client-1",
                    "client-2": "client-2",
                    "client-3": "client-3"
                }
            }),
            [
                setup_client(1, &format!("{fifo_path_string}_1")),
                setup_client(2, &format!("{fifo_path_string}_2")),
                setup_client(3, &format!("{fifo_path_string}_3")),
                setup_server(1, &format!("{fifo_path_string}_1")),
                setup_server(2, &format!("{fifo_path_string}_2")),
                setup_server(3, &format!("{fifo_path_string}_3")),
            ],
            [],
        ),
        |root| {
            let results = run(
                root,
                RunOptions::new("parallel")
                    .with_selector_source(SelectorSource::Provided(ProjectSelector::all()))
                    .with_parallelism(Parallelism::Infinite),
                Default::default(),
            );

            let executions = Executions::from_run_result(results);

            let target_names = (0..6)
                .into_iter()
                .map(|i| {
                    format!(
                        "{}-{}:parallel",
                        if i < 3 { "server" } else { "client" },
                        (i % 3) + 1
                    )
                })
                .collect::<Vec<_>>();

            executions.assert_targets(
                (0..6)
                    .into_iter()
                    .map(|i| (target_names[i].as_str(), ExpectedExecution::success())),
            );
        },
    );

    cmd(format!("rm {fifo_path_string}_1"));
    cmd(format!("rm {fifo_path_string}_2"));
    cmd(format!("rm {fifo_path_string}_3"));
}
