import React from 'react'
import type { DocsThemeConfig } from 'nextra-theme-docs'

const config: DocsThemeConfig = {
  logo: (
    <>
      <img src="/synapse_logo.png" alt="Synapse" height={36} style={{ height: 36 }} />
    </>
  ),
  project: {
    link: 'https://github.com/krypticmouse/synapse',
  },
  docsRepositoryBase: 'https://github.com/krypticmouse/synapse/tree/master/docs',
  useNextSeoProps() {
    return {
      titleTemplate: '%s | Synapse',
    }
  },
  head: (
    <>
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      <meta name="description" content="Synapse: Configuration Language for Memory Systems" />
      <meta name="og:title" content="Synapse Documentation" />
    </>
  ),
  primaryHue: 270,
  primarySaturation: 80,
  sidebar: {
    defaultMenuCollapseLevel: 1,
    toggleButton: true,
  },
  toc: {
    backToTop: true,
  },
  footer: {
    text: (
      <span>
        MIT {new Date().getFullYear()} &copy; Synapse. Configuration Language for Memory Systems.
      </span>
    ),
  },
  editLink: {
    text: 'Edit this page on GitHub',
  },
  feedback: {
    content: 'Question? Give us feedback →',
    labels: 'feedback',
  },
  navigation: {
    prev: true,
    next: true,
  },
}

export default config
