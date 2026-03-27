'use client'

import { useEffect, useState } from 'react'
import { cn } from '@/lib/utils'
import type { Heading } from '@/lib/mdx'

export function TableOfContents({ headings }: { headings: Heading[] }) {
  const [activeId, setActiveId] = useState<string>('')

  useEffect(() => {
    const elements = headings
      .map((h) => document.getElementById(h.id))
      .filter(Boolean) as HTMLElement[]

    if (elements.length === 0) return

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) setActiveId(entry.target.id)
        }
      },
      { rootMargin: '-120px 0px -80% 0px' }
    )

    elements.forEach((el) => observer.observe(el))
    return () => observer.disconnect()
  }, [headings])

  if (headings.length === 0) return null

  return (
    <aside className="hidden xl:block w-52 shrink-0">
      <div className="sticky top-[6.5rem] h-[calc(100vh-6.5rem)] overflow-y-auto py-8 pr-4">
        <p className="text-[0.625rem] font-semibold uppercase tracking-[0.1em] text-muted-foreground/60 mb-3">
          On this page
        </p>
        <ul className="space-y-0 border-l border-border/50">
          {headings.map((heading) => (
            <li key={heading.id}>
              <a
                href={`#${heading.id}`}
                className={cn(
                  'block py-1 text-[0.8125rem] leading-snug transition-colors duration-200 -ml-px border-l',
                  heading.level === 2 && 'pl-3',
                  heading.level === 3 && 'pl-5',
                  heading.level === 4 && 'pl-7',
                  activeId === heading.id
                    ? 'border-primary text-primary font-medium'
                    : 'border-transparent text-muted-foreground/70 hover:text-foreground hover:border-border'
                )}
              >
                {heading.text}
              </a>
            </li>
          ))}
        </ul>
      </div>
    </aside>
  )
}
