// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const { 
    themes: { 
        github: lightCodeTheme, 
        dracula: darkCodeTheme 
    } 
} = require('prism-react-renderer')

/** @type {import('@docusaurus/types').Config} */
const config = {
    title: 'Blaze',
    tagline: 'A fast, simple and flexible build system.',
    favicon: 'favicons/favicon.ico',
    staticDirectories: [process.env['ASSETS_LOCATION'], 'static'],

    // Set the production url of your site here
    url: 'https://blaze-monorepo.dev',
    // Set the /<baseUrl>/ pathname under which your site is served
    // For GitHub pages deployment, it is often '/<projectName>/'
    baseUrl: '/',

    // GitHub pages deployment config.
    // If you aren't using GitHub pages, you don't need these.
    organizationName: 'rnza0u', // Usually your GitHub org/user name.
    projectName: 'blaze', // Usually your repo name.

    onBrokenLinks: 'throw',
    onBrokenMarkdownLinks: 'throw',
    onDuplicateRoutes: 'throw',

    // Even if you don't use internalization, you can use this field to set useful
    // metadata like html lang. For example, if your site is Chinese, you may want
    // to replace "en" with "zh-Hans".
    i18n: {
        defaultLocale: 'en',
        locales: ['en'],
    },

    presets: [
        [
            'classic',
            /** @type {import('@docusaurus/preset-classic').Options} */
            ({
                docs: {
                    sidebarPath: require.resolve('./sidebars.js'),
                    // Please change this to your repo.
                    // Remove this to remove the "edit this page" links.
                    // editUrl:
                    //  'https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/',
                },
                //blog: {
                //  showReadingTime: true,
                // Please change this to your repo.
                // Remove this to remove the "edit this page" links.
                //  editUrl:
                //    'https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/',
                // },
                theme: {
                    customCss: require.resolve('./src/css/custom.css'),
                },
            }),
        ],
    ],

    themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
        announcementBar: {
            id: 'wip',
            content: '🚧 Blaze is still very much under development and will release in alpha soon. Help us stabilize it by <a href="/downloads" target="_blank">testing it</a> ! 🚧',
            backgroundColor: 'var(--ifm-color-warning-light)',
            textColor: 'black'
        },

        // Replace with your project's social card
        colorMode: {
            defaultMode: 'dark',
            respectPrefersColorScheme: false
        },
        image: 'assets/logos/Blaze-logos-rounded.png',
        algolia: {
            appId: 'M58RO1JY7H',
            apiKey: '80144c83096b621942a21d2830b1760b',
            indexName: 'blaze-monorepo'
        },
        navbar: {
            title: 'Blaze',
            items: [
                {
                    type: 'docSidebar',
                    sidebarId: 'tutorialSidebar',
                    position: 'left',
                    label: 'Documentation',
                },
                {
                    position: 'left',
                    label: 'Downloads',
                    to: 'downloads'
                },
                {
                    href: 'https://github.com/rnza0u/blaze.git',
                    label: 'GitHub',
                    position: 'right',
                },
            ],
        },
        footer: {

            style: 'dark',
            links: [
                {
                    title: 'Community',
                    items: [
                        {
                            label: 'About',
                            href: '/about',
                        },
                        {
                            label: 'Discord',
                            href: 'https://discord.gg/htKUS7wc',
                        },
                    ],
                },
            ],
            copyright: `Copyright © ${new Date().getFullYear()} Blaze contributors.`,
        },
        prism: {
            theme: lightCodeTheme,
            darkTheme: darkCodeTheme,
            additionalLanguages: [
                'rust',
                'toml',
                'json'
            ]
        }
    }),
    themes: ['docusaurus-json-schema-plugin']
}

module.exports = config
