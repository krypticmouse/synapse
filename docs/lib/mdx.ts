import fs from 'fs'
import path from 'path'
import { compileMDX } from 'next-mdx-remote/rsc'
import rehypePrettyCode from 'rehype-pretty-code'
import rehypeSlug from 'rehype-slug'
import remarkGfm from 'remark-gfm'
import { createHighlighter } from 'shiki'
import { mdxComponents } from '@/components/mdx-components'

const contentDir = path.join(process.cwd(), 'content')
const mnmGrammar = JSON.parse(
  fs.readFileSync(path.join(process.cwd(), 'grammar', 'mnm.tmLanguage.json'), 'utf-8')
)

export interface Heading {
  id: string
  text: string
  level: number
}

export interface PageData {
  content: React.ReactElement
  headings: Heading[]
  title: string | undefined
}

export async function getPage(slug: string[]): Promise<PageData | null> {
  const filePath =
    slug.length === 0
      ? path.join(contentDir, 'index.mdx')
      : path.join(contentDir, ...slug) + '.mdx'

  if (!fs.existsSync(filePath)) {
    return null
  }

  const raw = fs.readFileSync(filePath, 'utf-8')

  const source = raw
    .replace(/^import\s+\{[^}]*\}\s+from\s+['"]nextra\/components['"];?\s*$/gm, '')
    .replace(/^import\s+\{[^}]*\}\s+from\s+['"]nextra-theme-docs['"];?\s*$/gm, '')

  const { content } = await compileMDX({
    source,
    components: mdxComponents,
    options: {
      mdxOptions: {
        remarkPlugins: [remarkGfm],
        rehypePlugins: [
          rehypeSlug,
          [
            rehypePrettyCode as any,
            {
              theme: { light: 'github-light', dark: 'github-dark-dimmed' },
              keepBackground: false,
              defaultLang: 'plaintext',
              getHighlighter: (options: any) =>
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
          ],
        ],
      },
    },
  })

  const headings = extractHeadings(raw)
  const titleMatch = raw.match(/^#\s+(.+)$/m)
  const title = titleMatch ? titleMatch[1].trim() : undefined

  return { content, headings, title }
}

function extractHeadings(source: string): Heading[] {
  const headings: Heading[] = []
  const lines = source.split('\n')
  let inCodeBlock = false

  for (const line of lines) {
    if (line.trim().startsWith('```')) {
      inCodeBlock = !inCodeBlock
      continue
    }
    if (inCodeBlock) continue

    const match = line.match(/^(#{2,4})\s+(.+)$/)
    if (match) {
      const level = match[1].length
      const text = match[2].trim()
      const id = text
        .toLowerCase()
        .replace(/[^a-z0-9\s-]/g, '')
        .replace(/\s+/g, '-')
        .replace(/-+/g, '-')
        .replace(/^-|-$/g, '')
      headings.push({ id, text, level })
    }
  }

  return headings
}

export function getAllSlugs(): string[][] {
  const slugs: string[][] = [[]]

  function walk(dir: string, prefix: string[] = []) {
    const entries = fs.readdirSync(dir, { withFileTypes: true })
    for (const entry of entries) {
      if (entry.isFile() && entry.name.endsWith('.mdx') && entry.name !== 'index.mdx') {
        slugs.push([...prefix, entry.name.replace('.mdx', '')])
      } else if (entry.isDirectory() && !entry.name.startsWith('_') && !entry.name.startsWith('.')) {
        walk(path.join(dir, entry.name), [...prefix, entry.name])
      }
    }
  }

  walk(contentDir)
  return slugs
}
