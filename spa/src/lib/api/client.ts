import { clearAuth, getToken } from "@/lib/auth"

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
      clearAuth()
      window.location.href = "/login"
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
