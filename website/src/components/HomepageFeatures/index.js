import React from 'react'
import clsx from 'clsx'
import styles from './styles.module.css'

const FeatureList = [
    {
        title: 'One tool to rule them all 👑',
        description: (
            <>
                Blaze manages all the dependencies between your projects. Collaboration between different teams becomes very easy.
            </>
        ),
    },
    {
        title: 'Tired of writing a CI/CD chain for each and every project ? 😫',
        description: (
            <>
                With Blaze, you can make your whole integration and deployment process part of the monorepo. Write it once and for all.
            </>
        ),
    },
    {
        title: 'Re-use everything ♻️',
        description: (
            <>
                Writing reusable code has never been easier. You don't even need to publish your libraries if they are not intended for public use.
            </>
        ),
    },
    {
        title: 'Not invasive 👍',
        description: (
            <>
                Blaze supports any language/framework and will not have any impact on your application code.
            </>
        )
    },
    {
        title: 'Save your time ⏱️',
        description: (
            <>
                Thanks to Blaze cache system, you will never need to redo what's already done.
            </>
        )
    },
    {
        title: 'Blazing fast 🔥',
        description: (
            <>
                Blaze is built with performance in mind. It is written in Rust and supports parallel tasks execution.
            </>
        )
    }
]

function Feature({ title, description }) {
    return (
        <div className={clsx('col col--4')}>
            <div className="text--center padding-horiz--md">
                <h3>{title}</h3>
                <p>{description}</p>
            </div>
        </div>
    )
}

export default function HomepageFeatures() {
    return (
        <section className={styles.features}>
            <div className="container">
                <div className="row">
                    {FeatureList.map((props, idx) => (
                        <Feature key={idx} {...props} />
                    ))}
                </div>
            </div>
        </section>
    )
}
