# Wellness Streak Duel — backend service

Real-time multiplayer backend for the Step & Share game, built for the FLAME server.

## Stack
- **actix-web 4** — HTTP + WebSocket server
- **sqlx (Postgres)** — async, no ORM macros requiring a live DB at compile time
- **tokio broadcast channels** — one per room, fan out live updates to every connected socket
- No Redis: a single-process, single-server deployment doesn't need cross-process pub/sub.
  If this ever needs to run across multiple server instances, that's the point to add it back.

## First-time setup on the FLAME server

```bash
# 1. Create a dedicated database (separate from academicplanning)
sudo -u postgres createdb wellness_duel

# 2. Copy this whole project to the server, then:
cd wellness-duel-service
cp .env.example .env
# edit .env: set DB_PASSWORD to your real Postgres password

# 3. Build
cargo build --release
# Migrations run automatically on startup — no manual `psql < migrations/...` needed.

# 4. Install as a systemd service (one-time)
make install

# 5. Start it
make start
make health   # should return {"status": "ok"}
```

## Day-to-day deploy loop
```bash
make restart   # rebuilds + restarts the systemd service
make logs      # tail live logs
make status    # check it's running
```

## How a room works
1. `POST /api/rooms` creates a room, returns a 6-character code (e.g. `AB3XZ9`).
2. Each player calls `POST /api/rooms/{code}/join` with a random per-device
   token (generated once client-side, stored in their browser) and a name.
3. Every player opens a WebSocket to `/ws/{code}`, which is a **read-only
   live feed** — it pushes a fresh scoreboard the instant anyone checks in.
4. All actual writes go through `POST /api/rooms/{code}/checkin` (a normal
   REST call, multipart so it can carry an optional photo). The server
   recomputes points from scratch every time — a client can say "I did
   squats," never "give me 3 points."
5. A "day" is real elapsed calendar time since the room was created, not a
   client-side button — nobody can rapid-fire through a week.

## What I could not verify myself
My own sandbox only has an apt-installed Rust 1.75 (Dec 2023), and several
current crate versions now require edition2024 (rustc 1.85+), so a full
`cargo build` here is not possible. I validated as much as I could: every
source file passes a real syntax parse, and I smoke-tested the dependency
resolution and most of the HTTP/WebSocket wiring against a deliberately
downgraded dependency set before hitting that wall. Your FLAME server almost
certainly has a modern enough toolchain already, since it's already building
this same actix-web/sqlx family for your other service — **run `cargo build`
there as the real first test**, and send me the compiler output if anything
doesn't line up. The most likely spot for a small fix, if any, is the
`actix-ws` session API in `src/ws.rs`, since that crate's exact method
signatures are the one piece I was least able to cross-check offline.
