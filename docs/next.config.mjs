import nextra from 'nextra'
import { readFileSync } from 'fs'
import { resolve, dirname } from 'path'
import { fileURLToPath } from 'url'
import { createHighlighter } from 'shiki'

const __dirname = dirname(fileURLToPath(import.meta.url))
const mnmGrammar = JSON.parse(
  readFileSync(resolve(__dirname, './grammar/mnm.tmLanguage.json'), 'utf-8')
)

const withNextra = nextra({
  latex: true,
  defaultShowCopyCode: true,
  search: {
    codeblocks: true
  },
  mdxOptions: {
    rehypePrettyCodeOptions: {
      getHighlighter: options =>
        createHighlighter({
          ...options,
          langs: [
            ...(options.langs || []),
            {
              name: 'mnm',
              scopeName: 'source.mnm',
              ...mnmGrammar,
            },
          ],
        }),
    },
  },
})

export default withNextra({
  output: 'export',
  images: { unoptimized: true },
})
