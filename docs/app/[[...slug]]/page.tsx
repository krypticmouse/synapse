import type { Metadata } from 'next'
import { getPage, getAllSlugs } from '@/lib/mdx'
import { TableOfContents } from '@/components/toc'
import { OG_IMAGE_PATH, SITE_DESCRIPTION } from '@/lib/site'
import { notFound } from 'next/navigation'

export function generateStaticParams() {
  return getAllSlugs().map((slug) => ({
    slug: slug.length === 0 ? undefined : slug,
  }))
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>
}): Promise<Metadata> {
  const { slug } = await props.params
  const page = await getPage(slug || [])

  if (!page) {
    return { title: 'Not Found' }
  }

  const pathname = slug?.length ? `/${slug.join('/')}` : '/'
  const fullTitle =
    pathname === '/'
      ? 'Synapse Documentation'
      : page.title
        ? `${page.title} | Synapse`
        : 'Synapse Documentation'

  return {
    title:
      pathname === '/'
        ? { absolute: 'Synapse Documentation' }
        : page.title,
    description: SITE_DESCRIPTION,
    alternates: {
      canonical: pathname,
    },
    openGraph: {
      url: pathname,
      title: fullTitle,
      description: SITE_DESCRIPTION,
      images: [
        {
          url: OG_IMAGE_PATH,
          alt: pathname === '/' ? 'Synapse' : page.title ? `${page.title} — Synapse` : 'Synapse',
        },
      ],
    },
    twitter: {
      card: 'summary_large_image',
      title: fullTitle,
      description: SITE_DESCRIPTION,
      images: [OG_IMAGE_PATH],
    },
  }
}

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>
}) {
  const { slug } = await props.params
  const page = await getPage(slug || [])

  if (!page) {
    notFound()
  }

  return (
    <div className="flex">
      <article className="flex-1 min-w-0 py-10 px-6 sm:px-8 lg:px-12">
        <div className="prose prose-neutral dark:prose-invert max-w-none">
          {page.content}
        </div>
      </article>
      {page.headings.length > 0 && (
        <TableOfContents headings={page.headings} />
      )}
    </div>
  )
}
