#![allow(dead_code)]

use blaze_core::time::now;
use filetime::{set_file_atime, set_file_mtime, FileTime};
use rand::{distributions::Alphanumeric, Rng};
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, UnwindSafe},
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Default)]
pub struct TestWorkspaceConfiguration {
    pub workspace: serde_json::Value,
    pub projects: BTreeMap<String, serde_json::Value>,
    pub files: BTreeMap<String, String>,
}

impl TestWorkspaceConfiguration {
    pub fn new<
        S: AsRef<str>,
        P: IntoIterator<Item = (S, serde_json::Value)>,
        F: IntoIterator<Item = (S, S)>,
    >(
        workspace: serde_json::Value,
        projects: P,
        files: F,
    ) -> Self {
        Self {
            workspace,
            projects: projects
                .into_iter()
                .map(|(name, config)| (name.as_ref().to_owned(), config))
                .collect(),
            files: files
                .into_iter()
                .map(|(from, to)| (from.as_ref().to_owned(), to.as_ref().to_owned()))
                .collect(),
        }
    }
}

pub fn with_test_folder<F: FnOnce(PathBuf) + UnwindSafe>(
    create_if_not_exist: bool,
    test_routine: F,
) {
    let name = random_workspace_name();

    let tmpdir = std::env::temp_dir().join(name);

    if create_if_not_exist {
        std::fs::create_dir(&tmpdir).expect("could not create directory");
    }

    let result = catch_unwind(|| test_routine(tmpdir.to_owned()));

    let keep_files = result.is_err()
        && option_env!("KEEP_FILES").is_some_and(|value| value == (true).to_string());

    if !keep_files {
        std::fs::remove_dir_all(tmpdir).expect("could not cleanup test dir");
    } else if result.is_err() {
        println!("workspace files: {}", tmpdir.display());
    }

    result.expect("test failure")
}

pub const FIXTURES_DIR: &str = "tests/fixtures";

pub fn with_test_workspace<F: FnOnce(&Path) + UnwindSafe>(
    config: TestWorkspaceConfiguration,
    test_routine: F,
) {
    with_test_folder(true, move |root| {
        let set_timestamps = |time: SystemTime, path: &Path| {
            let file_time = FileTime::from_system_time(time);
            set_file_atime(path, file_time).expect("could not set access time");
            set_file_mtime(path, file_time).expect("could not set modified time");
        };

        let now = now();
        let workspace_json =
            serde_json::to_string(&config.workspace).expect("could not serialize workspace");
        let workspace_json_path = root.join("workspace.json");
        std::fs::write(&workspace_json_path, workspace_json)
            .expect("could not write workspace file");
        set_timestamps(now, &workspace_json_path);

        for (name, project) in &config.projects {
            let project_json = serde_json::to_string(project).expect("could not serialize project");

            std::fs::create_dir_all(root.join(name)).expect("could not create project dir");

            let project_json_path = root.join(name).join("project.json");

            std::fs::write(&project_json_path, project_json).expect("could not write project file");
            set_timestamps(now, &project_json_path);
        }

        for (from, to) in &config.files {
            let paths = glob::glob(&format!("{FIXTURES_DIR}/{from}"))
                .expect("files glob pattern error")
                .map(|result| result.expect("glob lookup error"))
                .collect::<Vec<PathBuf>>();

            let shortest_path = paths.iter().fold(usize::MAX, |shortest, path| {
                let length = path.components().count() - 1;
                usize::min(shortest, length)
            });

            std::fs::create_dir_all(root.join(to)).expect("could not create directory");

            for path in paths {
                let dest = root
                    .join(to)
                    .join(path.components().skip(shortest_path).collect::<PathBuf>());

                if std::fs::metadata(&path)
                    .map(|m| m.is_dir())
                    .expect("could not read metadata")
                {
                    std::fs::create_dir_all(&dest).expect("could not create directory");
                } else {
                    std::fs::copy(&path, &dest).expect("could not copy file");
                }
                set_timestamps(now, &dest);
            }
        }

        catch_unwind(|| test_routine(&root)).expect("workspace test failed");
    });
}

fn random_workspace_name() -> String {
    format!(
        "blaze_test_workspace_{}",
        &rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .map(char::from)
            .collect::<String>()
    )
}
