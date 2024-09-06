import { Executor } from '@blaze-repo/node-devkit'
import { deepEqual, equal, ok } from 'assert'
import { isAbsolute, join } from 'path'
import { Chance } from 'chance'

const executor: Executor = (ctx, options) => {

    const chance = new Chance()

    // options check
    deepEqual(
        options, 
        {
            number: 1,
            string: 'hello',
            bool: true,
            array: [1, 2, 3],
            null: null,
            float: 1.0
        }
    )

    // workspace check
    ok(isAbsolute(ctx.workspace.root))
    equal(ctx.workspace.name, 'workspace')
    equal(ctx.workspace.configurationFileFormat, 'Json')
    equal(ctx.workspace.configurationFilePath, join(ctx.workspace.root, 'workspace.json'))
    deepEqual(ctx.workspace.projects, { project: { path: 'project', tags: [] } })

    // project check
    equal(ctx.project.name, 'project')
    equal(ctx.project.configurationFileFormat, 'Json')
    ok(isAbsolute(ctx.project.root))
    equal(ctx.project.root, join(ctx.workspace.root, 'project'))
    equal(ctx.project.configurationFilePath, join(ctx.workspace.root, 'project/project.json'))

    equal(ctx.target, 'target')

    ctx.logger.info('hello from node')
    ctx.logger.error('error from node')
    ctx.logger.warn('warning from node')
    ctx.logger.debug('debug from node')

    if (chance.bool())
        return Promise.resolve()
}

export default executor