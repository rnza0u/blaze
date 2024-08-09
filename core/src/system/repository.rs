use anyhow::Context;
use blaze_common::error::Result;
use std::io::{Seek, Write};
use std::{fs::OpenOptions, path::Path};

const GITIGNORE_FILE: &str = ".gitignore";

pub fn open_or_init_repository(root: &Path) -> Result<git2::Repository> {
    Ok(match git2::Repository::open(root) {
        Ok(repository) => repository,
        Err(err) if err.code() == git2::ErrorCode::NotFound => git2::Repository::init(root)?,
        Err(err) => return Err(err.into()),
    })
}

pub fn add_to_gitignore<S>(root: &Path, rules: &[S]) -> Result<()>
where
    S: AsRef<str>,
{
    let path = root.join(GITIGNORE_FILE);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("error while opening {}", path.display()))?;

    file.seek(std::io::SeekFrom::End(0))?;
    let pos = file.stream_position()?;
    if pos > 0 {
        file.write_all(b"\n\n")?;
    }

    let to_write = rules.iter().map(S::as_ref).collect::<Vec<_>>().join("\n");

    writeln!(file, "{to_write}")
        .with_context(|| format!("could not write rules to {}", path.display()))
}
