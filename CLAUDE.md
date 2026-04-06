# Remora — Claude Project Instructions

## Collaboration Rules

- **Vineet writes all the code.** Claude's job is to guide, explain, and show code snippets — never to write files directly.
- Claude should show code blocks clearly so Vineet can type/paste them himself.
- Claude should explain *why* before showing *what* — reasoning first, then the code.
- If Vineet is stuck, Claude should give a targeted hint before revealing the full solution.
- Keep responses focused and direct. No padding.

---

## Project Overview

**Remora** is a 3-day Rust Zero Trust proxy prototype that demonstrates the core concepts required for the Zscaler Principal Rust Developer role (Platform Convergence Team).

### Tech Stack
- `tokio` — async runtime
- `tonic` + `prost` — gRPC server/client + protobuf codegen
- `jsonwebtoken` — JWT issuance and validation
- `dashmap` — concurrent in-memory policy store
- `clap` — CLI argument parsing
- `tracing` + `tracing-subscriber` — structured logging and spans
- `tokio-console` — async task profiling (Day 2 addition)
- `criterion` — micro-benchmarking (Day 2 addition)

---

## Day 1 — gRPC Control Plane with Policy Engine and JWT Auth

**Goal:** Build a gRPC server that enforces access policy and issues/validates JWT tokens.

### Step 1 — Cargo workspace setup
- Create a Cargo workspace with two crates: `control-plane` and `proto`
- `proto` holds `.proto` definitions and the `build.rs` codegen
- `control-plane` is the binary crate

### Step 2 — Define the protobuf schema
- Define a `PolicyService` with two RPCs:
  - `CheckAccess(AccessRequest) → AccessResponse`
  - `IssueToken(TokenRequest) → TokenResponse`
- Fields: `user_id`, `resource`, `action`, `allowed`, `token`

### Step 3 — Implement the policy engine
- Use `DashMap<String, Vec<String>>` as the in-memory policy store (user → allowed resources)
- Seed a few policies at startup
- Implement `CheckAccess`: look up user, check resource, return `allowed: true/false`

### Step 4 — Implement JWT issuance and validation
- On `IssueToken`: generate a signed JWT with claims `{ sub: user_id, exp: now+1h }`
- Add a gRPC interceptor that validates the JWT on every `CheckAccess` call
- Use `HS256` signing with a hardcoded secret (note: in prod this would be a KMS-managed key)

### Step 5 — Wire it up and test
- Run the server, write a small client binary that:
  - Issues a token
  - Calls `CheckAccess` with the token
  - Prints `ALLOW` or `DENY`

**Concepts demonstrated:** async Rust, gRPC, protobuf, concurrent state, JWT auth, interceptors

---

## Day 2 — TCP L4 Forwarding + Performance Profiling

**Goal:** Build a TCP proxy that forwards connections to a backend, then profile it.

### Step 1 — Basic TCP forwarder
- Listen on `0.0.0.0:8080` using `TcpListener`
- For each incoming connection, connect to a configurable backend (`host:port`)
- Bidirectional copy using `tokio::io::copy_bidirectional`
- Each connection handled in its own `tokio::spawn` task

### Step 2 — Policy enforcement at L4
- Before forwarding, call the control plane's `CheckAccess` gRPC endpoint
- Pass `user_id` from a simple header or config, `resource` as the backend address
- If `DENY`, close the connection immediately with a log message

### Step 3 — tokio-console instrumentation
- Add `console-subscriber` to the forwarder
- Instrument key tasks with `tracing::instrument`
- Run `tokio-console` and observe task lifecycle, poll times, and waker activity
- **Interview talking point:** Explain what you see — long poll times, task starvation signals

### Step 4 — Criterion benchmark
- Write a benchmark that measures bidirectional copy throughput (bytes/sec) for:
  - 1KB payloads
  - 64KB payloads
  - 1MB payloads
- Run `cargo bench` and record baseline numbers
- **Interview talking point:** Where is the bottleneck? Syscall overhead? Buffer sizing? `splice(2)` as a kernel-bypass option?

### Step 5 — Flame graph (stretch)
- Build with `CARGO_PROFILE_RELEASE_DEBUG=true`
- Use `cargo flamegraph` to profile the forwarder under load (`iperf3` or a simple load script)
- Identify hot paths

**Concepts demonstrated:** Linux TCP sockets, async task spawning, bidirectional I/O, kernel/userspace profiling, benchmarking

---

## Day 3 — CLI, Observability, and Talking Points

**Goal:** Polish the CLI, add structured observability, and prepare talking points for gaps.

### Step 1 — Unified CLI with clap
- Single binary entrypoint with subcommands:
  - `remora control-plane --port 50051`
  - `remora proxy --listen 8080 --backend localhost:9000 --control-plane localhost:50051`
  - `remora client --user alice --resource db.internal`

### Step 2 — Structured tracing
- Use `tracing-subscriber` with JSON formatter for production-style logs
- Add spans around: connection accept, policy check, token validation, bytes forwarded
- Emit a summary log on connection close: `{ user, resource, bytes_tx, bytes_rx, duration_ms, verdict }`

### Step 3 — Metrics (stretch)
- Add `prometheus` crate and expose `/metrics` on a side HTTP port
- Track: `connections_total`, `connections_active`, `bytes_forwarded_total`, `policy_check_latency_ms`

### Step 4 — Talking points for JD gaps

#### eBPF / XDP
- Where it fits: replace the userspace TCP accept loop with an XDP program that does L4 load balancing in the kernel, bypassing the network stack entirely
- Tooling: `aya` (Rust-native eBPF), `libbpf`
- Trade-off: complexity vs. latency — XDP is sub-microsecond but debugging is hard

#### QUIC
- Where it fits: replace the gRPC-over-HTTP/2-over-TCP control plane with QUIC for 0-RTT reconnects and better multiplexing under packet loss
- Rust crate: `quinn`
- Trade-off: QUIC moves congestion control to userspace — more tuning surface but also more CPU overhead vs. kernel TCP

#### mTLS
- Where it fits: add `rustls` + `tokio-rustls` to both the control plane and the proxy
- Each service presents a certificate; the other side verifies it — no token needed
- Trade-off: certificate rotation complexity vs. stronger identity guarantees than JWT

#### Kubernetes / Service Mesh
- Remora maps to a sidecar proxy pattern (like Envoy in Istio)
- Control plane = xDS-compatible config server
- Data plane = the TCP forwarder
- In a real deployment: each pod gets a sidecar, control plane pushes policy updates via gRPC streaming

---

## Architecture Diagram (ASCII)

```
  Client
    │
    ▼
┌─────────────┐     CheckAccess (gRPC)    ┌──────────────────┐
│  TCP Proxy  │ ────────────────────────► │  Control Plane   │
│  (Day 2)    │ ◄──────────────────────── │  (Day 1)         │
└─────────────┘     ALLOW / DENY          │  PolicyEngine    │
    │                                     │  JWT Auth        │
    │  (if ALLOW)                         │  DashMap store   │
    ▼                                     └──────────────────┘
 Backend
 Service
```

---

## Key Interview Talking Points (from this project)

| Topic | What to say |
|---|---|
| Why DashMap over Mutex<HashMap>? | Lock-free reads at the shard level — better throughput under concurrent policy lookups |
| Why tonic over raw HTTP/2? | Generated stubs, interceptor support, streaming — production gRPC without boilerplate |
| tokio::spawn vs rayon? | tokio::spawn for I/O-bound tasks; rayon for CPU-bound parallelism; mixing them requires a dedicated thread pool |
| tokio-console findings | Poll time outliers indicate tasks holding the executor too long — signal to break up work or use `yield_now()` |
| splice(2) | Kernel-level zero-copy between two file descriptors — avoids userspace buffer copy in the forwarding hot path |
| io_uring future | Async I/O submission without syscall-per-op — Zscaler's forwarding plane would benefit at very high connection counts |
