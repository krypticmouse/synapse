'use client'

import { useRef, useState } from 'react'

export function CodeBlock(props: React.HTMLAttributes<HTMLPreElement>) {
  const preRef = useRef<HTMLPreElement>(null)
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    const text = preRef.current?.textContent ?? ''
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="relative group/code">
      <pre ref={preRef} {...props} />
      <button
        onClick={handleCopy}
        className="absolute top-2.5 right-2.5 z-10 flex items-center gap-1 rounded-lg px-2 py-1 text-[0.6875rem] opacity-0 group-hover/code:opacity-100 transition-all duration-200 bg-background/90 backdrop-blur-sm border border-border/60 text-muted-foreground hover:text-foreground hover:border-border"
        aria-label={copied ? 'Copied' : 'Copy code'}
      >
        {copied ? (
          <>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-emerald-500">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            <span className="text-emerald-500">Copied</span>
          </>
        ) : (
          <>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
            </svg>
            <span>Copy</span>
          </>
        )}
      </button>
    </div>
  )
}
