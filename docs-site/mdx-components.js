import { useMDXComponents as getThemeComponents } from 'nextra-theme-docs'

// Nextra 4 requires this file to exist. It is the single place where MDX tags
// are mapped to React components, so a page can use a component without
// importing it.
const themeComponents = getThemeComponents()

export function useMDXComponents(components) {
  return { ...themeComponents, ...components }
}
