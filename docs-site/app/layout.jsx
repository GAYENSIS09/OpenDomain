import 'nextra-theme-docs/style.css'

export const metadata = { title: 'OpenDomain' }

export default function RootLayout({ children }) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  )
}
