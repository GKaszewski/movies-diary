const AUTH_KEY = "auth_state"

export type AuthState = {
  token: string
  user_id: string
  email: string
  role: string
  expires_at: string
}

export function getAuth(): AuthState | null {
  const raw = localStorage.getItem(AUTH_KEY)
  if (!raw) return null
  try {
    return JSON.parse(raw) as AuthState
  } catch {
    return null
  }
}

export function setAuth(state: AuthState) {
  localStorage.setItem(AUTH_KEY, JSON.stringify(state))
}

export function clearAuth() {
  localStorage.removeItem(AUTH_KEY)
}

export function getToken(): string | null {
  return getAuth()?.token ?? null
}

export function isAdmin(): boolean {
  return getAuth()?.role === "admin"
}
