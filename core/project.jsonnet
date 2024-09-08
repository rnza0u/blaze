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
        format: 'Git',
        pull: true
      },
      options: {
        releaseVersion: blaze.vars.publish.version,
        linkedDependencies: {
          runtime: [dep.crate for dep in cargoDependencies],
          build: [dep.crate for dep in cargoBuildDependencies]
        }
      },
      dependencies: [
        {
          projects: [dep.project for dep in cargoDependencies + cargoBuildDependencies] + npmDependencies,
          target: 'publish'
        }
      ]
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
