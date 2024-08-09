use std::{collections::HashMap, path::Path};

use anyhow::bail;
use blaze_common::{error::Result, executor::ExecutorKind};

use crate::executors::{node::is_node_executor, rust::is_rust_executor};

pub fn infer_local_executor_type(root: &Path) -> Result<ExecutorKind> {
    let cases = HashMap::from([
        (
            ExecutorKind::Node,
            is_node_executor as fn(&Path) -> Result<bool>,
        ),
        (ExecutorKind::Rust, is_rust_executor),
    ]);

    for (kind, supports) in cases {
        if supports(root)? {
            return Ok(kind);
        }
    }

    bail!("could not infer executor type from path {}", root.display())
}
