import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  acceptFollower,
  addBlockedDomain,
  blockActor,
  follow,
  getBlockedActors,
  getBlockedDomains,
  getFollowers,
  getFollowing,
  getPendingFollowers,
  getUserFollowers,
  getUserFollowing,
  rejectFollower,
  removeBlockedDomain,
  removeFollower,
  unblockActor,
  unfollow,
} from "@/lib/api/social"
import type {
  ActorUrlRequest,
  AddBlockedDomainRequest,
  FollowRequest,
} from "@/lib/api/social"

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
    mutationFn: (data: FollowRequest) => unfollow(data),
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
