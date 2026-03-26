import nextra from 'nextra'
import { readFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const mnmGrammar = JSON.parse(
  readFileSync(resolve(__dirname, './grammar/mnm.tmLanguage.json'), 'utf-8')
)

const withNextra = nextra({
  theme: 'nextra-theme-docs',
  themeConfig: './theme.config.tsx',
  defaultShowCopyCode: true,
  latex: true,
  mdxOptions: {
    rehypePrettyCodeOptions: {
      getHighlighter: async (options) => {
        const { getHighlighter } = await import('shiki')
        const highlighter = await getHighlighter({
          ...options,
          langs: [
            ...(options.langs || []),
            {
              id: 'mnm',
              scopeName: 'source.mnm',
              grammar: mnmGrammar,
              aliases: ['mnm', 'synapse'],
            },
          ],
        })
        return highlighter
      },
    },
  },
})

export default withNextra({
  output: 'export',
  images: { unoptimized: true },
})
