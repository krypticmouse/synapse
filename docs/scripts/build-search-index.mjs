#!/usr/bin/env node
/**
 * Reads all .mdx files under content/, strips markdown/JSX syntax,
 * builds a lunr index + document store, and writes them to public/search-index.json.
 * Run as part of the build: `node scripts/build-search-index.mjs && next build`
 */

import fs from 'fs'
import path from 'path'
import lunr from 'lunr'

const CONTENT_DIR = path.join(process.cwd(), 'content')
const OUT_FILE = path.join(process.cwd(), 'public', 'search-index.json')

function stripMdx(raw) {
  return raw
    .replace(/^---[\s\S]*?---/m, '')                     // frontmatter
    .replace(/^import\s.+$/gm, '')                       // import statements
    .replace(/<[^>]+>/g, '')                              // JSX / HTML tags
    .replace(/```[\s\S]*?```/g, '')                       // fenced code blocks
    .replace(/`[^`]+`/g, (m) => m.slice(1, -1))          // inline code → plain text
    .replace(/!\[[^\]]*\]\([^)]*\)/g, '')                 // images
    .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')              // links → text
    .replace(/^#{1,6}\s+/gm, '')                          // heading markers
    .replace(/[*_~]{1,3}/g, '')                           // bold/italic/strikethrough
    .replace(/^\s*[-*+]\s+/gm, '')                        // unordered list markers
    .replace(/^\s*\d+\.\s+/gm, '')                        // ordered list markers
    .replace(/^\|.*\|$/gm, '')                            // table rows
    .replace(/^[-|:\s]+$/gm, '')                          // table separators
    .replace(/^>\s?/gm, '')                               // blockquote markers
    .replace(/\n{3,}/g, '\n\n')                           // collapse whitespace
    .trim()
}

function fileToSlug(filePath) {
  const rel = path.relative(CONTENT_DIR, filePath).replace(/\.mdx$/, '')
  if (rel === 'index') return '/'
  return '/' + rel
}

function extractTitle(raw) {
  const m = raw.match(/^#\s+(.+)$/m)
  return m ? m[1].trim() : undefined
}

function extractSection(slug) {
  const parts = slug.split('/').filter(Boolean)
  if (parts.length === 0) return 'Home'
  const sectionMap = {
    language: 'Language Guide',
    tutorials: 'Tutorials',
    examples: 'Examples',
    reference: 'Reference',
  }
  return sectionMap[parts[0]] || parts[0]
}

function collectMdxFiles(dir, files = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory() && !entry.name.startsWith('_') && !entry.name.startsWith('.')) {
      collectMdxFiles(full, files)
    } else if (entry.isFile() && entry.name.endsWith('.mdx')) {
      files.push(full)
    }
  }
  return files
}

const files = collectMdxFiles(CONTENT_DIR)
const docs = []

for (const file of files) {
  const raw = fs.readFileSync(file, 'utf-8')
  const slug = fileToSlug(file)
  const title = extractTitle(raw) || slug
  const body = stripMdx(raw)
  const section = extractSection(slug)

  docs.push({ slug, title, body, section })
}

const idx = lunr(function () {
  this.ref('slug')
  this.field('title', { boost: 10 })
  this.field('body')

  for (const doc of docs) {
    this.add(doc)
  }
})

const store = {}
for (const doc of docs) {
  store[doc.slug] = {
    title: doc.title,
    section: doc.section,
    excerpt: doc.body.slice(0, 200).replace(/\n/g, ' '),
  }
}

fs.mkdirSync(path.dirname(OUT_FILE), { recursive: true })
fs.writeFileSync(OUT_FILE, JSON.stringify({ index: idx.toJSON(), store }))

console.log(`Search index: ${docs.length} documents → ${OUT_FILE}`)
