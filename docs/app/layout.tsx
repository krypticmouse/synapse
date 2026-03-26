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
        <img
          src="/synapse_logo.png"
          alt="Synapse"
          height={36}
          style={{ height: 36 }}
        />
      }
      projectLink="https://github.com/krypticmouse/synapse"
    />
  )

  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head
        color={{
          hue: { dark: 270, light: 270 },
          saturation: { dark: 80, light: 80 }
        }}
      >
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
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
              MIT {new Date().getFullYear()} © Synapse. Configuration Language
              for Memory Systems.
            </Footer>
          }
        >
          {children}
        </Layout>
      </body>
    </html>
  )
}
