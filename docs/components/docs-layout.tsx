'use client'

import { useState, useEffect } from 'react'
import { usePathname } from 'next/navigation'
import Link from 'next/link'
import { useTheme } from 'next-themes'
import { tabs, type HeaderTab } from '@/lib/navigation'
import { cn } from '@/lib/utils'
import { GitHubHeaderLink } from '@/components/github-header-link'

function getActiveTab(pathname: string): HeaderTab | undefined {
  if (pathname === '/') return tabs.find((t) => t.prefix === '/')
  return tabs.find((t) => t.prefix !== '/' && pathname.startsWith(t.prefix))
}

export function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const activeTab = getActiveTab(pathname)
  const sidebarItems = activeTab?.items

  useEffect(() => setMobileMenuOpen(false), [pathname])

  useEffect(() => {
    document.body.style.overflow = mobileMenuOpen ? 'hidden' : ''
    return () => { document.body.style.overflow = '' }
  }, [mobileMenuOpen])

  return (
    <>
      {/* ── Header ─────────────────────────────────── */}
      <header className="sticky top-0 z-40 bg-background/70 backdrop-blur-2xl backdrop-saturate-150">
        {/* Row 1: Brand + actions */}
        <div className="border-b border-border/40">
          <div className="mx-auto flex h-14 max-w-[88rem] items-center gap-4 px-5 sm:px-6">
            <button
              onClick={() => setMobileMenuOpen(true)}
              className="inline-flex items-center justify-center rounded-lg p-1.5 text-muted-foreground hover:text-foreground lg:hidden"
              aria-label="Open menu"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M4 7h16M4 12h16M4 17h16" />
              </svg>
            </button>

            <Link href="/" className="flex items-center gap-2.5 group shrink-0">
              <img src="/synapse_logo.png" alt="Synapse" className="h-6" />
            </Link>

            <div className="flex-1" />

            <nav className="flex items-center gap-2">
              <GitHubHeaderLink />
              <ThemeToggle />
            </nav>
          </div>
        </div>

        {/* Row 2: Navigation tabs */}
        <div className="border-b border-border/60">
          <div className="mx-auto max-w-[88rem] px-5 sm:px-6">
            <nav className="hidden md:flex items-center gap-0.5 -mb-px" aria-label="Main navigation">
              {tabs.map((tab) => {
                const isActive =
                  tab.prefix === '/'
                    ? pathname === '/'
                    : pathname.startsWith(tab.prefix)

                return (
                  <Link
                    key={tab.prefix}
                    href={tab.href}
                    className={cn(
                      'relative px-3 py-2.5 text-[0.8125rem] font-medium whitespace-nowrap transition-colors',
                      isActive
                        ? 'text-foreground'
                        : 'text-muted-foreground hover:text-foreground'
                    )}
                  >
                    {tab.title}
                    {isActive && (
                      <span className="absolute inset-x-3 -bottom-px h-[2px] rounded-full bg-primary" />
                    )}
                  </Link>
                )
              })}
            </nav>

            {/* Mobile: show current tab name */}
            <div className="flex md:hidden items-center h-10 text-sm font-medium text-muted-foreground">
              {activeTab?.title ?? 'Home'}
            </div>
          </div>
        </div>
      </header>

      {/* ── Mobile drawer ──────────────────────────── */}
      {mobileMenuOpen && (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div className="fixed inset-0 bg-background/60 backdrop-blur-md" onClick={() => setMobileMenuOpen(false)} />
          <div className="fixed left-0 top-0 bottom-0 w-72 border-r border-border/50 bg-background overflow-y-auto shadow-2xl" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 pb-2">
              <Link href="/" className="flex items-center gap-2">
                <img src="/synapse_logo.png" alt="Synapse" className="h-6" />
                <span className="text-sm font-semibold">Synapse</span>
              </Link>
              <button onClick={() => setMobileMenuOpen(false)} className="rounded-lg p-1.5 text-muted-foreground hover:text-foreground" aria-label="Close">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M18 6L6 18M6 6l12 12" /></svg>
              </button>
            </div>

            {/* Mobile tabs */}
            <div className="px-3 pt-2 pb-3 border-b border-border/40">
              {tabs.map((tab) => {
                const isActive =
                  tab.prefix === '/'
                    ? pathname === '/'
                    : pathname.startsWith(tab.prefix)

                return (
                  <Link
                    key={tab.prefix}
                    href={tab.href}
                    className={cn(
                      'block px-2.5 py-2 text-sm rounded-lg transition-colors',
                      isActive
                        ? 'text-primary font-medium bg-primary/[0.06]'
                        : 'text-muted-foreground hover:text-foreground'
                    )}
                  >
                    {tab.title}
                  </Link>
                )
              })}
            </div>

            {/* Mobile sidebar items for active section */}
            {sidebarItems && sidebarItems.length > 0 && (
              <div className="px-3 pt-4 pb-6">
                <p className="px-2.5 mb-2 text-[0.625rem] font-semibold uppercase tracking-[0.1em] text-muted-foreground/50">
                  {activeTab?.title}
                </p>
                <ul className="space-y-px ml-2.5 border-l border-border/50">
                  {sidebarItems.map((item) => (
                    <li key={item.href}>
                      <Link
                        href={item.href}
                        className={cn(
                          'block py-1.5 pl-3 pr-2 text-[0.8125rem] -ml-px border-l transition-colors',
                          pathname === item.href
                            ? 'border-primary text-primary font-medium'
                            : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'
                        )}
                      >
                        {item.title}
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      )}

      {/* ── Body ───────────────────────────────────── */}
      <div className="mx-auto flex max-w-[88rem]">
        {sidebarItems && sidebarItems.length > 0 && (
          <aside className="hidden lg:block w-56 shrink-0 border-r border-border/40">
            <div className="sticky top-[6.5rem] h-[calc(100vh-6.5rem)] overflow-y-auto py-6 pl-5 pr-3">
              <SidebarNav items={sidebarItems} pathname={pathname} />
            </div>
          </aside>
        )}
        <div className="flex-1 min-w-0">{children}</div>
      </div>
    </>
  )
}

function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme()
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])
  if (!mounted) return <div className="w-8 h-8" />

  return (
    <button
      onClick={() => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')}
      className="inline-flex items-center justify-center rounded-lg w-8 h-8 text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
      aria-label="Toggle theme"
    >
      {resolvedTheme === 'dark' ? (
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
        </svg>
      ) : (
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      )}
    </button>
  )
}

function SidebarNav({ items, pathname }: { items: { title: string; href: string }[]; pathname: string }) {
  return (
    <nav>
      <ul className="space-y-px border-l border-border/50">
        {items.map((item) => (
          <li key={item.href}>
            <Link
              href={item.href}
              className={cn(
                'block py-[0.375rem] pl-3.5 pr-2 text-[0.8125rem] -ml-px border-l transition-colors',
                pathname === item.href
                  ? 'border-primary text-primary font-medium'
                  : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'
              )}
            >
              {item.title}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  )
}
