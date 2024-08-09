import React from 'react'
import clsx from 'clsx'
import useDocusaurusContext from '@docusaurus/useDocusaurusContext'
import Layout from '@theme/Layout'
import HomepageFeatures from '@site/src/components/HomepageFeatures'

import styles from './index.module.css'

import '@uiw/icons/fonts/w-icon.css'

import BlazeLogo from '../../../assets/logos/Blaze-logo.svg'

function HomepageHeader() {
    const { siteConfig } = useDocusaurusContext()

    return (
        <header className={clsx('hero hero--dark', styles.heroBanner)}>
            <div className="container">
                <BlazeLogo className={styles.mainLogo}/>
                <p className="hero__subtitle">{siteConfig.tagline}</p>
                <div className={styles.buttons}>
                    <a href="/docs/introduction" className="button button--secondary d-flex align-center">
                        <i className="w-icon-login"></i>
                        Discover
                    </a>
                    <a href="/downloads" className="button button--secondary d-flex align-center">
                        <i className="w-icon-download"></i>
                        Downloads
                    </a>
                </div>
            </div>
        </header>
    )
}

export default function Home() {
    return (
        <Layout
            title={'Home'}
            description="Blaze is a monorepo-based build system.">
            <HomepageHeader />
            <main role="main">
                <HomepageFeatures />
            </main>
        </Layout>
    )
}
