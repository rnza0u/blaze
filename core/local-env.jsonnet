// environment variables for local Blaze builds
function(target) {
  BLAZE_NODE_BRIDGE_BUNDLE_PATH: target.nodeBridge.outputPath + '/' + target.nodeBridge.bundle,
  BLAZE_JSON_SCHEMAS_LOCATION: '{{ root }}/{{ workspace.projects.schemas.path }}/schemas',
}