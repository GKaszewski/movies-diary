import { useMutation, useQueryClient } from "@tanstack/react-query"
import { useAuth } from "@/components/auth-provider"
import { apiLogout, login, register } from "@/lib/api/auth"
import type { LoginRequest, RegisterRequest } from "@/lib/api/auth"
import { getRefreshToken } from "@/lib/auth"

export function useLogin() {
  const { login: setAuth } = useAuth()
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: LoginRequest) => login(data),
    onSuccess: (res) => {
      setAuth({
        token: res.token,
        refresh_token: res.refresh_token,
        user_id: res.user_id,
        email: res.email,
        role: res.role,
        expires_at: res.expires_at,
      })
      qc.clear()
    },
  })
}

export function useRegister() {
  return useMutation({
    mutationFn: (data: RegisterRequest) => register(data),
  })
}

export function useLogout() {
  const { logout } = useAuth()
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async () => {
      const rt = getRefreshToken()
      if (rt) {
        try {
          await apiLogout(rt)
        } catch {}
      }
      logout()
      qc.clear()
    },
  })
}
