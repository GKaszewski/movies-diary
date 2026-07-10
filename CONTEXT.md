# Movies Diary

A personal movie diary that tracks what you watch, when, and what you thought about it. Supports federation via ActivityPub.

## Language

**Movie**:
A film in the catalog, identified by title and release year. Optionally linked to an external metadata provider (e.g. TMDb) for enrichment. One Movie record is shared across all users — "Blade Runner (1982)" exists once regardless of how many people review it.
_Avoid_: Film entry, title record

**Person**:
Someone involved in making a movie — actor, director, crew member. Sourced from an external metadata provider and enriched with biographical data. Linked to Movies through cast/crew credits. Not a User — Person is movie-industry people only.
_Avoid_: Celebrity, artist, talent

**Review**:
A single record of watching a movie — captures the rating, optional comment, when it was watched, and how it was watched.
_Avoid_: Diary entry, watch, log entry

**Rating**:
A 1–5 whole-star score given to a movie in a Review. No half-stars, no zero.
_Avoid_: Score, grade, stars (as a noun for the value itself)

**WatchMedium**:
The channel through which a movie was watched: Cinema, Streaming, TV, PhysicalMedia, Download, MediaServer, or Other.
_Avoid_: Source, format, venue, platform

**Watchlist**:
A user's collection of movies they intend to watch. Each item is a simple bookmark — no priority or ordering. A movie leaves the watchlist implicitly when reviewed, or explicitly when removed.
_Avoid_: Queue, backlog, to-watch list

**Goal**:
A yearly target a user sets — e.g. "watch 50 movies in 2025." Progress is tracked automatically as reviews are logged. Currently only supports movie-count goals, but the model is designed for other goal types in the future.
_Avoid_: Challenge, resolution, target

**WrapUp**:
A generated summary report of viewing activity over a date range — statistics, trends, highlights, top directors/actors/genres. Can be personal (one user) or global (all users). Generated asynchronously. Shown to users as "Year in Review."
_Avoid_: Stats page, recap, summary

**User**:
A registered account with a username, email, and profile (display name, bio, avatar, banner). Can be Standard or Admin.
_Avoid_: Account, member, profile (as a synonym for the whole User)

**SocialIdentity**:
The uniform identifier for anyone involved in a social interaction — either a local User or a remote federated actor. Social commands and queries operate on SocialIdentity so the domain never branches on local vs remote.
_Avoid_: Actor, participant, social user

**Follow**:
A social relationship where one user subscribes to another's activity. Always requires acceptance by the target user. Works identically for local and federated (ActivityPub) users. Once accepted, the followed user's reviews appear in the follower's Feed.
_Avoid_: Subscribe, connect, friend

**Feed**:
A chronological timeline of reviews from users you follow — both local and federated. The main social surface of the app.
_Avoid_: Timeline, activity stream, home

**WatchEvent**:
An automatically detected viewing reported by an external source — currently Jellyfin and Plex via webhook, but conceptually any system that can report "this person watched this movie" (e.g. a cinema ticket service). Arrives in a pending state; the user confirms it (creating a Review) or dismisses it.
_Avoid_: Playback event, webhook event, auto-import

**Import**:
Bulk ingestion of reviews from an external file — Letterboxd CSV, IMDb CSV, or a generic JSON format. The user uploads a file, column mappings are applied, and reviews are created in batch.
_Avoid_: Upload, migration, sync

**ImportProfile**:
A saved set of column-to-field mappings for an Import. Reusable across imports and shareable between users.
_Avoid_: Template, mapping preset, import config
