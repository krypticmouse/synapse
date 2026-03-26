'use client'

import { useState, useEffect } from 'react'
import { usePathname } from 'next/navigation'
import Link from 'next/link'
import { useTheme } from 'next-themes'
import { navigation, type NavItem, type NavSection } from '@/lib/navigation'
import { cn } from '@/lib/utils'

export function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  useEffect(() => {
    setMobileMenuOpen(false)
  }, [pathname])

  useEffect(() => {
    document.body.style.overflow = mobileMenuOpen ? 'hidden' : ''
    return () => {
      document.body.style.overflow = ''
    }
  }, [mobileMenuOpen])

  return (
    <>
      <header className="sticky top-0 z-40 h-14 border-b border-border bg-background/80 backdrop-blur-xl">
        <div className="mx-auto flex h-14 max-w-[90rem] items-center px-6">
          <button
            onClick={() => setMobileMenuOpen(true)}
            className="mr-3 inline-flex items-center justify-center rounded-md p-1.5 text-muted-foreground hover:text-foreground lg:hidden"
            aria-label="Open menu"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            >
              <path d="M3 12h18M3 6h18M3 18h18" />
            </svg>
          </button>

          <Link href="/" className="flex items-center gap-2.5 group">
            <img src="/synapse_logo.png" alt="Synapse" className="h-7" />
          </Link>

          <div className="flex-1" />

          <nav className="flex items-center gap-0.5">
            <a
              href="https://github.com/krypticmouse/synapse"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center justify-center rounded-lg p-2 text-muted-foreground hover:text-foreground hover:bg-accent"
              aria-label="GitHub"
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
              </svg>
            </a>
            <ThemeToggle />
          </nav>
        </div>
      </header>

      {mobileMenuOpen && (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div
            className="fixed inset-0 bg-background/80 backdrop-blur-sm"
            onClick={() => setMobileMenuOpen(false)}
          />
          <div
            className="fixed left-0 top-0 bottom-0 w-72 border-r border-border bg-background p-6 overflow-y-auto shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between mb-8">
              <Link href="/" className="flex items-center gap-2">
                <img src="/synapse_logo.png" alt="Synapse" className="h-6" />
                <span className="font-semibold text-sm">Synapse</span>
              </Link>
              <button
                onClick={() => setMobileMenuOpen(false)}
                className="rounded-lg p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent"
                aria-label="Close menu"
              >
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                >
                  <path d="M18 6L6 18M6 6l12 12" />
                </svg>
              </button>
            </div>
            <SidebarNav pathname={pathname} />
          </div>
        </div>
      )}

      <div className="mx-auto flex max-w-[90rem]">
        <aside className="hidden lg:block w-64 shrink-0">
          <div className="sticky top-14 h-[calc(100vh-3.5rem)] overflow-y-auto py-6 pl-6 pr-3">
            <SidebarNav pathname={pathname} />
          </div>
        </aside>
        <div className="flex-1 min-w-0">{children}</div>
      </div>
    </>
  )
}

function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)

  useEffect(() => setMounted(true), [])

  if (!mounted) return <div className="w-9 h-9" />

  return (
    <button
      onClick={() => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')}
      className="inline-flex items-center justify-center rounded-lg p-2 text-muted-foreground hover:text-foreground hover:bg-accent"
      aria-label="Toggle theme"
    >
      {resolvedTheme === 'dark' ? (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="5" />
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
        </svg>
      ) : (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      )}
    </button>
  )
}

function SidebarNav({ pathname }: { pathname: string }) {
  const [expanded, setExpanded] = useState<Record<string, boolean>>(() => {
    const initial: Record<string, boolean> = {}
    for (const item of navigation) {
      if ('items' in item) {
        const section = item as NavSection
        initial[section.title] = section.items.some((c) => pathname === c.href)
      }
    }
    return initial
  })

  useEffect(() => {
    for (const item of navigation) {
      if ('items' in item) {
        const section = item as NavSection
        if (section.items.some((c) => pathname === c.href)) {
          setExpanded((prev) => ({ ...prev, [section.title]: true }))
        }
      }
    }
  }, [pathname])

  const toggle = (title: string) =>
    setExpanded((prev) => ({ ...prev, [title]: !prev[title] }))

  return (
    <nav className="space-y-1">
      {navigation.map((item) => {
        if ('items' in item) {
          const section = item as NavSection
          const isExpanded = expanded[section.title] ?? false
          const hasActive = section.items.some((c) => pathname === c.href)

          return (
            <div key={section.title} className="pt-4 first:pt-0">
              <button
                onClick={() => toggle(section.title)}
                className={cn(
                  'flex w-full items-center justify-between rounded-lg px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors',
                  hasActive
                    ? 'text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {section.title}
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className={cn(
                    'transition-transform duration-200',
                    isExpanded ? 'rotate-90' : 'rotate-0'
                  )}
                >
                  <path d="M9 18l6-6-6-6" />
                </svg>
              </button>
              <div
                className={cn(
                  'overflow-hidden transition-all duration-200 ease-out',
                  isExpanded ? 'max-h-[600px] opacity-100' : 'max-h-0 opacity-0'
                )}
              >
                <ul className="space-y-0.5 pt-1 pb-1">
                  {section.items.map((child) => (
                    <li key={child.href}>
                      <Link
                        href={child.href}
                        className={cn(
                          'block py-1.5 pl-4 pr-3 text-[0.8125rem] rounded-lg transition-colors border-l-2',
                          pathname === child.href
                            ? 'border-primary text-foreground font-medium bg-primary/[0.06]'
                            : 'border-transparent text-muted-foreground hover:text-foreground hover:bg-accent'
                        )}
                      >
                        {child.title}
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )
        }

        const link = item as NavItem
        return (
          <Link
            key={link.href}
            href={link.href}
            className={cn(
              'block py-1.5 px-3 text-[0.8125rem] rounded-lg transition-colors',
              pathname === link.href
                ? 'text-primary font-medium bg-primary/[0.06]'
                : 'text-muted-foreground hover:text-foreground hover:bg-accent'
            )}
          >
            {link.title}
          </Link>
        )
      })}
    </nav>
  )
}
