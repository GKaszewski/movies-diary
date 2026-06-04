# Movies Diary — SPA

Mobile-first single page application for Movies Diary, served at `/app/`.

## Stack

- **React 19** + **TypeScript**
- **TanStack Router** — file-based routing
- **TanStack Query** — data fetching, caching, mutations
- **Tailwind CSS v4** + **shadcn/ui** — styling and components
- **Vaul** — mobile drawers
- **date-fns** — date formatting
- **i18next** — internationalization
- **Zod** — API response validation

## Routes

| Path | Page |
|------|------|
| `/login` | Login |
| `/register` | Registration |
| `/` | Home — Feed / Watchlist / Queue tabs |
| `/diary` | Diary with month navigation + CSV export |
| `/search` | Search movies and people |
| `/profile` | Own profile with trends |
| `/social` | Following / Followers / Pending |
| `/movies/:id` | Movie detail — cast, crew, stats, reviews |
| `/people/:id` | Person detail — filmography |
| `/users/:id` | Other user's profile |
| `/wrapup/:id` | Year in Review report |
| `/settings/` | Settings hub |
| `/settings/edit-profile` | Edit display name, bio, avatar, banner |
| `/settings/import` | CSV/JSON/XLSX import wizard |
| `/settings/webhooks` | Jellyfin/Plex webhook tokens |
| `/settings/wrapup` | Generate/manage year wrap-ups |
| `/settings/blocked` | Blocked users and domains (admin) |

## Development

```bash
npm install
npm run dev
```

The SPA expects the backend API at the URL defined by `VITE_API_URL` (defaults to empty string = same origin).

Create `spa/.env` for local development:

```
VITE_API_URL=http://localhost:3000
```

## Build

```bash
npm run build
```

Output goes to `dist/`, served by the backend at `/app/`.

## Project Structure

```
src/
├── components/        # Reusable UI components
│   ├── ui/           # shadcn/ui primitives
│   ├── back-button   # History-aware back navigation
│   ├── movie-card    # Movie display (compact/full)
│   ├── review-card   # Review with user, rating, date
│   ├── person-row    # Person search result
│   ├── log-sheet     # Log review drawer
│   ├── star-rating   # Interactive star input (with haptics)
│   ├── swipe-to-delete
│   └── ...
├── hooks/             # TanStack Query hooks
│   ├── use-diary      # Feed, diary, log/delete review
│   ├── use-movies     # Movie detail, profile, history
│   ├── use-search     # Search with infinite scroll
│   ├── use-social     # Follow/unfollow, block
│   ├── use-users      # User profiles, admin reindex
│   ├── use-watchlist   # Watchlist CRUD
│   ├── use-webhooks   # Webhook tokens, watch queue
│   └── use-wrapup     # Wrap-up generation/reports
├── lib/
│   ├── api/           # Typed API client (get/post/put/del)
│   ├── auth.ts        # Token storage
│   ├── date.ts        # timeAgo, shortDate formatters
│   ├── haptics.ts     # Vibration feedback
│   └── query-client.ts # QueryClient with error toasts
├── locales/           # i18n translations
└── routes/            # File-based TanStack Router pages
```

## Features

- **Federation-aware** — globe badge on federated reviews, `@user@instance` handles
- **Admin tools** — search reindex, user picker for wrap-ups, domain blocking
- **Offline-friendly** — stale-while-revalidate caching, retry on 5xx
- **Mobile UX** — swipe-to-delete, haptic feedback, iOS keyboard-safe drawers
- **Interactivity** — clickable cast/crew → people pages, movie highlights → detail
