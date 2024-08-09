local npm = import 'npm.libsonnet';
local executors = import 'executors.libsonnet';
local blaze = std.extVar('blaze');

local npmTargets = npm.all('@blaze-repos/json-schemas', {
    cleanExtraDirectories: ['schemas']
});

{
    targets: npmTargets + {
        publish: {
            executor: executors.npmPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun
            },
            dependencies: ['check-version', 'build']
        },
        'check-version': {
            executor: executors.npmVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version
            }
        },
        'build-generator': npmTargets.build,
        build: {
            executor: 'std:commands',
            description: 'Build the JSON schemas.',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: ['-rf', 'schemas']
                    },
                    {
                        program: 'mkdir',
                        arguments: ['-p', 'schemas']
                    },
                    {
                        program: 'npm',
                        arguments: ['start']
                    }
                ]
            },
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'schemas/**'
                    ]
                }
            },
            dependencies: [
                'build-generator'
            ]
        },
    }
}