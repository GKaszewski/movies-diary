import { clearAuth, getAuth, getToken, setAuth } from "@/lib/auth"

export const API_URL = import.meta.env.VITE_API_URL ?? ""

export function posterUrl(path: string | undefined | null): string | undefined {
  if (!path) return undefined
  const clean = path.startsWith("/") ? path.slice(1) : path
  return `${API_URL}/images/${clean}`
}

export function tmdbProfileUrl(path: string | undefined | null): string | undefined {
  if (!path) return undefined
  return `https://image.tmdb.org/t/p/w185${path}`
}

export class ApiError extends Error {
  status: number
  body: string
  constructor(status: number, body: string) {
    super(`API ${status}: ${body}`)
    this.status = status
    this.body = body
  }
}

function authHeaders(): HeadersInit {
  const token = getToken()
  return token ? { Authorization: `Bearer ${token}` } : {}
}

function buildUrl(
  path: string,
  params?: Record<string, unknown>,
): string {
  const base = `${API_URL}/api/v1${path}`
  if (!params) return base
  const sp = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v != null) sp.set(k, String(v))
  }
  const qs = sp.toString()
  return qs ? `${base}?${qs}` : base
}

let refreshPromise: Promise<boolean> | null = null

async function tryRefresh(): Promise<boolean> {
  const auth = getAuth()
  if (!auth?.refresh_token) return false
  try {
    const res = await fetch(buildUrl("/auth/refresh"), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: auth.refresh_token }),
    })
    if (!res.ok) return false
    const data = await res.json()
    setAuth({
      ...auth,
      token: data.token,
      refresh_token: data.refresh_token,
      expires_at: data.expires_at,
    })
    return true
  } catch {
    return false
  }
}

async function request<T = void>(
  url: string,
  options?: RequestInit,
): Promise<T> {
  const res = await fetch(url, {
    ...options,
    headers: {
      ...authHeaders(),
      ...options?.headers,
    },
  })
  if (!res.ok) {
    if (res.status === 401) {
      if (url.includes("/auth/refresh") || url.includes("/auth/login")) {
        clearAuth()
        window.location.href = "/app/login"
        throw new ApiError(res.status, await res.text())
      }

      if (!refreshPromise) {
        refreshPromise = tryRefresh().finally(() => {
          refreshPromise = null
        })
      }
      const ok = await refreshPromise
      if (!ok) {
        clearAuth()
        window.location.href = "/app/login"
        throw new ApiError(res.status, await res.text())
      }

      const retryRes = await fetch(url, {
        ...options,
        headers: {
          ...authHeaders(),
          ...options?.headers,
        },
      })
      if (!retryRes.ok) {
        throw new ApiError(retryRes.status, await retryRes.text())
      }
      const retryText = await retryRes.text()
      return retryText ? JSON.parse(retryText) : (undefined as T)
    }
    throw new ApiError(res.status, await res.text())
  }
  const text = await res.text()
  return text ? JSON.parse(text) : (undefined as T)
}

export async function get<T>(
  path: string,
  params?: Record<string, unknown>,
): Promise<T> {
  return request<T>(buildUrl(path, params))
}

export async function post<T = void>(
  path: string,
  body?: unknown,
): Promise<T> {
  return request<T>(buildUrl(path), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  })
}

export async function put<T = void>(
  path: string,
  body?: unknown,
): Promise<T> {
  return request<T>(buildUrl(path), {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  })
}

export async function putForm<T = void>(
  path: string,
  form: FormData,
): Promise<T> {
  return request<T>(buildUrl(path), {
    method: "PUT",
    body: form,
  })
}

export async function del<T = void>(path: string): Promise<T> {
  return request<T>(buildUrl(path), { method: "DELETE" })
}

export async function upload<T>(
  path: string,
  file: File,
): Promise<T> {
  const form = new FormData()
  form.append("file", file)
  return request<T>(buildUrl(path), {
    method: "POST",
    body: form,
  })
}

export async function uploadWithFields<T>(
  path: string,
  file: File,
  fields: Record<string, string>,
): Promise<T> {
  const form = new FormData()
  form.append("file", file)
  for (const [k, v] of Object.entries(fields)) form.append(k, v)
  return request<T>(buildUrl(path), {
    method: "POST",
    body: form,
  })
}
