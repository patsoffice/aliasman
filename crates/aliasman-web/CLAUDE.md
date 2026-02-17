# aliasman-web

Web frontend using Axum, Askama templates, and HTMX.

## Rules

- Error handling: custom `AppError` in `error.rs` — implements `IntoResponse` for Axum
- State: `AppState` (in `state.rs`) is wrapped in `Arc` (`SharedState` type alias) and shared across handlers
- Storage providers are lazy-loaded and cached in `RwLock<HashMap>` — always use `AppState` methods (`list_aliases`, `switch_system`, `refresh_active_system`), never acquire locks directly in route handlers
- Templates: Askama templates in `templates/` — partials for HTMX fragments in `templates/partials/`
- Static assets: embedded via `include_dir` and served from `/static/`
- Auth: `auth.rs` contains RBAC stubs for future implementation — not yet wired in

## HTMX Pattern

- Full page loads return complete HTML (base template)
- HTMX requests return partial HTML fragments (partials only)
- Routes returning partials: `/aliases`, `/system` (POST), `/refresh` (POST)
- Route returning full page: `/`

## Adding a Route

1. Add handler function in `routes.rs`
2. Create an Askama template struct with `#[template(path = "...")]`
3. For HTMX endpoints: return partial HTML, not the full page
4. Register the route in the `router()` function in `routes.rs`
5. If the handler needs state, take `State<SharedState>` as an extractor
