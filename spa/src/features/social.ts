import { z } from "zod"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { del, get, post } from "@/lib/api/client"

export const remoteActorDtoSchema = z.object({
  handle: z.string(),
  display_name: z.string().optional(),
  url: z.string(),
})
export type RemoteActorDto = z.infer<typeof remoteActorDtoSchema>

export const actorListResponseSchema = z.object({
  actors: z.array(remoteActorDtoSchema),
})
export type ActorListResponse = z.infer<typeof actorListResponseSchema>

export const followRequestSchema = z.object({
  handle: z.string(),
})
export type FollowRequest = z.infer<typeof followRequestSchema>

export const actorUrlRequestSchema = z.object({
  actor_url: z.string(),
})
export type ActorUrlRequest = z.infer<typeof actorUrlRequestSchema>

export const blockedDomainResponseSchema = z.object({
  domain: z.string(),
  reason: z.string().optional(),
  blocked_at: z.string(),
})
export type BlockedDomainResponse = z.infer<typeof blockedDomainResponseSchema>

export const addBlockedDomainRequestSchema = z.object({
  domain: z.string(),
  reason: z.string().optional(),
})
export type AddBlockedDomainRequest = z.infer<typeof addBlockedDomainRequestSchema>

export const blockedActorResponseSchema = z.object({
  url: z.string(),
  handle: z.string(),
  display_name: z.string().optional(),
  avatar_url: z.string().optional(),
})
export type BlockedActorResponse = z.infer<typeof blockedActorResponseSchema>

function getFollowing() {
  return get<ActorListResponse>("/social/following")
}

function getFollowers() {
  return get<ActorListResponse>("/social/followers")
}

function getUserFollowing(userId: string) {
  return get<ActorListResponse>(`/users/${userId}/following`)
}

function getUserFollowers(userId: string) {
  return get<ActorListResponse>(`/users/${userId}/followers`)
}

function getPendingFollowers() {
  return get<ActorListResponse>("/social/followers/pending")
}

function follow(data: FollowRequest) {
  return post("/social/follow", data)
}

function unfollow(data: ActorUrlRequest) {
  return post("/social/unfollow", data)
}

function acceptFollower(data: ActorUrlRequest) {
  return post("/social/followers/accept", data)
}

function rejectFollower(data: ActorUrlRequest) {
  return post("/social/followers/reject", data)
}

function removeFollower(data: ActorUrlRequest) {
  return post("/social/followers/remove", data)
}

function getBlockedDomains() {
  return get<BlockedDomainResponse[]>("/admin/blocked-domains")
}

function addBlockedDomain(data: AddBlockedDomainRequest) {
  return post("/admin/blocked-domains", data)
}

function removeBlockedDomain(domain: string) {
  return del(`/admin/blocked-domains/${domain}`)
}

function blockActor(data: ActorUrlRequest) {
  return post("/social/block", data)
}

function unblockActor(data: ActorUrlRequest) {
  return post("/social/unblock", data)
}

function getBlockedActors() {
  return get<BlockedActorResponse[]>("/social/blocked")
}

export const socialKeys = {
  following: ["following"] as const,
  followers: ["followers"] as const,
  pending: ["followers-pending"] as const,
  userFollowing: (id: string) => ["following", id] as const,
  userFollowers: (id: string) => ["followers", id] as const,
  blockedDomains: ["blocked-domains"] as const,
  blockedActors: ["blocked-actors"] as const,
}

export function useUserFollowing(userId: string) {
  return useQuery({
    queryKey: socialKeys.userFollowing(userId),
    queryFn: () => getUserFollowing(userId),
    enabled: !!userId,
  })
}

export function useUserFollowers(userId: string) {
  return useQuery({
    queryKey: socialKeys.userFollowers(userId),
    queryFn: () => getUserFollowers(userId),
    enabled: !!userId,
  })
}

export function useFollowing() {
  return useQuery({
    queryKey: socialKeys.following,
    queryFn: getFollowing,
  })
}

export function useFollowers() {
  return useQuery({
    queryKey: socialKeys.followers,
    queryFn: getFollowers,
  })
}

export function usePendingFollowers() {
  return useQuery({
    queryKey: socialKeys.pending,
    queryFn: getPendingFollowers,
  })
}

export function useFollow() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: FollowRequest) => follow(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.following })
    },
  })
}

export function useUnfollow() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => unfollow(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.following })
    },
  })
}

export function useAcceptFollower() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => acceptFollower(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.pending })
      qc.invalidateQueries({ queryKey: socialKeys.followers })
    },
  })
}

export function useRejectFollower() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => rejectFollower(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.pending })
    },
  })
}

export function useRemoveFollower() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => removeFollower(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.followers })
    },
  })
}

export function useBlockedDomains() {
  return useQuery({
    queryKey: socialKeys.blockedDomains,
    queryFn: getBlockedDomains,
  })
}

export function useAddBlockedDomain() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: AddBlockedDomainRequest) => addBlockedDomain(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.blockedDomains })
    },
  })
}

export function useRemoveBlockedDomain() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (domain: string) => removeBlockedDomain(domain),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.blockedDomains })
    },
  })
}

export function useBlockActor() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => blockActor(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.blockedActors })
    },
  })
}

export function useUnblockActor() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (data: ActorUrlRequest) => unblockActor(data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: socialKeys.blockedActors })
    },
  })
}

export function useBlockedActors() {
  return useQuery({
    queryKey: socialKeys.blockedActors,
    queryFn: getBlockedActors,
  })
}
