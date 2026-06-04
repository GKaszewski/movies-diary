import { createContext, useCallback, useContext, useMemo, useSyncExternalStore } from "react"
import { type AuthState, clearAuth, getAuth, setAuth } from "@/lib/auth"

type AuthContextValue = {
  auth: AuthState | null
  login: (state: AuthState) => void
  logout: () => void
  isAdmin: boolean
}

const AuthContext = createContext<AuthContextValue | null>(null)

let listeners: Array<() => void> = []
let cachedRaw: string | null = undefined as unknown as string | null
let cachedAuth: AuthState | null = null

function subscribe(cb: () => void) {
  listeners = [...listeners, cb]
  return () => {
    listeners = listeners.filter((l) => l !== cb)
  }
}
function emitChange() {
  cachedRaw = undefined as unknown as string | null
  for (const l of listeners) l()
}
function getSnapshot(): AuthState | null {
  const raw = localStorage.getItem("auth_state")
  if (raw === cachedRaw) return cachedAuth
  cachedRaw = raw
  cachedAuth = getAuth()
  return cachedAuth
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const auth = useSyncExternalStore(subscribe, getSnapshot, () => null)

  const login = useCallback((state: AuthState) => {
    setAuth(state)
    emitChange()
  }, [])

  const logout = useCallback(() => {
    clearAuth()
    emitChange()
  }, [])

  const value = useMemo(
    () => ({
      auth,
      login,
      logout,
      isAdmin: auth?.role === "admin",
    }),
    [auth, login, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error("useAuth must be used within AuthProvider")
  return ctx
}

export function useIsAdmin() {
  return useAuth().isAdmin
}
