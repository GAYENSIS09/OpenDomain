import nextra from 'nextra'

// Nextra 4 is app-router only and has no `theme.config` file. Two consequences
// shape this file: the theme is implied by the `nextra-theme-docs` dependency,
// and every theme option is passed as a prop to <Layout>/<Navbar>/<Banner> in
// app/layout.jsx instead.
//
// `search` is one of the few options that still belongs here, because it is an
// MDX-compiler concern (it marks the body for indexing) rather than a theme
// concern. The docs here are prose; the code blocks are shell transcripts that
// would only dilute a query.
const withNextra = nextra({
  search: { codeblocks: false }
})

export default withNextra({
  // Required from Next 15.3 onwards, otherwise the MDX compiler cannot resolve
  // the components file and the build fails with a module-not-found that names
  // no file of ours. The path has to carry the extension.
  turbopack: {
    resolveAlias: {
      'next-mdx-import-source-file': './mdx-components.js'
    }
  }
})
