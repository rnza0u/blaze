import { Schema } from './utils.js'

export const parallelismSchema = {
    oneOf: [
        {
            const: 'Infinite',
            description: 'No limit on maximum parallel jobs.'
        },
        {
            const: 'All',
            description: 'Maximum parallel jobs set to the number of available logical cores.'
        },
        {
            type: 'number',
            description: 'Explicitely set the number of parallel jobs.',
            minimum: 1
        },
        {
            const: 'None',
            description: 'No parallelism at all, which means only one job will be executed at a time. This is the same as providing 1.'
        }
    ],
    default: 'None'
} as const satisfies Schema