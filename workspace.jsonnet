{
  name: 'blaze',
  projects: {
    ci: {
      path: 'ci',
      description: 'CI/CD related files',
    },
    'cli': {
      path: 'cli',
      description: 'Blaze command line interface crate.',
      tags: ['rust']
    }, 
    'cli-docs': {
      path: 'cli-docs',
      description: 'Auto generated documentation for Blaze CLI (man files and mdx files for the website)',
      tags: ['rust', 'docs']
    },
    'core': {
      path: 'core',
      description: 'Blaze main library crate.',
      tags: ['rust']
    },
    'tests': {
      path: 'tests',
      description: 'Blaze integration tests suite.',
      tags: ['rust']
    },
    'common': {
      path: 'common',
      description: 'Blaze shared data structures.',
      tags: ['rust']
    },
    'node-bridge': {
      path: 'node/bridge',
      description: 'Blaze Node.js executors bridge script.',
      tags: ['node']
    },
    'node-devkit': {
      path: 'node/devkit',
      description: 'Blaze Node.js executors devkit (library).',
      tags: ['node']
    },
    'rust-bridge': {
      path: 'rust/bridge',
      description: 'Blaze Rust executors bridge executable.',
      tags: ['rust']
    },
    'rust-devkit': {
      path: 'rust/devkit',
      description: 'Blaze Rust executors devkit (library).',
      tags: ['rust']
    },
    'website': {
      path: 'website',
      description: 'Blaze main documentation website.',
      tags: ['node', 'web']
    },
    'assets': {
      path: 'assets',
      description: 'Blaze brand assets.',
      tags: []
    },
    'schemas': {
      path: 'schemas',
      description: 'Blaze JSON schemas.',
      tags: ['node', 'docs']
    },
    'downloads': {
      path: 'downloads',
      description: 'Blaze downloads REST API',
      tags: ['rust', 'web']
    }
  }
}