use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::bail;
use blaze_common::error::Result;
use wax::{FileIterator, LinkBehavior};

/// Main wrapper function for reading file paths from the provided root and glob expressions.
pub fn glob<P: AsRef<Path>, S: AsRef<str>, I: IntoIterator<Item = S>>(
    root: P,
    include: S,
    exclude: I,
) -> Result<HashSet<PathBuf>> {
    let exclude_vec = exclude.into_iter().collect::<Vec<_>>();

    let exclude_patterns = exclude_vec
        .iter()
        .map(|s| s.as_ref())
        .collect::<HashSet<_>>();

    let glob = wax::Glob::new(include.as_ref())?;

    if glob.has_semantic_literals() || glob.has_root() {
        bail!("glob match patterns must not be absolute or have semantic literals such as ../ or .")
    }

    let (invariant_prefix, glob) = glob.partition();

    let root = root.as_ref().join(invariant_prefix);

    if !root.try_exists()? {
        return Ok(HashSet::default());
    }

    let walk = glob.walk_with_behavior(root, LinkBehavior::ReadTarget);

    fn collect<I: FileIterator>(it: I) -> Result<HashSet<PathBuf>> {
        it.filter(|entry| !entry.as_ref().is_ok_and(|e| e.file_type().is_dir()))
            .map(|entry| Ok(entry?.path().to_owned()))
            .collect()
    }

    if exclude_patterns.is_empty() {
        collect(walk)
    } else {
        collect(walk.not(exclude_patterns)?)
    }
}
