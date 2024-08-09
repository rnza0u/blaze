export default [
    {
        path: 'root',
        description: 'The workspace root directory, as an absolute path.',
        example: '/path/to/my/workspace'
    },
    {
        path: 'environment.*',
        description: 'The current process environment variables (after loading all .env files).',
        example: {
            HOME: '/home/user',
            SHELL: '/bin/sh'
        }
    },
    {
        path: 'architecture',
        description: <>The current platform architecture. Possible values <a target="_blank" href="https://doc.rust-lang.org/std/env/consts/constant.ARCH.html">are listed here</a>.</>,
        example: 'x86_64'
    },
    {
        path: 'family',
        description: <>The operating system family. Can be either <code>unix</code> or <code>windows</code>.</>,
        example: 'unix'
    },
    {
        path: 'os',
        description: <>The specific current operating system. Possible values <a target="_blank" href="https://doc.rust-lang.org/std/env/consts/constant.OS.html">are listed here</a>.</>,
        example: 'linux'
    },
    {
        path: 'user',
        description: 'The current user name.',
        example: 'root'
    },
    {
        path: 'hostname',
        description: 'The current machine hostname. This variable can be unset in some cases.',
        hostname: 'my-machine-name'
    },
    {
        path: 'sep',
        description: 'Filesystem path component separator for the current platform',
        example: '/'
    }
]