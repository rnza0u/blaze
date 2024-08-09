use blaze_common::parallelism::Parallelism;

pub fn parallelism_input_hint() -> String {
    format!(
        "You can pass a number to indicate the maximum number of jobs running at the same time. \
\"{}\" will set the maximum number of executions to the number of logical cores on your system.\
\"{}\" will run every jobs in parallel without any max limit.\
\"{}\" can be used to totally disable parallelism and run jobs sequentially.",
        Parallelism::All,
        Parallelism::Infinite,
        Parallelism::None
    )
}

pub fn include_projects_input_hint() -> &'static str {
    "You can pass this option multiple times if you want to use more than one inclusion pattern. Each pattern must be a valid regular expression. If you want to exclude some projects, you can use the --exclude option."
}

pub fn exclude_projects_input_hint() -> &'static str {
    "You can pass this option multiple times if you want to use more than one exclusion pattern. Each pattern must a valid regular expression."
}
