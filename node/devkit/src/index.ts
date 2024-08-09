import { z } from 'zod'

const logLevelSchema = z.union([
    z.literal('Debug'),
    z.literal('Error'),
    z.literal('Info'),
    z.literal('Warn')
])

const projectSelectorSchema = z.union([
    z.object({
        include: z.array(z.string().min(1)),
        exclude: z.array(z.string().min(1))
    }),
    z.array(z.string().min(1)),
    z.literal('All'),
    z.object({
        tags: z.array(z.string().min(1))
    })
])

const configurationFileFormatSchema = z.union([
    z.literal('Json'),
    z.literal('Jsonnet'),
    z.literal('Yaml')
])

export type Value = string | number | boolean | null | { [k: string]: Value } | Value[]

export const valueSchema = z.custom<Value>(value => value !== undefined)

const targetDependencySchema = z.object({
    projects: projectSelectorSchema.optional(),
    target: z.string().min(1),
    cachePropagation: z.union([
        z.literal('Always'),
        z.literal('Never')
    ]),
    optional: z.boolean()
})

const matchingBehaviorSchema = z.union([
    z.literal('Mixed'),
    z.literal('Hash'),
    z.literal('Timestamps')
])

const fileChangesMatchersSchema = z.object({
    pattern: z.string().min(1),
    exclude: z.array(z.string().min(1)),
    root: z.string().min(1).optional(),
    behavior: matchingBehaviorSchema
})

const ttlSchema = z.object({
    unit: z.union([
        z.literal('Milliseconds'),
        z.literal('Seconds'),
        z.literal('Hours'),
        z.literal('Days')
    ])
})

const commandFailsSchema = z.object({
    program: z.string().min(1),
    arguments: z.array(z.string()),
    environment: z.record(z.string()),
    cwd: z.string().min(1).optional(),
    verbose: z.boolean()
})

const rebuildStrategySchema = z.union([
    z.literal('Always'), 
    z.literal('OnChanges')
])

const executorKindSchema = z.union([
    z.literal('Rust'),
    z.literal('Node')
])

const gitOptionsProperties = {
    branch: z.string().min(1).optional(),
    rev: z.string().min(1).optional(),
    tag: z.string().min(1).optional(),
    path: z.string().min(1).optional(),
    pull: z.boolean()
} as const

const httpTransportProperties = {
    insecure: z.boolean(),
    headers: z.record(z.string().min(1)),
} as const

const sshTransportProperties = {
    insecure: z.boolean(),
    fingerprints: z.array(z.string().min(1)).optional()
} as const

const sshAuthentication = z.union([
    z.object({
        username: z.string().min(1).optional(),
        password: z.string().min(1)
    }),
    z.object({
        username: z.string().min(1).optional(),
        key: z.string().min(1),
        passphrase: z.string().min(1).optional()
    })
])

const targetExecutorSchema = z.union([
    z.object({
        url: z.string().url()
    }),
    z.object({
        url: z.string().url(),
        watch: z.array(fileChangesMatchersSchema).optional(),
        rebuild: rebuildStrategySchema,
        kind: executorKindSchema.optional()
    }),
    z.object({
        url: z.string().url(),
        ...gitOptionsProperties,
        ...httpTransportProperties,
        authentication: z.object({
            username: z.string().min(1),
            password: z.string().min(1)
        }).optional()
    }),
    z.object({
        url: z.string().url(),
        ...sshTransportProperties,
        ...gitOptionsProperties,
        authentication: sshAuthentication.optional()
    })
])

const targetCacheSchema = z.object({
    invalidateWhen: z.object({
        fileChanges: z.array(fileChangesMatchersSchema).optional(),
        filesMissing: z.array(z.string().min(1)).optional(),
        ttl: ttlSchema.optional(),
        commandFails: commandFailsSchema.optional()
    }).optional()
})

const targetConfigurationSchema = z.object({
    executor: targetExecutorSchema.optional(),
    options: valueSchema,
    dependencies: z.array(targetDependencySchema),
    cache: targetCacheSchema.optional(),
    stateless: z.boolean()
})

export const projectSchema = z.object({
    root: z.string().min(1),
    configurationFileFormat: configurationFileFormatSchema,
    configurationFilePath: z.string().min(1),
    name: z.string().min(1),
    targets: z.record(targetConfigurationSchema)
})

const loggerSchema = z.object({
    info: z.function().args(z.string()).returns(z.void()),
    warn: z.function().args(z.string()).returns(z.void()),
    error: z.function().args(z.string()).returns(z.void()),
    debug: z.function().args(z.string()).returns(z.void()),
    log: z.function().args(z.string(), logLevelSchema).returns(z.void()),
})

export const projectRefSchema = z.object({
    path: z.string().min(1),
    description: z.string().optional(),
    tags: z.array(z.string().min(1))
})

export const workspaceSchema = z.object({
    root: z.string().min(1),
    configurationFileFormat: configurationFileFormatSchema,
    configurationFilePath: z.string().min(1),
    name: z.string().min(1),
    projects: z.record(projectRefSchema),
    settings: z.object({
        parallelism: z.union([
            z.number().min(1),
            z.literal('All'),
            z.literal('Infinite'),
            z.literal('None')
        ]).optional(),
        defaultSelector: projectSelectorSchema.optional().optional(),
        selectors: z.record(projectSelectorSchema),
        logLevel: logLevelSchema.optional()
    })
})

const executorContextSchema = z.object({
    logger: loggerSchema,
    workspace: workspaceSchema,
    project: projectSchema,
    target: z.string().min(1)
})

export type ExecutorContext = z.infer<typeof executorContextSchema>

export const executorFunctionSchema = z.function()
    .args(
        executorContextSchema,
        valueSchema
    )
    .returns(z.union([
        z.promise(z.void()),
        z.void()
    ]))

export type Executor = z.infer<typeof executorFunctionSchema>

export type ExecutorResult = ReturnType<Executor>