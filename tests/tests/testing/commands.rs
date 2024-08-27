#[cfg(windows)]
const SHELL: &str = "powershell";

#[cfg(not(windows))]
const SHELL: &str = "sh";

#[cfg(windows)]
fn get_shell_args(cmd: &str) -> Vec<String> {
    vec!["-C".to_string(), cmd.to_owned()]
}

#[cfg(not(windows))]
fn get_shell_args(cmd: &str) -> Vec<String> {
    vec!["-c".to_string(), cmd.to_owned()]
}

pub fn cmd<C: AsRef<str>>(command: C) {
    println!("+ {}", command.as_ref());

    if !std::process::Command::new(SHELL)
        .args(get_shell_args(command.as_ref()))
        .spawn()
        .unwrap_or_else(|_| panic!("process creation error ({})", command.as_ref()))
        .wait()
        .unwrap_or_else(|_| panic!("could not wait for process ({})", command.as_ref()))
        .success()
    {
        panic!("command exited with failure ({})", command.as_ref());
    }
}
