# dkv — Distributed Key-Value Store

`dkv` is a distributed, replicated key-value store written in Rust. This document is the design spec: what the system does, how it's built, and the order it will be built in.

> **Status:** Early development. `Cache` (an in-memory store) and a TCP server that answers `PING` exist today. Everything else in this document is planned work.

---

## 1. Goals and Non-Goals

### Goals

- **Simple data model:** string keys mapped to string values, with `GET`, `SET`, `DEL`, `EXISTS` and a few admin commands.
- **Durability:** writes that have been acknowledged survive a process crash or restart.
- **High availability:** a cluster of `2f + 1` nodes keeps serving reads and writes while up to `f` nodes are down.
- **Strong consistency:** writes are linearizable, and reads are linearizable by default.
- **Horizontal scale:** the keyspace is split into shards that are spread across nodes.
- **Learning-friendly codebase:** each layer (storage, networking, consensus, sharding) is a separate module with its own tests.

### Non-Goals (for v1)

- Transactions that span multiple keys.
- Secondary indexes, range queries or rich data types (lists, hashes, sets).
- Replication across geographic regions.
- Authentication and TLS. These are planned after v1 (see [§10](#10-future-work)).
- Wire compatibility with Redis. The port matches Redis's, but the protocol is our own.

---

## 2. Terminology

| Term | Meaning |
|------|---------|
| **Node** | One `dkv` server process. |
| **Cluster** | A set of nodes that together serve the whole keyspace. |
| **Shard** | A contiguous range of the hash space, replicated as a unit. |
| **Replica group** | The nodes that hold copies of one shard. Each group runs its own Raft instance. |
| **Leader** | The replica that accepts writes for a shard in the current Raft term. |
| **Follower** | A replica that copies the leader's log. |
| **WAL** | Write-ahead log, the append-only on-disk record of every applied command. |

---

## 3. Data Model

- **Key:** a UTF-8 string of 1 to 512 bytes. It must not contain whitespace or newlines (a protocol limit; see §5).
- **Value:** a UTF-8 string of 0 to 1 MiB.
- **Versioning:** every key has a monotonically increasing `u64` version, which is set from the Raft log index of the write that last changed it. This makes compare-and-set possible later (§10).
- **TTL (planned, Milestone 2):** an optional expiry time for each key. Expired keys can no longer be read and are removed lazily on access and by a background sweep.

---

## 4. Architecture

```
            ┌──────────────────────────────────────────────┐
 client ──▶ │ Network layer   (tokio TCP, protocol codec)  │
            ├──────────────────────────────────────────────┤
            │ Router          (key → shard → leader)       │
            ├──────────────────────────────────────────────┤
            │ Consensus       (Raft per shard)             │
            ├──────────────────────────────────────────────┤
            │ State machine   (Cache: HashMap<K, V>)       │
            ├──────────────────────────────────────────────┤
            │ Storage         (WAL + snapshots on disk)    │
            └──────────────────────────────────────────────┘
```

### 4.1 Crate layout (target)

```
src/
  lib.rs            # re-exports
  cache.rs          # in-memory state machine (today's `Cache`)
  protocol/         # command parsing + response encoding
  server/           # TCP listener, connection handling
  storage/          # WAL, snapshots, recovery
  raft/             # consensus integration
  cluster/          # membership, shard map, routing
  config.rs         # node configuration
  bin/
    server.rs       # `dkv-server`
    cli.rs          # `dkv-cli` client
```

### 4.2 Concurrency model

- One tokio task for each client connection. Connections must never be handled one after another. Today's `server.rs` awaits each connection inline, which blocks every other client.
- The state machine sits behind a single owner task that receives work over an `mpsc` channel, so `Cache` itself has no locks. The connection task sends a command together with a `oneshot` sender for the reply.
- A `RwLock<Cache>` is an acceptable stepping stone for Milestone 1, before Raft exists.

---

## 5. Client Protocol

The protocol is text-based and line-oriented, so it can be driven by hand with `nc` or `telnet`.

### 5.1 Framing

- A request is a single line ending in `\n` (an optional `\r` before it is ignored).
- Tokens are separated by single spaces. The first token is the command name, which is case-insensitive.
- For `SET`, everything after the key is the value. The value may contain spaces but not newlines.
- A response is a single line ending in `\n`.
- One connection carries many requests. The server reads line by line and must not read to EOF (today's `read_to_string` does).

### 5.2 Commands

| Command | Response | Notes |
|---------|----------|-------|
| `PING` | `PONG` | Health check. |
| `GET <key>` | `VALUE <value>` or `NIL` | |
| `SET <key> <value>` | `OK` | Creates or overwrites the key. |
| `DEL <key>` | `1` or `0` | Whether the key existed. |
| `EXISTS <key>` | `1` or `0` | |
| `LEN` | `<n>` | Number of keys on this node or shard. |
| `CLEAR` | `OK` | Admin only. Disabled in cluster mode for v1. |
| `EXPIRE <key> <secs>` | `1` or `0` | Milestone 2. |
| `CLUSTER INFO` | multiline (see below) | Milestone 4. Returns the shard map. |

### 5.3 Errors

Every error response has the form `ERR <code> <message>`:

| Code | Meaning |
|------|---------|
| `UNKNOWN` | Unknown command. |
| `ARGS` | Wrong number or format of arguments. |
| `TOOLARGE` | The key or value exceeds its size limit. |
| `MOVED <shard> <addr>` | The key belongs to a shard led by another node. The client should retry at `<addr>`. |
| `NOLEADER` | No leader has been elected yet. The client should retry with backoff. |
| `INTERNAL` | Unexpected server error. |

A multiline response (only `CLUSTER INFO`) starts with `ARRAY <n>`, followed by `n` lines.

### 5.4 Future: binary protocol

If the text protocol becomes a bottleneck, or binary-safe values are needed, a length-prefixed binary framing (or gRPC through `tonic`) can be added on a separate port. The text protocol stays available for debugging.

---

## 6. Storage and Durability (Milestone 2)

### 6.1 Write-ahead log

- Each mutating command is appended to `data/<shard>/wal-<segment>.log` before it is applied to `Cache`.
- Each record has the layout `[len: u32][crc32: u32][payload]`, where the payload is the serialized command (`bincode` or a hand-written encoding).
- A segment rolls over to a new file at 64 MiB.
- Fsync policy is configurable:
  - `always`: fsync before acknowledging. The safest and slowest option.
  - `batch` (default): group commit every 10 ms or every N writes.
  - `never`: rely on the OS. Only for tests and benchmarks.

### 6.2 Snapshots

- A snapshot is the full `Cache` serialized to `snapshot-<last_index>.bin`, written to a temporary file and then atomically renamed into place.
- Snapshots are taken when the WAL grows past a threshold (default 256 MiB). Older WAL segments are then deleted.
- Once Raft exists, these same snapshots are what Raft sends to followers that have fallen far behind (`InstallSnapshot`).

### 6.3 Recovery

1. Load the newest valid snapshot.
2. Replay the WAL records whose index is greater than the snapshot's index.
3. Stop at the first record that is truncated or fails its CRC, and truncate the file at that point. This is a torn write from a crash.

---

## 7. Replication and Consensus (Milestone 3)

### 7.1 Algorithm

Replication uses **Raft**. The preferred implementation is the [`openraft`](https://crates.io/crates/openraft) crate rather than a from-scratch Raft. A hand-written implementation is a possible learning stretch goal, behind the same trait boundary.

- The `Cache` together with the WAL and snapshots is the Raft **state machine** and **log store**.
- Nodes talk to each other over a separate peer port (default `7379`), using gRPC through `tonic` or a simple framed bincode protocol over TCP.

### 7.2 Write path

1. A client sends `SET k v` to any node.
2. If that node isn't the leader for `k`'s shard, it replies `ERR MOVED` (§5.3).
3. The leader appends the command to its Raft log and replicates it to followers.
4. Once a majority has the entry, it is committed and applied to `Cache`, and the leader replies `OK`.

### 7.3 Read path

| Mode | Guarantee | Mechanism |
|------|-----------|-----------|
| `linearizable` (default) | Reflects all acknowledged writes. | ReadIndex: the leader confirms it is still leader with a heartbeat round, waits until it has applied up to the commit index, then serves the read locally. |
| `lease` | Linearizable as long as clocks are bounded. | The leader serves reads locally while its lease has not expired. |
| `stale` | May lag behind. | Any replica serves the read from its local state. Enabled with a per-connection `READMODE stale` command. |

### 7.4 Failure handling

- **Election timeout:** randomized between 150 and 300 ms. **Heartbeat interval:** 50 ms. Both are configurable.
- **Leader crash:** a follower is elected after the timeout. Clients that receive `NOLEADER` or a connection error retry with backoff.
- **Network partition:** only the majority side can commit, so the minority side rejects writes.

---

## 8. Sharding and Cluster Membership (Milestone 4)

### 8.1 Partitioning

- **Hash slots:** `slot = xxhash64(key) % 1024`. There are 1024 fixed slots.
- Slots are grouped into **shards**. Each shard is one Raft replica group with a replication factor of 3 by default.
- The **shard map** records which slots belong to which shard and which nodes replicate each shard. It is stored in a dedicated **metadata Raft group** (shard 0) that runs on the first 3 nodes.

### 8.2 Routing

- Any node can accept a client request. If the node doesn't lead the key's shard, it replies `ERR MOVED`.
- Clients (including `dkv-cli`) cache the shard map from `CLUSTER INFO` and send requests straight to the right leader.

### 8.3 Membership changes

- Nodes join and leave the cluster with `dkv-cli cluster add-node` and `dkv-cli cluster remove-node`. These commands are written to the metadata group.
- Replica groups change their membership with Raft joint consensus, which `openraft` provides.
- **Rebalancing (stretch):** moving a slot means streaming a snapshot of that slot's keys to the destination shard, then switching ownership atomically in the shard map.

---

## 9. Configuration, Operations and Testing

### 9.1 Configuration

The node reads `dkv.toml`. CLI flags and `DKV_*` environment variables override it.

```toml
node_id      = 1
client_addr  = "127.0.0.1:6379"
peer_addr    = "127.0.0.1:7379"
data_dir     = "./data"
fsync        = "batch"          # always | batch | never
seeds        = ["127.0.0.1:7379", "127.0.0.1:7380", "127.0.0.1:7381"]

[raft]
heartbeat_ms        = 50
election_timeout_ms = [150, 300]
snapshot_threshold  = 268435456
```

### 9.2 Observability

- Logging uses `tracing` with `tracing-subscriber`, structured and with the level controlled by `RUST_LOG`.
- A Prometheus endpoint at `/metrics` reports ops/sec per command, p50/p99 latency, Raft term, commit index, applied index, WAL size and connection count.

### 9.3 Testing strategy

| Level | What | Where |
|-------|------|-------|
| Unit | `Cache`, protocol parser, WAL encoding and decoding. | `src/**` `#[cfg(test)]` |
| Integration | Real TCP server and client, plus crash-and-recover tests. | `tests/` |
| Cluster | 3- and 5-node clusters run in-process. Kill leaders, partition nodes, check results. | `tests/cluster_*.rs` |
| Linearizability | Record operation histories and check them with a Jepsen-style checker such as [`stateright`](https://crates.io/crates/stateright) or Porcupine-style analysis. | Stretch goal |
| Property / fuzz | `proptest` for the parser and the WAL, `cargo fuzz` for the protocol. | |

CI (`.github/workflows/`) already runs `fmt`, `clippy -D warnings`, `build`, and `test` on Linux, macOS and Windows. Every milestone must keep all of these passing.

### 9.4 Performance targets (single node, loopback, v1)

- Over 100k `GET` ops/sec and over 50k `SET` ops/sec with `fsync = batch`.
- p99 latency under 1 ms for `GET` and under 5 ms for replicated `SET` on a 3-node local cluster.

These numbers will be tracked with a `criterion` benchmark and a small load-generator binary.

---

## 10. Roadmap

| Milestone | Scope | Done when |
|-----------|-------|-----------|
| **0. Core** ✅ | In-memory `Cache` with `get`, `set`, `delete`, `exists`, `len` and `clear`. CI. | Done. |
| **1. Single-node server** | Line protocol (§5), all basic commands, one task per connection, shared state, `dkv-cli`, config file, `tracing`. | `nc localhost 6379` supports every §5.2 command, and integration tests cover them. |
| **2. Durability** | WAL, snapshots, recovery, TTL. | A server killed with `kill -9` loses no acknowledged writes. |
| **3. Replication** | `openraft` integration, peer RPC, leader redirects, ReadIndex reads. | A 3-node cluster survives the loss of any single node, and the cluster tests pass. |
| **4. Sharding** | Hash slots, metadata group, multiple Raft groups, `CLUSTER INFO`, smart client. | Keys spread across several shards, and adding a node works. |
| **5. Hardening** | Metrics, benchmarks, fuzzing, linearizability checks, docs. | The §9.4 targets are met. |

### Future work

- Auth (password or token) and TLS on both the client and peer ports.
- Compare-and-set (`CAS <key> <version> <value>`) using per-key versions.
- Watch and subscribe on keys.
- A pluggable storage engine, for example `sled` or `rocksdb`, for datasets larger than memory.
- Automatic rebalancing of slots.

---

## 11. Open Questions

1. **Raft:** use `openraft`, or write it from scratch for learning? This spec assumes `openraft` behind a trait so the choice can be revisited.
2. **Peer transport:** `tonic` (gRPC) brings protobuf tooling, while framed bincode over TCP has fewer dependencies. Which is preferred?
3. **Values:** stay with UTF-8 `String`, or switch to `Vec<u8>` / `bytes::Bytes` now, before the WAL format is fixed? Switching later means migrating the on-disk format.
4. **Metadata group:** keep a separate Raft group, or start with a static shard map from config for Milestone 4?

---

## Getting Started

```sh
# run tests
cargo test

# run the demo binary
cargo run

# run the server (listens on 127.0.0.1:6379)
cargo run --bin server

# in another terminal
echo PING | nc -q1 127.0.0.1 6379   # → PONG
```

## License

TBD.
