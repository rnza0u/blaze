use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn convert_env_path(env_path: &Path) -> PathBuf {
    if env_path.is_relative() {
        let core_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .expect("could not get cargo manifest dir");
        core_root
            .join(env_path)
            .canonicalize()
            .expect("could not canonicalize bridge bundle path")
    } else {
        env_path.to_owned()
    }
}

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("could not get package version");
    let out_dir = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .expect("could not get artifacts output directory");

    // Node bridge
    match std::env::var("BLAZE_NODE_BRIDGE_BUNDLE_PATH").map(PathBuf::from) {
        Ok(path) => {
            let final_path = convert_env_path(&path);
            if !final_path.is_file() {
                panic!("{} is not a file", final_path.display());
            }
            println!(
                "cargo::rustc-env=BLAZE_NODE_BRIDGE_BUNDLE_PATH={}",
                final_path.display()
            );
        }
        Err(std::env::VarError::NotPresent) => {
            let url = format!(
                "https://registry.npmjs.org/@blaze-repo/node-bridge/-/node-bridge-{version}.tgz",
            );
            let bundle_path = Path::new("package/dist/main.js");

            println!("fetching {url}");

            let response = reqwest::blocking::get(&url)
                .expect("could not send node bridge package metadata request");
            let status = response.status();

            if !status.is_success() {
                panic!("error response from {url} (status={status})");
            }

            let mut archive = tar::Archive::new(GzDecoder::new(response));
            let mut entries = archive
                .entries()
                .expect("could not get node bridge package entries");

            let node_bridge_location = out_dir.join("node-bridge/main.js");
            loop {
                let mut entry = entries
                    .next()
                    .expect("could not find node bridge bundle in package")
                    .expect("failed to read node bridge package entry");
                let path = entry
                    .path()
                    .expect("could not get node bridge package entry path");
                if &*path == bundle_path {
                    println!("found node bridge in bundle at {}", path.display());
                    let node_bridge_location_parent = node_bridge_location.parent().unwrap();
                    println!(
                        "creating node bridge artifact directory {}",
                        node_bridge_location_parent.display()
                    );
                    std::fs::create_dir_all(node_bridge_location_parent)
                        .expect("could not create node bridge output directory");
                    std::io::copy(
                        &mut entry,
                        &mut std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&node_bridge_location)
                            .expect("failed to open node bridge package entry extracted file"),
                    )
                    .expect("failed to extract entry from node bridge package");
                    break;
                }
            }
            println!(
                "cargo::rustc-env=BLAZE_NODE_BRIDGE_BUNDLE_PATH={}",
                node_bridge_location.display()
            );
        }
        Err(_) => panic!("could not read blaze node bundle path"),
    };

    // Rust bridge
    let rust_bridge_location = match std::env::var(
        "CARGO_BIN_FILE_BLAZE_RUST_BRIDGE_blaze-rust-bridge",
    )
    .map(PathBuf::from)
    {
        Ok(path) => {
            println!("Rust bridge is provided from bindeps => {}", path.display());
            path
        }
        // -Z bindeps only works locally, not recursively across crate dependencies.
        Err(std::env::VarError::NotPresent) => {
            let install_root = out_dir.join("bin-dependencies");
            std::fs::create_dir_all(&install_root)
                .expect("could not create bin dependencies output directory");
            let install_target_dir = out_dir.join("bin-dependencies-target");
            std::fs::create_dir_all(&install_target_dir)
                .expect("could not create bin dependencies target directory");

            let status = std::process::Command::new(
                std::env::var("CARGO").expect("could not get cargo binary path"),
            )
            .args(
                [
                    vec![
                        "install".to_owned(),
                        "--root".into(),
                        install_root.to_str().unwrap().into(),
                        "--force".into(),
                        "--no-track".into(),
                        "--target-dir".into(),
                        install_target_dir.to_str().unwrap().into(),
                        "-Z".into(),
                        "bindeps".into(),
                    ],
                    std::env::var("TARGET")
                        .map(|triple| vec!["--target".into(), triple])
                        .unwrap_or_default(),
                    vec![format!("blaze-rust-bridge@{version}")],
                ]
                .concat(),
            )
            .status()
            .expect("Rust bridge install process error");

            if !status.success() {
                panic!("error while installing Rust bridge ({status:?})");
            }

            #[cfg(windows)]
            let bin_path = "bin\\blaze-rust-bridge.exe";

            #[cfg(not(windows))]
            let bin_path = "bin/blaze-rust-bridge";

            install_root.join(bin_path)
        }
        Err(_) => panic!("could not read rust bridge bindeps path"),
    };

    let mut rust_bridge = std::fs::File::open(&rust_bridge_location)
        .expect("could not open Rust bridge executable file");
    let mut hasher = Sha256::new();
    std::io::copy(&mut rust_bridge, &mut hasher)
        .expect("checksum error for Rust bridge executable");
    let checksum = hasher
        .finalize()
        .into_iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("");

    println!(
        "cargo::rustc-env=BLAZE_RUST_BRIDGE_EXECUTABLE_PATH={}",
        rust_bridge_location.display()
    );
    println!("cargo::rustc-env=BLAZE_RUST_BRIDGE_CHECKSUM={checksum}");

    // JSON schemas
    let schemas_location = match std::env::var("BLAZE_JSON_SCHEMAS_LOCATION").map(PathBuf::from) {
        Ok(path) => {
            let final_path = convert_env_path(&path);
            if !final_path.is_dir() {
                panic!("{} is not a directory", final_path.display());
            }
            final_path
        }
        Err(std::env::VarError::NotPresent) => {
            let url = format!(
                "https://registry.npmjs.org/@blaze-repo/json-schemas/-/json-schemas-{version}.tgz",
            );
            let response =
                reqwest::blocking::get(&url).expect("failed to fetch JSON schemas from NPM");
            let status = response.status();

            if !status.is_success() {
                panic!("error response from {url} (status={status})");
            }

            let mut archive = tar::Archive::new(GzDecoder::new(response));

            let schemas_location = out_dir.join("json-schemas");
            std::fs::create_dir_all(&schemas_location).expect("failed to create schemas directory");

            for entry in archive.entries().expect("failed to get archive entries") {
                let mut entry = entry.expect("could not read JSON schemas package entry");
                let path = (*entry
                    .path()
                    .expect("could not get JSON schemas package entry path"))
                .to_owned();
                if path
                    .parent()
                    .is_some_and(|parent| parent == Path::new("package/schemas"))
                {
                    println!("extracting JSON schema {}", path.display());
                    let dst_path = schemas_location.join(path.file_name().unwrap());
                    std::io::copy(
                        &mut entry,
                        &mut std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&dst_path)
                            .expect("could not open JSON schema extracted file"),
                    )
                    .expect("could not extract JSON schema");
                }
            }

            schemas_location
        }
        Err(_) => panic!("could not read JSON schemas location path variable"),
    };

    println!(
        "cargo::rustc-env=BLAZE_JSON_SCHEMAS_LOCATION={}",
        schemas_location.display()
    );
}
