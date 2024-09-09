fn main() {
    // check if node is present or not
    println!("cargo::rustc-check-cfg=cfg(node)");

    match std::process::Command::new("node")
        .args(["--version"])
        .spawn()
    {
        Ok(mut child) => {
            child
                .wait()
                .expect("could not wait for node version process");
            println!("cargo::rustc-cfg=node");
        }
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            panic!("{err}");
        }
        _ => {}
    };

    println!(
        "cargo::rustc-env=PROJECT_ROOT={}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap()
    );
}
