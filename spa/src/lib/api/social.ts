import { z } from "zod"
import { del, get, post } from "./client"

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

export function getFollowing() {
  return get<ActorListResponse>("/social/following")
}

export function getFollowers() {
  return get<ActorListResponse>("/social/followers")
}

export function getUserFollowing(userId: string) {
  return get<ActorListResponse>(`/users/${userId}/following`)
}

export function getUserFollowers(userId: string) {
  return get<ActorListResponse>(`/users/${userId}/followers`)
}

export function getPendingFollowers() {
  return get<ActorListResponse>("/social/followers/pending")
}

export function follow(data: FollowRequest) {
  return post("/social/follow", data)
}

export function unfollow(data: ActorUrlRequest) {
  return post("/social/unfollow", data)
}

export function acceptFollower(data: ActorUrlRequest) {
  return post("/social/followers/accept", data)
}

export function rejectFollower(data: ActorUrlRequest) {
  return post("/social/followers/reject", data)
}

export function removeFollower(data: ActorUrlRequest) {
  return post("/social/followers/remove", data)
}

export function getBlockedDomains() {
  return get<BlockedDomainResponse[]>("/admin/blocked-domains")
}

export function addBlockedDomain(data: AddBlockedDomainRequest) {
  return post("/admin/blocked-domains", data)
}

export function removeBlockedDomain(domain: string) {
  return del(`/admin/blocked-domains/${domain}`)
}

export function blockActor(data: ActorUrlRequest) {
  return post("/social/block", data)
}

export function unblockActor(data: ActorUrlRequest) {
  return post("/social/unblock", data)
}

export function getBlockedActors() {
  return get<BlockedActorResponse[]>("/social/blocked")
}
