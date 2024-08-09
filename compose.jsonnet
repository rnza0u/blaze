// just a draft file format for blaze compose mode
{
    steps: [
        {
            name: 'step-1',
            targets: [
                'project:build',
                {
                    projects: ['project-1', 'project-2'],
                    target: 'build',
                    optional: true
                }
            ],
            when: 'Always',
            parallelism: 'All'
        }
    ]
}