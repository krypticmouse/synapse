import { useMDXComponents as getDocsMDXComponents } from 'nextra-theme-docs'

const themeComponents = getDocsMDXComponents()

export function useMDXComponents(components) {
  return {
    ...themeComponents,
    ...components,
  }
}
