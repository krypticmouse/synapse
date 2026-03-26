export interface NavItem {
  title: string
  href: string
}

export interface NavSection {
  title: string
  items: NavItem[]
}

export type NavEntry = NavItem | NavSection

export const navigation: NavEntry[] = [
  { title: 'Introduction', href: '/' },
  { title: 'Getting Started', href: '/getting-started' },
  {
    title: 'Language Guide',
    items: [
      { title: 'Overview', href: '/language/overview' },
      { title: 'Config Block', href: '/language/config' },
      { title: 'Memory', href: '/language/memory' },
      { title: 'Event Handlers', href: '/language/handlers' },
      { title: 'Queries', href: '/language/queries' },
      { title: 'Update Rules', href: '/language/updates' },
      { title: 'Channels', href: '/language/channels' },
      { title: 'Policies', href: '/language/policies' },
      { title: 'Type System', href: '/language/types' },
      { title: 'Expressions & Statements', href: '/language/expressions' },
      { title: 'Extern Functions', href: '/language/extern-functions' },
    ],
  },
  {
    title: 'Tutorials',
    items: [
      { title: 'Hello World', href: '/tutorials/hello-world' },
      { title: 'Building a Fact Memory', href: '/tutorials/fact-memory' },
      { title: 'Conversation Memory', href: '/tutorials/conversation-memory' },
      { title: 'Knowledge Graph with Zep', href: '/tutorials/knowledge-graph' },
      { title: 'Multi-Backend Systems', href: '/tutorials/multi-backend' },
      { title: 'Channel Ingestion', href: '/tutorials/channel-ingestion' },
    ],
  },
  {
    title: 'Examples',
    items: [
      { title: 'Hello World', href: '/examples/hello' },
      { title: 'Agent Facts', href: '/examples/agent-facts' },
      { title: 'Conversation Memory', href: '/examples/conversation' },
      { title: 'User Profile', href: '/examples/user-profile' },
      { title: 'Letta (Tiered Memory)', href: '/examples/letta' },
      { title: 'Zep (Temporal Knowledge Graph)', href: '/examples/zep' },
      { title: 'SuperMemory (Universal Layer)', href: '/examples/supermemory' },
      { title: 'Multi-Backend', href: '/examples/multi-backend' },
      { title: 'Channel Ingestion', href: '/examples/channels' },
    ],
  },
  {
    title: 'Reference',
    items: [
      { title: 'CLI Commands', href: '/reference/cli' },
      { title: 'Built-in Functions', href: '/reference/builtins' },
      { title: 'HTTP API', href: '/reference/http-api' },
      { title: 'Storage Backends', href: '/reference/backends' },
      { title: 'Channel Connectors', href: '/reference/connectors' },
      { title: 'SDKs', href: '/reference/sdks' },
    ],
  },
]
