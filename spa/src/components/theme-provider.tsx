/* eslint-disable react-refresh/only-export-components */
import * as React from "react"

type ThemeProviderState = {
  theme: "dark"
  setTheme: (theme: "dark") => void
}

const ThemeProviderContext = React.createContext<
  ThemeProviderState | undefined
>(undefined)

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  React.useEffect(() => {
    document.documentElement.classList.add("dark")
  }, [])

  const value = React.useMemo(
    () => ({ theme: "dark" as const, setTheme: () => {} }),
    [],
  )

  return (
    <ThemeProviderContext.Provider value={value}>
      {children}
    </ThemeProviderContext.Provider>
  )
}

export const useTheme = () => {
  const context = React.useContext(ThemeProviderContext)

  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider")
  }

  return context
}
