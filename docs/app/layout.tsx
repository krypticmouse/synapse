import { Footer, Layout, Navbar } from 'nextra-theme-docs'
import { Head } from 'nextra/components'
import { getPageMap } from 'nextra/page-map'
import 'nextra-theme-docs/style.css'
import './globals.css'

export const metadata = {
  title: {
    template: '%s | Synapse',
    default: 'Synapse Documentation'
  },
  description: 'Synapse: Configuration Language for Memory Systems',
  openGraph: {
    title: 'Synapse Documentation'
  }
}

export default async function RootLayout({
  children
}: {
  children: React.ReactNode
}) {
  const navbar = (
    <Navbar
      logo={
        <span style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <img
            src="/synapse_logo.png"
            alt="Synapse"
            height={30}
            style={{ height: 30 }}
          />
        </span>
      }
      projectLink="https://github.com/krypticmouse/synapse"
    />
  )

  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head
        color={{
          hue: { dark: 250, light: 250 },
          saturation: { dark: 65, light: 50 }
        }}
      >
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      </Head>
      <body>
        <Layout
          navbar={navbar}
          pageMap={await getPageMap()}
          docsRepositoryBase="https://github.com/krypticmouse/synapse/tree/master/docs"
          editLink="Edit this page on GitHub"
          feedback={{ content: 'Question? Give us feedback →', labels: 'feedback' }}
          sidebar={{ defaultMenuCollapseLevel: 1, toggleButton: true }}
          toc={{ backToTop: true }}
          footer={
            <Footer>
              <span style={{ fontFamily: "'Manrope', sans-serif", fontWeight: 600 }}>Synapse</span>
              {' '}&mdash; Configuration Language for Memory Systems. MIT {new Date().getFullYear()}.
            </Footer>
          }
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
