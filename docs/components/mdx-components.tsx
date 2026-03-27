import React from 'react'
import Link from 'next/link'
import { cn } from '@/lib/utils'
import { CodeBlock } from './code-block'

function Callout({
  type = 'info',
  children,
}: {
  type?: 'info' | 'warning' | 'error'
  children: React.ReactNode
}) {
  const icons: Record<string, React.ReactNode> = {
    info: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="shrink-0 mt-0.5">
        <circle cx="12" cy="12" r="10" /><path d="M12 16v-4M12 8h.01" />
      </svg>
    ),
    warning: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="shrink-0 mt-0.5">
        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01" />
      </svg>
    ),
    error: (
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="shrink-0 mt-0.5">
        <circle cx="12" cy="12" r="10" /><path d="M15 9l-6 6M9 9l6 6" />
      </svg>
    ),
  }

  return (
    <div
      className={cn(
        'my-6 flex gap-3 rounded-xl border px-4 py-3.5 text-sm leading-relaxed [&>div>p]:m-0',
        type === 'info' && 'border-sky-200/50 bg-sky-50/40 text-sky-900 dark:border-sky-500/15 dark:bg-sky-950/20 dark:text-sky-200',
        type === 'warning' && 'border-amber-200/50 bg-amber-50/40 text-amber-900 dark:border-amber-500/15 dark:bg-amber-950/20 dark:text-amber-200',
        type === 'error' && 'border-red-200/50 bg-red-50/40 text-red-900 dark:border-red-500/15 dark:bg-red-950/20 dark:text-red-200'
      )}
    >
      {icons[type]}
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  )
}

function Card({
  title,
  href,
  children,
}: {
  title: string
  href: string
  children: React.ReactNode
}) {
  return (
    <Link
      href={href}
      className="group relative flex flex-col rounded-xl border border-border/70 bg-surface/50 p-5 no-underline transition-all duration-300 hover:border-primary/20 hover:bg-surface hover:shadow-lg hover:shadow-primary/[0.03] hover:-translate-y-0.5"
    >
      <h3 className="!mt-0 !mb-1 text-[0.9375rem] font-semibold tracking-tight text-foreground group-hover:text-primary transition-colors duration-300">
        {title}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="inline-block ml-1 opacity-0 -translate-x-1 group-hover:opacity-60 group-hover:translate-x-0 transition-all duration-300">
          <path d="M5 12h14M12 5l7 7-7 7" />
        </svg>
      </h3>
      <p className="!m-0 text-[0.8125rem] text-muted-foreground leading-relaxed">{children}</p>
    </Link>
  )
}

function CardsContainer({ children }: { children: React.ReactNode }) {
  return (
    <div className="not-prose grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 my-8">{children}</div>
  )
}

const Cards = Object.assign(CardsContainer, { Card })

export const mdxComponents: Record<string, React.ComponentType<any>> = {
  Cards,
  Callout,
  pre: CodeBlock,
  a: ({
    href,
    children,
    ...props
  }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => {
    if (href?.startsWith('/') || href?.startsWith('#')) {
      return (
        <Link href={href} {...(props as any)}>
          {children}
        </Link>
      )
    }
    return (
      <a href={href} target="_blank" rel="noopener noreferrer" {...props}>
        {children}
      </a>
    )
  },
}
