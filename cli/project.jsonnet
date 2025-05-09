local blaze = std.extVar('blaze');
local LocalEnv = import '../core/local-env.jsonnet';
local targets = import '../targets.jsonnet';

local workspaceDependencies = [
  { crate: 'blaze-common', project: 'common' },
  { crate: 'blaze-core', project: 'core' },
];

local cargoArgs = (if blaze.vars.ci then ['--locked'] else []);

local finalTargets = std.filter(function(name) targets[name].rustTriple != null, std.objectFields(targets));

local buildsByTarget = {
  ['build-' + name]: {
    local useCross = targets[name].rustTriple != null,
    executor: 'std:commands',
    options: {
      commands: [
        {
          program: if useCross then 'cross' else 'cargo',
          arguments: cargoArgs + [
                       'build',
                     ]
                     + (if targets[name].release then ['--release'] else [])
                     + (if targets[name].rustTriple != null then ['--target', targets[name].rustTriple] else []),
          environment: {
                         CARGO_TARGET_DIR: blaze.project.root + '/' + targets[name].targetDir,
                       } + LocalEnv(targets[name])
                       + (if useCross then {
                            BLAZE_ROOT: blaze.root,
                            CROSS_CONFIG: blaze.root + '/Cross.toml',
                          } else {}),
        },
      ],
    },
    cache: {
      invalidateWhen: {
        outputChanges: [
          {
            root: targets[name].cli.outputPath,
            pattern: targets[name].cli.filename,
          },
        ],
      },
    },
    dependencies: [
      'source',
    ] + (if useCross then ['ci:docker-authenticate'] else []),
  }
  for name in std.objectFields(targets)
};

local deploymentsByTarget = {
  ['deploy-' + name]: {
    executor: {
      url: 'https://github.com/rnza0u/blaze-executors.git',
      path: 'package-binaries',
      format: 'Git',
      pull: true,
    },
    options: {
      binPath: targets[name].cli.outputPath + '/' + targets[name].cli.filename,
      outputPath: '/var/lib/blaze/builds',
      platform: name,
      overwrite: true,
    },
    dependencies: [
      'build-' + name,
    ],
  }
  for name in finalTargets
};

{
  targets: buildsByTarget + deploymentsByTarget + {
    'generate-lockfile': {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: cargoArgs + ['generate-lockfile'],
          },
        ],
      },
    },
    source: {
      cache: {
        invalidateWhen: {
          inputChanges: [
            'src/**',
            'Cargo.toml',
            'Cargo.lock'
          ],
        },
      },
      dependencies: [dep.project + ':source' for dep in workspaceDependencies],
    },
    run: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: cargoArgs + [
              'run',
              '--',
            ] + blaze.vars.runArgs,
            environment: LocalEnv(targets.dev),
          },
        ],
        shell: true,
      },
      dependencies: ['source'],
    },
    build: {
      cache: {},
      dependencies: [
        'build-dev',
      ],
    },
    install: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: cargoArgs + [
              'install',
              '--force',
              '--path',
              blaze.project.root,
            ],
            environment: LocalEnv(targets.release),
          },
        ],
      },
      dependencies: [
        'source',
      ],
    },
    publish: {
      executor: {
        url: 'https://github.com/rnza0u/blaze-executors.git',
        path: 'cargo-publish',
        pull: true,
        format: 'Git',
      },
      options: {
        releaseVersion: blaze.vars.publish.version,
        linkedDependencies: {
          runtime: [dep.crate for dep in workspaceDependencies],
        },
      },
      dependencies: [
        {
          projects: [dep.project for dep in workspaceDependencies],
          target: 'publish',
        },
      ],
    },
    lint: {
      executor: 'std:commands',
      options: {
        commands: (if blaze.vars.lint.fix then [
                     {
                       program: 'cargo',
                       arguments: cargoArgs + ['fmt'],
                       environment: LocalEnv(targets.dev),
                     },
                   ] else []) + [
          {
            program: 'cargo',
            arguments: cargoArgs + ['check'],
            environment: LocalEnv(targets.dev),
          },
          {
            program: 'cargo',
            arguments: cargoArgs + ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else []),
            environment: LocalEnv(targets.dev),
          },
        ],
      },
      dependencies: ['source'],
    },
    clean: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: cargoArgs + ['clean'],
          },
        ] + [
          {
            program: 'cargo',
            arguments: cargoArgs + ['clean'],
            environment: {
              CARGO_TARGET_DIR: blaze.project.root + '/' + targets[name].targetDir,
            },
          }
          for name in finalTargets
        ],
      },
    },
    deploy: {
      dependencies: ['deploy-' + name for name in finalTargets],
    },
    'build-all': {
      dependencies: ['build-' + name for name in finalTargets],
    },
  },
}
