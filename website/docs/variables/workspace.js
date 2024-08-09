export default [
    {
        path: 'workspace.root',
        description: <>Absolute path to the workspace root directory. Use <code>root</code> instead.</>
    },
    {
        path: 'workspace.name',
        description: 'The name of the workspace'
    },
    {
        path: 'workspace.projects',
        description: 'Project references declared at the workspace level',
        example: {
            'project-1': {
                path: 'path/to/project-1',
            },
            'project-2': {
                path: 'path/to/project-2'
            }
        }
    },
    {
        path: 'workspace.projects.*.path',
        description: 'Project reference relative path, from the workspace root directory.',
        example: 'path/to/project-1'
    }
]