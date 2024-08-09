use anyhow::{anyhow, bail, Context};
use regex::Regex;
use std::{
    collections::{BTreeSet, HashMap},
    str::FromStr,
};

use blaze_common::{
    error::Result,
    selector::ProjectSelector,
    workspace::{ProjectRef, Workspace},
};

pub struct SelectionContext<'a> {
    pub workspace: &'a Workspace,
}

#[derive(Debug, Clone)]
pub enum SelectorSource {
    Provided(ProjectSelector),
    Named(String),
}

#[derive(Debug, Clone, Default)]
pub struct Selection {
    source: Option<SelectorSource>,
}

impl Selection {
    pub fn from_source(source: SelectorSource) -> Self {
        Self {
            source: Some(source),
        }
    }

    pub fn select<'s>(
        &self,
        ctx: SelectionContext<'s>,
    ) -> Result<HashMap<&'s String, &'s ProjectRef>> {
        let selector = match &self.source {
            Some(SelectorSource::Provided(selector)) => selector,
            Some(SelectorSource::Named(name)) =>  ctx
                .workspace
                .settings()
                .selectors()
                .get(name)
                .ok_or_else(|| anyhow!("named project selector {name} does not exist."))?,
            None => ctx
                .workspace
                .settings()
                .default_selector()
                .as_ref()
                .ok_or_else(|| anyhow!("no selector was provided and no default selector exists at the workspace level."))?
        };

        match selector {
            ProjectSelector::All => Ok(self.select_all_project_refs(ctx)),
            ProjectSelector::Array(names) => self.select_named_project_refs(ctx, names),
            ProjectSelector::IncludeExclude { include, exclude } => {
                self.select_include_exclude_project_refs(ctx, include, exclude)
            }
            ProjectSelector::Tagged(tags) => Ok(self.select_tagged_project_refs(ctx, tags)),
        }
    }

    fn select_all_project_refs<'s>(
        &self,
        ctx: SelectionContext<'s>,
    ) -> HashMap<&'s String, &'s ProjectRef> {
        ctx.workspace.projects().iter().collect()
    }

    fn select_include_exclude_project_refs<'s>(
        &self,
        ctx: SelectionContext<'s>,
        include: &BTreeSet<String>,
        exclude: &BTreeSet<String>,
    ) -> Result<HashMap<&'s String, &'s ProjectRef>> {
        let build_regexes = |patterns: &BTreeSet<String>| {
            patterns.iter()
            .map(|pattern| Regex::from_str(pattern).with_context(|| format!("invalid exclude expression pattern {pattern} was supplied, it must be a valid regular expression.")))
            .collect::<Result<Vec<_>>>()
        };
        let include_patterns = build_regexes(include)?;
        let exclude_patterns = build_regexes(exclude)?;

        Ok(self.select_predicated_project_refs(ctx, |(name, _)| {
            include_patterns
                .iter()
                .any(|pattern| pattern.is_match(name))
                && !exclude_patterns
                    .iter()
                    .any(|pattern| pattern.is_match(name))
        }))
    }

    fn select_named_project_refs<'s>(
        &self,
        ctx: SelectionContext<'s>,
        project_names: &BTreeSet<String>,
    ) -> Result<HashMap<&'s String, &'s ProjectRef>> {
        let project_refs = self.select_predicated_project_refs(ctx, |(name, _)| {
            project_names
                .iter()
                .map(String::as_str)
                .any(|selected| name == selected)
        });

        let not_found = project_names
            .iter()
            .filter(|name| !project_refs.contains_key(name))
            .collect::<Vec<&String>>();

        if !not_found.is_empty() {
            bail!("some projects were not found: {not_found:?}");
        }

        Ok(project_refs)
    }

    fn select_tagged_project_refs<'s>(
        &self,
        ctx: SelectionContext<'s>,
        tags: &BTreeSet<String>,
    ) -> HashMap<&'s String, &'s ProjectRef> {
        self.select_predicated_project_refs(ctx, |(_, project_ref)| {
            !project_ref.tags().is_disjoint(tags)
        })
    }

    fn select_predicated_project_refs<'s, P: Fn((&str, &ProjectRef)) -> bool>(
        &self,
        ctx: SelectionContext<'s>,
        predicate: P,
    ) -> HashMap<&'s String, &'s ProjectRef> {
        ctx.workspace
            .projects()
            .iter()
            .filter(|(name, project_ref)| predicate((name.as_str(), *project_ref)))
            .collect()
    }
}
