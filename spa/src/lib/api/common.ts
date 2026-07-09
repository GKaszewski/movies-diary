import { z } from "zod"

export const movieDtoSchema = z.object({
  id: z.string().uuid(),
  title: z.string(),
  release_year: z.number(),
  director: z.string().optional(),
  poster_path: z.string().optional(),
  genres: z.array(z.string()),
  runtime_minutes: z.number().optional(),
  original_language: z.string().optional(),
  overview: z.string().optional(),
  collection_name: z.string().optional(),
})
export type MovieDto = z.infer<typeof movieDtoSchema>

export const reviewDtoSchema = z.object({
  id: z.string().uuid(),
  rating: z.number(),
  comment: z.string().optional(),
  watched_at: z.string(),
  watch_medium: z.string().optional(),
})
export type ReviewDto = z.infer<typeof reviewDtoSchema>

export const diaryEntryDtoSchema = z.object({
  movie: movieDtoSchema,
  review: reviewDtoSchema,
})
export type DiaryEntryDto = z.infer<typeof diaryEntryDtoSchema>

export function paginatedSchema<T extends z.ZodType>(itemSchema: T) {
  return z.object({
    items: z.array(itemSchema),
    total_count: z.number(),
    limit: z.number(),
    offset: z.number(),
  })
}
export type Paginated<T> = {
  items: T[]
  total_count: number
  limit: number
  offset: number
}
