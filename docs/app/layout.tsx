import type { Metadata } from 'next'
import { DM_Sans, JetBrains_Mono } from 'next/font/google'
import { ThemeProvider } from '@/components/theme-provider'
import { DocsLayout } from '@/components/docs-layout'
import { getMetadataBase, OG_IMAGE_PATH, SITE_DESCRIPTION } from '@/lib/site'
import './globals.css'

const dmSans = DM_Sans({
  subsets: ['latin'],
  variable: '--font-dm-sans',
  display: 'swap',
})

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-jetbrains-mono',
  display: 'swap',
})

export const metadata: Metadata = {
  metadataBase: getMetadataBase(),
  title: {
    template: '%s | Synapse',
    default: 'Synapse Documentation',
  },
  description: SITE_DESCRIPTION,
  icons: {
    icon: '/logo.png',
    apple: '/logo.png',
  },
  openGraph: {
    type: 'website',
    siteName: 'Synapse',
    title: 'Synapse Documentation',
    description: SITE_DESCRIPTION,
    locale: 'en_US',
    images: [
      {
        url: OG_IMAGE_PATH,
        alt: 'Synapse',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Synapse Documentation',
    description: SITE_DESCRIPTION,
    images: [OG_IMAGE_PATH],
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${dmSans.variable} ${jetbrainsMono.variable}`}
    >
      <body className="min-h-screen bg-background font-sans antialiased">
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <DocsLayout>{children}</DocsLayout>
        </ThemeProvider>
      </body>
    </html>
  )
}
