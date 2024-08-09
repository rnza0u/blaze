local blaze = std.extVar('blaze');
local targets = import '../targets.jsonnet';
local LocalEnv = import '../core/local-env.jsonnet';
local cargo = (import 'cargo.libsonnet')(blaze.vars.blaze.rust.channel, ['-Z', 'bindeps']);

local cargoTargets = cargo.all({
    workspaceDependencies: ['blaze-cli'],
    environment: LocalEnv(targets.dev)
});

{
    targets: cargoTargets + {
        build: {
            executor: 'std:commands',
            description: 'Build the documentation files.',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: ['+' + blaze.vars.blaze.rust.channel, 'run', '-Z', 'bindeps', '--release'],
                        environment: LocalEnv(targets.release) + {
                            OUT_DIR: '{{ project.root }}/dist'
                        }
                    }
                ],
            },
            cache: {
                invalidateWhen: {
                    outputChanges: ['dist/**']
                }
            },
            dependencies: [
                'source'
            ]
        },
        ci: {
            dependencies: ['lint', 'check']
        }
    }
}