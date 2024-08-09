import process from 'node:process'
import { Socket, connect } from 'node:net'
import { EOL } from 'node:os'
import { ExecutorContext, executorFunctionSchema, projectSchema, valueSchema, workspaceSchema } from '@blaze-repo/node-devkit'
import { z } from 'zod'

const bridgeInputMessageSchema = z.object({
    executorParams: z.tuple([
        z.object({
            workspace: workspaceSchema,
            project: projectSchema,
            target: z.string().min(1),
            logger: z.string().min(1)
        }),
        valueSchema
    ]),
    metadata: z.object({
        module: z.string().min(1),
    })
})

type BridgeInputMessage = z.infer<typeof bridgeInputMessageSchema>

type BridgeContext = ExecutorContext & {
    logger: {
        _drop: () => void
    }
}

function dropContext(context: BridgeContext): void {
    context.logger._drop()
}

function connectPipe(path: string): Promise<Socket> {
    return new Promise((resolve, reject) => {
        const stream = connect(path)
        const connectErrorListener = (err: Error) => reject(err)
        
        stream.once('error', connectErrorListener)
        stream.once('ready', () => {
            stream.removeListener('error', connectErrorListener)
            resolve(stream)
        })
    })
}

async function convertContext(inputContext: BridgeInputMessage['executorParams'][0]): Promise<BridgeContext> {
    const logStream = await connectPipe(inputContext.logger)
    
    const logger: BridgeContext['logger'] = {
        _drop() {
            logStream.end()
        },
        log(message, level){
            logStream.write(
                JSON.stringify({
                    level,
                    message
                }) + EOL, 
                err => {
                    if (!err)
                        return
                    // switch back to Node console in case of error
                    switch(level){
                        case 'Info':
                            console.log(message)
                            break
                        case 'Error':
                            console.error(message)
                            break
                        case 'Debug':
                            console.debug(message)
                            break
                        case 'Warn':
                            console.warn(message)
                            break
                    }
                }
            )
        },
        debug(message) {
            this.log(message, 'Debug')
        },
        error(message) {
            this.log(message, 'Error')
        },
        warn(message) {
            this.log(message, 'Warn')
        },
        info(message) {
            this.log(message, 'Info')
        },
    }

    logStream.on('error', err => {
        console.error(`log channel error (${err})`)
    })

    const context = {
        ...inputContext,
        logger
    }

    Object.freeze(context)

    return context
}

let convertedContext: null | BridgeContext = null

const start = async (): Promise<void> => {

    const inputMessage: unknown = JSON.parse(process.argv[process.argv.length - 1])

    const {
        metadata: { module },
        executorParams: [context, options]
    } = await bridgeInputMessageSchema.parseAsync(inputMessage)

    let defaultExport: unknown
    try {
        defaultExport = (await import(`file://${module}`)).default
    } catch (err) {
        throw Error(`Executor module could not be found at ${module}, please check the value of \`blaze.path\` in your package.json file (${err})`)
    }

    convertedContext = await convertContext(context)

    try {
        const executor = await executorFunctionSchema.parseAsync(defaultExport)
        await executor(
            convertedContext,
            options
        )
    } catch(err){
        if (err instanceof Error){
            convertedContext.logger.error(err.message)
            convertedContext.logger.error(err.stack ?? 'no stack')
        } else {
            convertedContext.logger.error(`${err}`)
        }
        process.exit(1)
    }

    process.exit(0)
}

start()
    .catch(err => {
        const logError = (message: string) => convertedContext?.logger.error(message) ?? console.error(message)
        logError(err instanceof Error && err.stack ? err.stack : `${err}`)
        process.exit(1)
    })
    .finally(() => {
        if (convertedContext) {
            dropContext(convertedContext)
        }
    })
