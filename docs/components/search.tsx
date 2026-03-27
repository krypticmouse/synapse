'use client'

import { useCallback, useEffect, useRef, useState } from 'react'
import { useRouter } from 'next/navigation'
import { createPortal } from 'react-dom'
import lunr from 'lunr'
import { cn } from '@/lib/utils'

interface DocEntry {
  title: string
  section: string
  excerpt: string
}

interface SearchIndex {
  index: object
  store: Record<string, DocEntry>
}

let cachedData: { idx: lunr.Index; store: Record<string, DocEntry> } | null = null

async function loadIndex(): Promise<typeof cachedData> {
  if (cachedData) return cachedData
  const res = await fetch('/search-index.json')
  const data: SearchIndex = await res.json()
  cachedData = {
    idx: lunr.Index.load(data.index),
    store: data.store,
  }
  return cachedData
}

export function SearchButton() {
  const [open, setOpen] = useState(false)
  const [mounted, setMounted] = useState(false)

  useEffect(() => setMounted(true), [])

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setOpen((o) => !o)
      }
    }
    document.addEventListener('keydown', down)
    return () => document.removeEventListener('keydown', down)
  }, [])

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex w-full min-h-9 items-center gap-3 rounded-lg border border-border/60 bg-muted/50 px-4 py-2 text-sm text-muted-foreground transition-colors hover:border-border/80 hover:bg-muted/70 hover:text-foreground"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="shrink-0 opacity-50">
          <circle cx="11" cy="11" r="8" />
          <path d="M21 21l-4.35-4.35" />
        </svg>
        <span className="min-w-0 flex-1 truncate text-left">Search docs...</span>
        <kbd className="hidden shrink-0 sm:inline-flex items-center gap-0.5 rounded-md border border-border/50 bg-background/90 px-2 py-0.5 text-[0.625rem] font-mono text-muted-foreground/45">
          <span className="text-xs">⌘</span>K
        </kbd>
      </button>

      {mounted && open && createPortal(
        <SearchModal onClose={() => setOpen(false)} />,
        document.body
      )}
    </>
  )
}

function SearchModal({ onClose }: { onClose: () => void }) {
  const router = useRouter()
  const inputRef = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<{ slug: string; doc: DocEntry }[]>([])
  const [selected, setSelected] = useState(0)
  const [loading, setLoading] = useState(true)
  const dataRef = useRef<{ idx: lunr.Index; store: Record<string, DocEntry> } | null>(null)

  useEffect(() => {
    loadIndex().then((d) => {
      dataRef.current = d
      setLoading(false)
    })
    requestAnimationFrame(() => inputRef.current?.focus())
  }, [])

  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onEsc)
    return () => document.removeEventListener('keydown', onEsc)
  }, [onClose])

  useEffect(() => {
    document.body.style.overflow = 'hidden'
    return () => { document.body.style.overflow = '' }
  }, [])

  useEffect(() => {
    if (!dataRef.current || !query.trim()) {
      setResults([])
      setSelected(0)
      return
    }

    try {
      const raw = dataRef.current.idx.search(
        query
          .split(/\s+/)
          .map((t) => `${t}* ${t}~1`)
          .join(' ')
      )
      const hits = raw.slice(0, 12).map((r) => ({
        slug: r.ref,
        doc: dataRef.current!.store[r.ref],
      })).filter((h) => h.doc)
      setResults(hits)
      setSelected(0)
    } catch {
      setResults([])
    }
  }, [query])

  const navigate = useCallback(
    (slug: string) => {
      onClose()
      router.push(slug)
    },
    [router, onClose]
  )

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelected((s) => Math.min(s + 1, results.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelected((s) => Math.max(s - 1, 0))
    } else if (e.key === 'Enter' && results[selected]) {
      e.preventDefault()
      navigate(results[selected].slug)
    }
  }

  return (
    <div className="fixed inset-0 z-[100]" onClick={onClose}>
      <div className="fixed inset-0 bg-background/70 backdrop-blur-md" />

      <div className="fixed inset-x-0 top-[12vh] mx-auto w-full max-w-2xl px-4" onClick={(e) => e.stopPropagation()}>
        <div className="overflow-hidden rounded-2xl border border-border/50 bg-background shadow-2xl shadow-black/10 dark:shadow-black/30">
          {/* Input */}
          <div className="flex items-center gap-3 border-b border-border/40 px-5">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="shrink-0 text-muted-foreground/60">
              <circle cx="11" cy="11" r="8" />
              <path d="M21 21l-4.35-4.35" />
            </svg>
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKey}
              placeholder="Search documentation..."
              className="flex-1 bg-transparent py-4 text-[0.9375rem] text-foreground placeholder:text-muted-foreground/40 outline-none"
              autoComplete="off"
              spellCheck={false}
            />
            <kbd className="rounded-md border border-border/40 px-2 py-1 text-[0.6875rem] font-mono text-muted-foreground/30">
              ESC
            </kbd>
          </div>

          {/* Results */}
          <div className="max-h-[55vh] overflow-y-auto">
            {loading && (
              <div className="px-5 py-10 text-center text-sm text-muted-foreground/60">
                Loading index...
              </div>
            )}

            {!loading && query.trim() && results.length === 0 && (
              <div className="px-5 py-10 text-center text-sm text-muted-foreground/60">
                No results for &ldquo;{query}&rdquo;
              </div>
            )}

            {!loading && !query.trim() && (
              <div className="px-5 py-10 text-center text-sm text-muted-foreground/30">
                Type to search across all pages
              </div>
            )}

            {results.length > 0 && (
              <ul className="py-2">
                {results.map((r, i) => (
                  <li key={r.slug}>
                    <button
                      onClick={() => navigate(r.slug)}
                      onMouseEnter={() => setSelected(i)}
                      className={cn(
                        'flex w-full items-start gap-3 px-5 py-3 text-left transition-colors',
                        i === selected
                          ? 'bg-primary/[0.06]'
                          : 'hover:bg-accent/40'
                      )}
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className={cn('shrink-0 mt-0.5', i === selected ? 'text-primary' : 'text-muted-foreground/30')}>
                        <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
                        <polyline points="14 2 14 8 20 8" />
                      </svg>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className={cn(
                            'text-sm font-medium truncate',
                            i === selected ? 'text-primary' : 'text-foreground'
                          )}>
                            {r.doc.title}
                          </span>
                          <span className="shrink-0 text-[0.6rem] text-muted-foreground/40 uppercase tracking-widest">
                            {r.doc.section}
                          </span>
                        </div>
                        <p className="mt-0.5 text-[0.8125rem] text-muted-foreground/50 line-clamp-1">
                          {r.doc.excerpt}
                        </p>
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Footer */}
          {results.length > 0 && (
            <div className="flex items-center gap-5 border-t border-border/30 px-5 py-2.5 text-[0.6875rem] text-muted-foreground/30">
              <span className="inline-flex items-center gap-1">
                <kbd className="rounded border border-border/30 px-1.5 py-0.5 font-mono">↑</kbd>
                <kbd className="rounded border border-border/30 px-1.5 py-0.5 font-mono">↓</kbd>
                navigate
              </span>
              <span className="inline-flex items-center gap-1">
                <kbd className="rounded border border-border/30 px-1.5 py-0.5 font-mono">↵</kbd>
                open
              </span>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
