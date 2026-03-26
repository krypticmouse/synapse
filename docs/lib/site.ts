/** Canonical site URL for Open Graph / Twitter cards. Set at build time for production. */
export function getMetadataBase(): URL {
  const fromEnv = process.env.NEXT_PUBLIC_SITE_URL?.trim()
  if (fromEnv) {
    return new URL(fromEnv.endsWith('/') ? fromEnv : `${fromEnv}/`)
  }
  if (process.env.VERCEL_URL) {
    return new URL(`https://${process.env.VERCEL_URL}/`)
  }
  return new URL('http://localhost:3000/')
}

export const SITE_DESCRIPTION =
  'Synapse: configuration language and runtime for building memory systems for AI agents.'

/** Path under /public; crawlers resolve via metadataBase */
export const OG_IMAGE_PATH = '/synapse_logo.png'

/** `owner/repo` for GitHub links and API */
export const GITHUB_REPO = 'krypticmouse/synapse'

export const GITHUB_REPO_URL = `https://github.com/${GITHUB_REPO}`
