local blaze = std.extVar('blaze');
local LocalEnv = import '../core/local-env.jsonnet';
local targets = import '../targets.jsonnet';
local cargoExtraOpt = ['-Z', 'bindeps'];
local cargo = (import 'cargo.libsonnet')(blaze.vars.blaze.rust.channel, cargoExtraOpt);
local executors = import 'executors.libsonnet';

local workspaceDependencies = ['blaze-common', 'blaze-core'];

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
                       '+' + blaze.vars.blaze.rust.channel,
                       'build',
                     ]
                     + (if targets[name].release then ['--release'] else [])
                     + (if targets[name].rustTriple != null then ['--target', targets[name].rustTriple] else []),
          environment: {
                         CARGO_TARGET_DIR: blaze.project.root + '/' + targets[name].targetDir,
                       } + LocalEnv(targets[name])
                       + (if useCross then {
                            BLAZE_ROOT: blaze.root + '/blaze',
                            CROSS_CONFIG: blaze.root + '/blaze/Cross.toml',
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
    ] + (if useCross then ['docker-registry:authenticate'] else []),
  }
  for name in std.objectFields(targets)
};

local deploymentsByTarget = {
  ['deploy-' + name]: {
    executor: executors.packageBinaries(),
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

local cargoTargets = cargo.all({
  workspaceDependencies: workspaceDependencies,
  environment: LocalEnv(targets.dev),
  extraTargetDirs: std.map(
    function(name) 'target-' + targets[name].rustTriple,
    finalTargets
  ),
});

{
  targets: cargoTargets + buildsByTarget + deploymentsByTarget + {
    run: {
      executor: 'std:commands',
      options: {
        commands: [
          {
            program: 'cargo',
            arguments: [
              '+' + blaze.vars.blaze.rust.channel,
              'run',
            ] + ['--'] + blaze.vars.blaze.runArgs,
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
            arguments: ['+' + blaze.vars.blaze.rust.channel, 'install', '--force', '--path', '{{ project.root }}'],
            environment: LocalEnv(targets.release),
          },
        ],
      },
      dependencies: [
        'source',
      ],
    },
    'publish-crate': {
      executor: executors.cargoPublish(),
      options: {
        dryRun: blaze.vars.blaze.publish.dryRun,
        channel: blaze.vars.blaze.rust.channel,
      },
      dependencies: [
        'check-version',
        {
          projects: workspaceDependencies,
          target: 'publish',
        },
      ],
    },
    'push-tags': {
      executor: executors.pushTags(),
      options: {
        dryRun: blaze.vars.blaze.publish.dryRun,
        tags: ['blaze-' + blaze.vars.blaze.publish.version],
      },
    },
    'check-version': {
      executor: executors.cargoVersionCheck(),
      options: {
        version: blaze.vars.blaze.publish.version,
        workspaceDependencies: workspaceDependencies,
      },
    },
    'deploy-all': {
      dependencies: ['deploy-' + name for name in finalTargets],
    },
    ci: {
      dependencies: ['lint', 'check'],
    },
    publish: {
      dependencies: ['publish-crate', 'push-tags'],
    },
  },
}
