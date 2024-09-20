local blaze = std.extVar('blaze');
local LocalEnv = import '../core/local-env.jsonnet';
local targets = import '../targets.jsonnet';

local workspaceDependencies = [
  { crate: 'blaze-common', project: 'common' }, 
  { crate: 'blaze-core', project: 'core' }
];

local finalTargets = std.filter(function(name) targets[name].rustTriple != null, std.objectFields(targets));

local buildsByTarget = {
  ['build-' + name]: {
    local useCross = targets[name].rustTriple != null,
    executor: 'std:commands',
    options: {
      commands: [
        {
          program: if useCross then 'cross' else 'cargo',
          arguments: [
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
    ] + (if useCross then ['ci:docker-authenticate'] else [])
  }
  for name in std.objectFields(targets)
};

local deploymentsByTarget = {
  ['deploy-' + name]: {
    executor: {
      url: 'https://github.com/rnza0u/blaze-executors.git',
      path: 'package-binaries',
      format: 'Git',
      pull: true
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
    run: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: [
              'run',
              '--'
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
            arguments: [
              'install', 
              '--force', 
              '--path', 
              blaze.project.root
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
        format: 'Git'
      },
      options: {
        releaseVersion: blaze.vars.publish.version,
        linkedDependencies: {
          runtime: [dep.crate for dep in workspaceDependencies]
        }
      },
      dependencies: [
        {
          projects: [dep.project for dep in workspaceDependencies],
          target: 'publish',
        },
      ],
    },
    source: {
      cache: {
        invalidateWhen: {
          inputChanges: [
            'src/**',
            'Cargo.toml'
          ],
          outputChanges: ['Cargo.lock']
        }
      },
      dependencies: [dep.project + ':source' for dep in workspaceDependencies]
    },
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
    clean: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: ['clean']
          }
        ] + [
          {  
            program: 'cargo',
            arguments: ['clean'],
            environment: {
              CARGO_TARGET_DIR: blaze.project.root + '/' + targets[name].targetDir,
            }
          } for name in finalTargets
        ]
      }
    },
    deploy: {
      dependencies: ['deploy-' + name for name in finalTargets],
    },
    'build-all': {
      dependencies: ['build-' + name for name in finalTargets]
    }
  },
}
