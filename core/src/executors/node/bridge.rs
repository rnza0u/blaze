use std::path::Path;

use blaze_common::{error::Result, value::Value};
use serde::Serialize;

use crate::{
    executors::{
        bridge::{bridge_executor, BridgeProcessParams},
        ExecutorContext,
    },
    system::env::Env,
};

#[derive(Serialize)]
pub struct NodeBridgeMetadata<'a> {
    module: &'a Path,
}

const NODE_LOCATION: &str = "node";
const OVERRIDE_NODE_LOCATION_ENVIRONMENT_VARIABLE: &str = "BLAZE_NODE_LOCATION";

pub struct NodeBridgeParameters<'a> {
    pub module: &'a Path,
    pub context: ExecutorContext<'a>,
    pub options: &'a Value,
}

pub fn execute_node_bridge(parameters: NodeBridgeParameters) -> Result<()> {
    bridge_executor(
        (parameters.context, parameters.options),
        BridgeProcessParams {
            program: &Env::get_as_str(OVERRIDE_NODE_LOCATION_ENVIRONMENT_VARIABLE)?
                .unwrap_or_else(|| NODE_LOCATION.to_owned()),
            arguments: [
                "--unhandled-rejections=strict",
                "--input-type=module",
                "--title=blaze-node-bridge",
                "-",
                "--",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .as_slice(),
            input: Some(include_bytes!(env!("BLAZE_NODE_BRIDGE_BUNDLE_PATH"))),
        },
        NodeBridgeMetadata {
            module: parameters.module,
        },
    )
}
