'use client'

import { useEffect, useState } from 'react'
import { GITHUB_REPO, GITHUB_REPO_URL } from '@/lib/site'

function formatCount(n: number): string {
  if (n >= 1_000_000) return `${Math.round(n / 100_000) / 10}M`
  if (n >= 10_000) return `${Math.round(n / 1000)}k`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
}

const repoShortName = GITHUB_REPO.split('/')[1] ?? GITHUB_REPO

export function GitHubHeaderLink() {
  const [stats, setStats] = useState<{ stars: number; forks: number } | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    fetch(`https://api.github.com/repos/${GITHUB_REPO}`, {
      headers: { Accept: 'application/vnd.github+json' },
    })
      .then((res) => (res.ok ? res.json() : Promise.reject()))
      .then((data: { stargazers_count: number; forks_count: number }) => {
        if (!cancelled) {
          setStats({
            stars: data.stargazers_count,
            forks: data.forks_count,
          })
        }
      })
      .catch(() => {
        if (!cancelled) setStats(null)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const label =
    stats != null
      ? `${GITHUB_REPO} on GitHub — ${formatCount(stats.stars)} stars, ${formatCount(stats.forks)} forks`
      : `${GITHUB_REPO} on GitHub`

  return (
    <a
      href={GITHUB_REPO_URL}
      target="_blank"
      rel="noopener noreferrer"
      aria-label={label}
      className="group flex max-w-[min(100%,11rem)] items-center gap-2 rounded-xl border border-border bg-muted/50 px-2 py-1 text-left shadow-sm transition-colors hover:border-primary/30 hover:bg-accent sm:max-w-none sm:gap-2.5 sm:px-2.5"
    >
      <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-background/90 text-foreground ring-1 ring-border/80 group-hover:ring-primary/25">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
          <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
        </svg>
      </span>
      <span className="min-w-0 flex-1 leading-tight">
        <span className="block truncate text-[0.8125rem] font-semibold tracking-tight text-foreground group-hover:text-primary transition-colors">
          {repoShortName}
        </span>
        <span className="mt-0.5 flex items-center gap-2 text-[0.625rem] tabular-nums text-muted-foreground">
          {loading ? (
            <>
              <span className="inline-flex h-2.5 w-8 animate-pulse rounded-sm bg-muted-foreground/20" />
              <span className="inline-flex h-2.5 w-8 animate-pulse rounded-sm bg-muted-foreground/20" />
            </>
          ) : stats != null ? (
            <>
              <span className="inline-flex items-center gap-0.5" title="Stars">
                <StarIcon className="h-2.5 w-2.5 text-primary" />
                {formatCount(stats.stars)}
              </span>
              <span className="inline-flex items-center gap-0.5" title="Forks">
                <ForkIcon className="h-2.5 w-2.5 opacity-80" />
                {formatCount(stats.forks)}
              </span>
            </>
          ) : (
            <span className="text-muted-foreground/80">Open</span>
          )}
        </span>
      </span>
    </a>
  )
}

function StarIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z" />
    </svg>
  )
}

function ForkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <circle cx="12" cy="18" r="2" />
      <circle cx="6" cy="6" r="2" />
      <circle cx="18" cy="6" r="2" />
      <path d="M6 8v1a4 4 0 0 0 4 4h4a4 4 0 0 0 4-4V8M12 12v4" strokeLinecap="round" />
    </svg>
  )
}
