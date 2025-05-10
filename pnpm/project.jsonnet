local blaze = std.extVar('blaze');

local pnpmProjectRoots = std.filterMap(
  function (project) std.member(project.tags, 'pnpm'), 
  function (project) blaze.workspace.root + '/' + project.path, 
  std.objectValues(blaze.workspace.projects)
);

{
  targets: {
    'install': {
      executor: 'std:commands',
      description: 'Install PNPM dependencies across the workspace.',
      options: {
        commands: [
          {
            program: 'pnpm',
            arguments: (if blaze.vars.ci then ['--frozen-lockfile'] else []) + [
              'install'
            ]
          }
        ]
      },
      cache: {
        invalidateWhen: {
          filesMissing: std.map(function (root) root + '/node_modules', pnpmProjectRoots),
          inputChanges: [
            {
              root: '{{ root }}',
              pattern: 'pnpm-workspace.yaml'
            },
          ] + std.map(function (root) {
              root: root,
              pattern: 'package.json'
            }, pnpmProjectRoots),
          outputChanges: [
            {
              root: '{{ root }}',
              pattern: 'pnpm-lock.yaml'
            }
          ]
        }
      }
    },
    clean: {
      executor: 'std:commands',
      description: 'Remove all dependencies from the PNPM workspace.',
      options: {
        commands: [
          'rm -rf {{ root }}/node_modules',
        ] + std.map(function (root) 'rm -rf ' + root + '/node_modules', pnpmProjectRoots)
      }
    }
  }
}