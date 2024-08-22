use std::collections::BTreeSet;

use blaze_common::{
    cache::FileChangesMatcher,
    error::Result,
    logger::Logger,
    value::{to_value, Value},
};

use crate::system::file_changes::{FileChangeType, MatchedFiles, MatchedFilesState};
use serde::Deserialize;

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

const INPUT_FILE_CHANGES_STATE_KEY: &str = "input-file-changes";
const OUTPUT_FILE_CHANGES_STATE_KEY: &str = "output-file-changes";

pub struct OutputFileChangesCheck<'a> {
    logger: &'a Logger,
    matchers: &'a BTreeSet<FileChangesMatcher>,
}

impl<'a> OutputFileChangesCheck<'a> {
    pub fn new(matchers: &'a BTreeSet<FileChangesMatcher>, logger: &'a Logger) -> Self {
        Self { matchers, logger }
    }
}

impl CacheInvalidationCheck for OutputFileChangesCheck<'_> {
    fn state(&self, execution: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            OUTPUT_FILE_CHANGES_STATE_KEY,
            to_value(MatchedFilesState::from_files(MatchedFiles::try_new(
                execution.get_project().root(),
                self.matchers,
            )?)?)?,
        )])))
    }

    fn validate(
        &mut self,
        execution: &TargetExecution,
        cached: &ExecutionCacheState,
    ) -> Result<bool> {
        let maybe_last_state = cached
            .metadata
            .at(OUTPUT_FILE_CHANGES_STATE_KEY)
            .map(MatchedFilesState::deserialize)
            .transpose()?;

        let last_state = match maybe_last_state {
            Some(state) => state,
            None => return Ok(false),
        };

        let current_matched_files =
            MatchedFiles::try_new(execution.get_project().root(), self.matchers)?;

        let merge_result = last_state.merge(current_matched_files)?;

        for change in &merge_result.changes {
            self.logger.debug(match change.change_type {
                FileChangeType::Created => format!("{} was created", change.path.display()),
                FileChangeType::Modified => format!("{} was modified", change.path.display()),
                FileChangeType::Removed => format!("{} was removed", change.path.display()),
            });
        }

        Ok(merge_result.changes.is_empty())
    }
}

pub struct InputFileChangesCheck<'a> {
    logger: &'a Logger,
    matchers: &'a BTreeSet<FileChangesMatcher>,
    computed_state: Option<MatchedFilesState>,
}

impl<'a> InputFileChangesCheck<'a> {
    pub fn new(matchers: &'a BTreeSet<FileChangesMatcher>, logger: &'a Logger) -> Self {
        Self {
            matchers,
            logger,
            computed_state: None,
        }
    }
}

impl CacheInvalidationCheck for InputFileChangesCheck<'_> {
    fn state(&self, execution: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            INPUT_FILE_CHANGES_STATE_KEY,
            if let Some(files) = self.computed_state.as_ref() {
                to_value(files)?
            } else {
                to_value(MatchedFilesState::from_files(MatchedFiles::try_new(
                    execution.get_project().root(),
                    self.matchers,
                )?)?)?
            },
        )])))
    }

    fn validate(
        &mut self,
        execution: &TargetExecution,
        cached: &ExecutionCacheState,
    ) -> Result<bool> {
        let maybe_last_state = cached
            .metadata
            .at(INPUT_FILE_CHANGES_STATE_KEY)
            .map(MatchedFilesState::deserialize)
            .transpose()?;

        let last_state = match maybe_last_state {
            Some(state) => state,
            None => return Ok(false),
        };

        let current_matched_files =
            MatchedFiles::try_new(execution.get_project().root(), self.matchers)?;

        let merge_result = last_state.merge(current_matched_files)?;

        let _ = self.computed_state.insert(merge_result.files_state);

        for change in &merge_result.changes {
            self.logger.debug(match change.change_type {
                FileChangeType::Created => format!("{} was created", change.path.display()),
                FileChangeType::Modified => format!("{} was modified", change.path.display()),
                FileChangeType::Removed => format!("{} was removed", change.path.display()),
            });
        }

        Ok(merge_result.changes.is_empty())
    }
}
