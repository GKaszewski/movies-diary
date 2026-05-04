# Frontend HTML/CSS Design

**Date:** 2026-05-04

## Summary

Server-rendered HTML frontend using Rust/Axum + Askama templates + HTTP-only cookie JWT auth. No JavaScript.

## Pages

| Route | Access | Description |
|---|---|---|
| GET / | public | Diary index |
| GET /login | public | Login form |
| POST /login | public | Set cookie → redirect / |
| GET /logout | — | Clear cookie → redirect / |
| GET /register | public | Only if ALLOW_REGISTRATION |
| POST /register | public | Set cookie → redirect / |
| GET /reviews/new | auth | New review form |
| POST /reviews | auth | Log review → redirect / |

## Design Decisions

- **Auth:** Cookie-based JWT (HttpOnly, SameSite=Lax). Existing Bearer auth untouched.
- **Template inheritance:** base.html owns header. Child templates use {% extends %}/{% block %}.
- **Entry layout:** Poster thumbnail (60px) + text block. Fallback to text-only when no poster.
- **Header (logged out):** [Login] [Register?]
- **Header (logged in):** [Add Review]  email@example.com  [Logout]
- **Form errors:** PRG → redirect back with ?error=<msg>
- **Diary visibility:** Public (anyone can read, auth required to add)
