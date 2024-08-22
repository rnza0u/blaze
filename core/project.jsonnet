local blaze = std.extVar('blaze');
local targets = import '../targets.jsonnet';
local LocalEnv = import './local-env.jsonnet';

local cargoDependencies = [
  { crate: 'blaze-common', project: 'common' }
];

local cargoBuildDependencies = [
  { crate: 'blaze-rust-bridge', project: 'rust-bridge' }
];

local npmDependencies = [
  'node-bridge',
  'schemas'
];

{
  targets: {
    lint: {
      executor: 'std:commands',
      options: {
        commands: (if blaze.vars.lint.fix then [
            {
                program: 'cargo',
                arguments: ['fmt'],
                environment: LocalEnv(targets.dev)
            }
        ] else []) + [
            {
                program: 'cargo',
                arguments: ['check'],
                environment: LocalEnv(targets.dev)
            },
            {
                program: 'cargo',
                arguments: ['clippy'],
                environment: LocalEnv(targets.dev)
            }
        ]
      },
      dependencies: ['source']
    },
    source: {
      cache: {
        invalidateWhen: {
          inputChanges: [
            'src/**',
            'Cargo.toml',
            'Cargo.lock',
            'build.rs'
          ]
        }
      },
      dependencies: [
        {
          projects: [dep.project for dep in cargoDependencies + cargoBuildDependencies],
          target: 'source'
        },
        {
          projects: npmDependencies,
          target: 'build',
        }
      ]
    },
    publish: {
      executor: {
        url: 'https://github.com/rnza0u/blaze-executors.git',
        path: 'cargo-publish',
        format: 'Git'
      },
      options: {
        dryRun: blaze.vars.publish.dryRun
      },
      dependencies: [
        'check-version',
        {
          projects: [dep.project for dep in cargoDependencies + cargoBuildDependencies] + npmDependencies,
          target: 'publish'
        }
      ]
    },
    'check-version': {
      executor: {
        url: 'https://github.com/rnza0u/blaze-executors.git',
        path: 'cargo-version-check',
        format: 'Git'
      },
      options: {
        version: blaze.vars.publish.version,
        workspaceDependencies: [dep.crate for dep in cargoDependencies],
        workspaceBuildDependencies: [dep.crate for dep in cargoBuildDependencies]
      }
    },
    clean: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: ['clean']
          }
        ]
      }
    }
  },
}
