{
    targets: {
        'build-drone': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'drone',
                        arguments: [
                            'jsonnet', 
                            '--source', '{{ project.root }}/.drone.jsonnet', 
                            '--format', 
                            '--stream', 
                            '--target', '{{ root }}/.drone.yml'
                        ]
                    },
                    {
                        program: 'drone',
                        arguments: [
                            'sign',
                            '--save',
                            'rnza0u/blaze'
                        ],
                        cwd: '{{ root }}'
                    }
                ]
            }
        },
        'docker-authenticate': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'docker-authenticate',
                format: 'Git'
            },
            options: {
                registry: 'registry.rnzaou.me'
            }
        },
        'push-tags': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'push-tags',
                format: 'Git'
            },
            options: {
                dryRun: '{{ vars.publish.dryRun }}',
                tags: ['{{ vars.publish.version }}'],
            },
        },
    }
}