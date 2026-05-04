# movies review

This project is a self-hosted, server-side rendered movie logging system designed to be embedded as a lightweight widget on a personal website. Built entirely in Rust, it acts as an immutable ledger of a filmmaker and software engineer's viewing history, blending a classic web aesthetic with pristine, enterprise-grade backend architecture.

## Core Principles

- **Zero-JS & Bloat-Free:** The web interface relies strictly on standard HTML form submissions and server-side rendering (via Askama). There is absolutely no JavaScript, no Single Page Application (SPA) overhead, and no client-side state to manage.
- **Personal & Embeddable:** It is designed for a single actor. Rather than being a commercial product or a bloated social network, it functions as a highly personal, iframe-ready widget for a personal site.
- **Append-Only Ledger:** Reviews are not rows to be updated; they are immutable events. The system tracks a chronological history of viewings for the same movie, separating domain time (when it was watched) from system time (when it was logged), allowing the user to track how their cinematic taste evolves over time.
- **Pristine Architecture:** It strictly adheres to Domain-Driven Design (DDD) and Hexagonal Architecture (Ports and Adapters). The core domain consists of strong value objects and pure business logic, entirely decoupled from external infrastructure like the SQLite database, TMDB API, and Axum HTTP router.
- **Frictionless "Lazy" Logging:** While the backend is robust, the user experience is minimal. The system automatically fetches rich metadata and poster art in the background via external APIs, requiring only a TMDB ID and a 0-5 rating. It supports both classic HTML forms and a secure REST API for quick terminal or iOS shortcut entries.
- **Old-School Syndication:** Instead of jumping into complex federalized moderation, the project embraces classic, open web standards by generating a native RSS/Atom feed, allowing others to subscribe to the movie diary without needing an account.

# domain

The absolute center. It has zero dependencies on other workspace crates. It holds pure data structures like `Movie`, `Review`, and `User`. This is also where project define the interfaces (Traits) like `MovieRepository`, `MetadataClient`, and `TokenValidator`. It does not know about infrastructure implementations like `SQLite` or JWTs.

# application

It sits between web endpoints and domain. It holds "Use Cases" (e.g., `LogNewMovie`, `GetRecentWatches`). When a request comes in, this crate coordinates the workflow: it asks the adapter-meta for the TMDB data, validates the 0-5 rating using domain rules, and tells database adapter to save it.

# sqlite

`SQLite` and `sqlx` implementation. It implements the `MovieRepository` trait defined in the domain.

# metadata

HTTP client (likely `reqwest`) that talks to `TMDB` or `OMDb`. It implements the `MetadataClient` trait.

# auth

This handles the JWT logic using a crate like `jsonwebtoken`. It issues the tokens when you log in and implements a `TokenValidator` trait to verify claims (like your admin ID) when a request is made.

# presentation

It wires all the traits and adapters together into Axum's application state. Inside this crate, you can split your routing into two clean modules:

- `html_routes`: Uses Askama templates, handles standard form submissions, and checks for the JWT in cookies.
- `rest_routes`: Speaks purely in JSON, handles your background API calls, and checks for the JWT in the Bearer header.
