local Target(rustTriple=null, release = false, ext='') = {
  local targetDir = if rustTriple == null then 'target' else 'target-' + rustTriple,
  targetDir: targetDir,
  rustTriple: rustTriple,
  release: release,
  cli: {
    outputPath: '{{ root }}/{{ workspace.projects.cli.path }}/' + targetDir
      + (if rustTriple != null then '/' + rustTriple else '')
      + (if release then '/release' else '/debug'),
    filename: 'blaze' + ext
  },
  rustBridge: {
    outputPath: '{{ root }}/{{ workspace.projects.rust-bridge.path }}/' + targetDir
      + (if rustTriple != null then '/' + rustTriple else '')
      + (if release then '/release' else '/debug'), 
    filename: 'blaze-rust-bridge' + ext
  },
  nodeBridge: {
    outputPath: '{{ root }}/{{ workspace.projects.node-bridge.path }}/dist',
    bundle: 'main.js',
  }
};

{
  'dev': Target(),
  'release': Target(null, true),
  'x86_64-linux-gnu': Target('x86_64-unknown-linux-gnu', true),
  'x86_64-linux-musl': Target('x86_64-unknown-linux-musl', true),
  'x86_64-windows': Target('x86_64-pc-windows-gnu', true, '.exe'),
  'aarch64-osx': Target('aarch64-apple-darwin', true),
  'x86_64-osx': Target('x86_64-apple-darwin', true),
}
