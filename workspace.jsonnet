{
  name: 'blaze',
  projects: {
    ci: {
      path: 'ci',
      description: 'CI/CD related files',
    },
    'blaze-cli': {
      path: 'cli',
      description: 'Blaze command line interface crate.',
      tags: ['rust']
    }, 
    'blaze-cli-docs': {
      path: 'cli-docs',
      description: 'Auto generated documentation for Blaze CLI (man files and mdx files for the website)',
      tags: ['rust', 'docs']
    },
    'blaze-core': {
      path: 'core',
      description: 'Blaze main library crate.',
      tags: ['rust']
    },
    'blaze-tests': {
      path: 'tests',
      description: 'Blaze integration tests suite.',
      tags: ['rust']
    },
    'blaze-common': {
      path: 'common',
      description: 'Blaze shared data structures.',
      tags: ['rust']
    },
    'blaze-node-bridge': {
      path: 'node/bridge',
      description: 'Blaze Node.js executors bridge script.',
      tags: ['node']
    },
    'blaze-node-devkit': {
      path: 'node/devkit',
      description: 'Blaze Node.js executors devkit (library).',
      tags: ['node']
    },
    'blaze-rust-bridge': {
      path: 'rust/bridge',
      description: 'Blaze Rust executors bridge executable.',
      tags: ['rust']
    },
    'blaze-rust-devkit': {
      path: 'rust/devkit',
      description: 'Blaze Rust executors devkit (library).',
      tags: ['rust']
    },
    'blaze-website': {
      path: 'website',
      description: 'Blaze main documentation website.',
      tags: ['node', 'web']
    },
    'blaze-assets': {
      path: 'assets',
      description: 'Blaze brand assets.',
      tags: []
    },
    'blaze-schemas': {
      path: 'schemas',
      description: 'Blaze JSON schemas.',
      tags: ['node', 'docs']
    },
    'blaze-downloads': {
      path: 'downloads',
      description: 'Blaze downloads REST API',
      tags: ['rust', 'web']
    }
  }
}