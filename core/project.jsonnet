local blaze = std.extVar('blaze');
local targets = import '../targets.jsonnet';
local cargo = (import 'cargo.libsonnet')(blaze.vars.blaze.rust.channel, ['-Z', 'bindeps']);
local executors = import 'executors.libsonnet';
local LocalEnv = import './local-env.jsonnet';

local cargoDependencies = [
  { project: 'blaze-common', crate: 'blaze-common' },
];

local cargoBuildDependencies = [
  { project: 'blaze-rust-bridge', crate: 'blaze-rust-bridge' },
];

local npmDependencies = [
  'blaze-node-bridge',
  'blaze-schemas',
];

local cargoTargets = cargo.all({
  workspaceDependencies: [dep.project for dep in cargoDependencies + cargoBuildDependencies],
  environment: LocalEnv(targets.dev)
});

{
  targets: cargoTargets + {
    source: cargoTargets.source + {
      dependencies: cargoTargets.source.dependencies + [
        {
          projects: npmDependencies,
          target: 'build',
        }
      ]
    },
    publish: {
      executor: executors.cargoPublish(),
      options: {
        dryRun: blaze.vars.blaze.publish.dryRun,
        channel: blaze.vars.blaze.rust.channel
      },
      dependencies: [
        'check-version',
        {
          projects: [dep for dep in [cargoDep.project for cargoDep in cargoDependencies + cargoBuildDependencies] + npmDependencies],
          target: 'publish'
        }
      ]
    },
    'check-version': {
      executor: executors.cargoVersionCheck(),
      options: {
        version: blaze.vars.blaze.publish.version,
        workspaceDependencies: [dep.project for dep in cargoDependencies],
        workspaceBuildDependencies: [dep.project for dep in cargoBuildDependencies]
      }
    },
    ci: {
      dependencies: ['lint', 'check']
    }
  },
}
