import { z } from "zod"
import { paginatedSchema } from "./common"
import { get } from "./client"

export const searchQueryParamsSchema = z.object({
  q: z.string().optional(),
  genre: z.string().optional(),
  year: z.number().optional(),
  person_id: z.string().uuid().optional(),
  department: z.string().optional(),
  language: z.string().optional(),
  limit: z.number().optional(),
  offset: z.number().optional(),
})
export type SearchQueryParams = z.infer<typeof searchQueryParamsSchema>

export const movieSearchHitDtoSchema = z.object({
  movie_id: z.string().uuid(),
  title: z.string(),
  release_year: z.number().optional(),
  director: z.string().optional(),
  poster_path: z.string().optional(),
  genres: z.array(z.string()),
})
export type MovieSearchHitDto = z.infer<typeof movieSearchHitDtoSchema>

export const personSearchHitDtoSchema = z.object({
  person_id: z.string().uuid(),
  name: z.string(),
  known_for_department: z.string().optional(),
  profile_path: z.string().optional(),
  known_for_titles: z.array(z.string()),
})
export type PersonSearchHitDto = z.infer<typeof personSearchHitDtoSchema>

export const searchResponseSchema = z.object({
  movies: paginatedSchema(movieSearchHitDtoSchema),
  people: paginatedSchema(personSearchHitDtoSchema),
})
export type SearchResponse = z.infer<typeof searchResponseSchema>

export const personDtoSchema = z.object({
  id: z.string().uuid(),
  external_id: z.string(),
  name: z.string(),
  known_for_department: z.string().optional(),
  profile_path: z.string().optional(),
  biography: z.string().optional(),
  birthday: z.string().optional(),
  deathday: z.string().optional(),
  place_of_birth: z.string().optional(),
  also_known_as: z.array(z.string()).default([]),
  homepage: z.string().optional(),
  imdb_url: z.string().optional(),
  enriched: z.boolean().default(false),
})
export type PersonDto = z.infer<typeof personDtoSchema>

export const castCreditDtoSchema = z.object({
  movie_id: z.string().uuid(),
  title: z.string(),
  release_year: z.number().optional(),
  character: z.string(),
  poster_path: z.string().optional(),
})
export type CastCreditDto = z.infer<typeof castCreditDtoSchema>

export const crewCreditDtoSchema = z.object({
  movie_id: z.string().uuid(),
  title: z.string(),
  release_year: z.number().optional(),
  job: z.string(),
  department: z.string(),
  poster_path: z.string().optional(),
})
export type CrewCreditDto = z.infer<typeof crewCreditDtoSchema>

export const personCreditsDtoSchema = z.object({
  person: personDtoSchema,
  cast: z.array(castCreditDtoSchema),
  crew: z.array(crewCreditDtoSchema),
})
export type PersonCreditsDto = z.infer<typeof personCreditsDtoSchema>

export function search(params: SearchQueryParams) {
  return get<SearchResponse>("/search", params)
}

export function getPerson(id: string) {
  return get<PersonDto>(`/people/${id}`)
}

export function getPersonCredits(id: string) {
  return get<PersonCreditsDto>(`/people/${id}/credits`)
}
