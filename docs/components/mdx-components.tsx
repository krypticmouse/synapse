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
  return (
    <div
      className={cn(
        'my-6 rounded-xl border px-5 py-4 text-sm leading-relaxed [&>p]:m-0',
        type === 'info' &&
          'border-blue-200/60 bg-blue-50/60 text-blue-900 dark:border-blue-500/20 dark:bg-blue-950/30 dark:text-blue-200',
        type === 'warning' &&
          'border-amber-200/60 bg-amber-50/60 text-amber-900 dark:border-amber-500/20 dark:bg-amber-950/30 dark:text-amber-200',
        type === 'error' &&
          'border-red-200/60 bg-red-50/60 text-red-900 dark:border-red-500/20 dark:bg-red-950/30 dark:text-red-200'
      )}
    >
      {children}
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
      className="group block rounded-xl border border-border p-5 no-underline transition-all hover:border-primary/30 hover:shadow-md hover:shadow-primary/5"
    >
      <h3 className="!mt-0 !mb-1.5 text-base font-semibold group-hover:text-primary transition-colors">
        {title}
      </h3>
      <p className="!m-0 text-sm text-muted-foreground leading-relaxed">{children}</p>
    </Link>
  )
}

function CardsContainer({ children }: { children: React.ReactNode }) {
  return (
    <div className="not-prose grid grid-cols-1 gap-3 sm:grid-cols-2 my-6">{children}</div>
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
