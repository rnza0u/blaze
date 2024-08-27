use blaze_core::{ExecutedGraph, ExecutionDetails, RunResult};

pub struct Executions {
    result: RunResult,
}

#[derive(Debug)]
pub enum ExpectedExecutionState {
    Success { cached: bool },
    Failure,
}

#[derive(Debug)]
pub struct ExpectedExecution {
    result: Option<ExpectedExecutionState>,
}

impl ExpectedExecution {
    pub fn success() -> Self {
        Self {
            result: Some(ExpectedExecutionState::Success { cached: false }),
        }
    }

    pub fn cached() -> Self {
        Self {
            result: Some(ExpectedExecutionState::Success { cached: true }),
        }
    }

    pub fn failure() -> Self {
        Self {
            result: Some(ExpectedExecutionState::Failure),
        }
    }

    pub fn not_executed() -> Self {
        Self { result: None }
    }
}

impl Executions {
    pub fn from_run_result(result: RunResult) -> Self {
        Executions { result }
    }

    pub fn assert_global_failure(&self) {
        assert!(self.result.is_err())
    }

    pub fn assert_targets<'a, T: IntoIterator<Item = (&'a str, ExpectedExecution)>>(
        &self,
        targets: T,
    ) {
        let graph = self.result.as_ref().expect("run error");

        Self::check_nodes(&targets.into_iter().collect::<Vec<_>>()[..], graph);
    }

    fn check_nodes(
        expected_targets: &[(&str, ExpectedExecution)],
        execution_graph: &ExecutedGraph<ExecutionDetails>,
    ) {
        assert_eq!(
            expected_targets.len(),
            execution_graph.execution().len(),
            "expected target results must have the same length as the actual target results"
        );

        for (expected_double, expected_target) in expected_targets {
            let maybe_actual_execution = execution_graph
                .execution()
                .get(&expected_double.to_string());

            assert!(
                maybe_actual_execution.is_some(),
                "could not find any actual executed target with name {expected_double}"
            );

            let actual_execution = maybe_actual_execution.unwrap();

            assert_eq!(
                *expected_double,
                actual_execution.execution.get_double().as_str(),
                "target execution double is invalid"
            );

            match expected_target.result.as_ref() {
                None => {
                    assert!(
                        actual_execution.result.is_none(),
                        "{expected_double} is expected to be not executed"
                    );
                }
                Some(expected_execution) => {
                    assert!(actual_execution.result.is_some());

                    let actual_result = actual_execution.result.as_ref().unwrap();

                    match expected_execution {
                        ExpectedExecutionState::Success { cached } => {
                            assert!(
                                actual_result.is_ok(),
                                "{} is expected to be executed successfully\n\n{:?}",
                                expected_double,
                                actual_result.as_ref().unwrap_err()
                            );
                            let unwrapped_result = actual_result.as_ref().unwrap();
                            assert_eq!(
                                *cached,
                                matches!(unwrapped_result, ExecutionDetails::Cached),
                                "{expected_double} must have cache state: {cached}"
                            );
                        }
                        ExpectedExecutionState::Failure => {
                            let is_failure = actual_result.is_err();
                            if !is_failure {
                                eprintln!(
                                    "target execution error: {:?}",
                                    actual_result.as_ref().err().unwrap()
                                );
                            }
                            assert!(is_failure, "{expected_double} is expected to be fail");
                        }
                    }
                }
            }
        }
    }
}
