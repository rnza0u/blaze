use std::{
    borrow::Cow,
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

use blaze_common::{
    error::Result,
    shell::{Shell, ShellKind},
    util::path_to_string,
};

#[derive(Debug)]
pub struct ShellFormatter<'a> {
    program: Cow<'a, Path>,
    kind: ShellKind,
}

impl Display for ShellFormatter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.program.display(), self.kind)
    }
}

impl Default for ShellFormatter<'_> {
    fn default() -> Self {
        let kind = default_shell_kind();
        Self {
            program: Cow::Owned(default_program_path_for_kind(kind)),
            kind,
        }
    }
}

impl<'a> ShellFormatter<'a> {
    pub fn from_shell(shell: &'a Shell) -> Self {
        Self {
            kind: shell
                .kind()
                .or_else(|| shell_kind_from_path(shell.program()))
                .unwrap_or_else(default_shell_kind),
            program: Cow::Borrowed(shell.program()),
        }
    }

    pub fn format_command(&self, command: &str) -> Result<(PathBuf, Vec<String>)> {
        let mut shell_args = match self.kind {
            ShellKind::Posix => ["-c"],
            ShellKind::Cmd => ["/C"],
            ShellKind::Powershell => ["-Command"],
        }
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

        shell_args.push(command.to_owned());

        Ok(((*self.program).to_owned(), shell_args))
    }

    pub fn format_program_and_args<P, S, A>(
        &self,
        program: P,
        args: A,
    ) -> Result<(PathBuf, Vec<String>)>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
        A: IntoIterator<Item = S>,
    {
        let mut command_with_args = path_to_string(program)?;
        for arg in args.into_iter() {
            command_with_args.push(' ');
            command_with_args.push_str(arg.as_ref());
        }

        self.format_command(&command_with_args)
    }

    pub fn format_script<P: AsRef<Path>, S: AsRef<str>, A: IntoIterator<Item = S>>(
        &self,
        script_path: P,
        script_arguments: A,
    ) -> Result<(PathBuf, Vec<String>)> {
        let script_path_string = path_to_string(script_path)?;

        let arguments = script_arguments
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>();

        let arguments = match self.kind {
            ShellKind::Posix => [vec![script_path_string], arguments].concat(),
            ShellKind::Cmd => [vec!["/C".into(), script_path_string], arguments].concat(),
            ShellKind::Powershell => [vec!["-File".into(), script_path_string], arguments].concat(),
        };

        Ok(((*self.program).to_owned(), arguments))
    }
}

fn shell_kind_from_path(path: &Path) -> Option<ShellKind> {
    path.file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| match name {
            "bash" | "sh" | "zsh" | "ksh" | "dash" | "tcsh" | "csh" => Some(ShellKind::Posix),
            "cmd.exe" | "cmd" => Some(ShellKind::Cmd),
            "powershell.exe" | "powershell" => Some(ShellKind::Powershell),
            _ => None,
        })
}

fn default_program_path_for_kind(kind: ShellKind) -> PathBuf {
    PathBuf::from(match kind {
        ShellKind::Posix => "/bin/sh",
        ShellKind::Cmd => "C:\\Windows\\System32\\cmd.exe",
        ShellKind::Powershell => "C:\\Windows\\WindowsPowerShell\\v1.0\\powershell.exe",
    })
}

fn default_shell_kind() -> ShellKind {
    #[cfg(windows)]
    return ShellKind::Cmd;
    #[cfg(not(windows))]
    ShellKind::Posix
}
