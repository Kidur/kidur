# Kidur API Specification v0.1

> 𒆠𒆕 ki-dur — "foundational binding structure"
> First Layer: Integrated Intelligence Information System
> Status: draft

## 1. Principles (from architectural notes)

These are non-negotiable. Every endpoint must honor them.

1. **No new data silos** — kidur federates existing data, never traps it
2. **Local-first** — all operations work offline; sync is eventual
3. **Privacy-first, EU sovereign** — data stays in EU jurisdictions (Hetzner, Cloudscale.ch), never Oracle
4. **Forkable** — any instance can fork, diverge, and optionally re-merge
5. **Multi-hierarchical** — a node can belong to multiple hierarchies simultaneously (key differentiator from Tana and existing tools)
6. **Immutable + mutable layers** — original content is immutable; annotations, corrections, and AI enrichment layer on top
7. **Folders are nodes** — a folder is a node that can be referenced, made into a text file, which can be a node in an outliner

## 2. Core Concepts

### 2.1 Node (Nucleus)

The fundamental unit. Everything is a node — a paragraph, a word, a highlight, a folder, a file, a person, a project.

```
Node {
  kid_id:      BlockID        // federated UUID (v4 Logseq-native, or v7 Kidur-originated)
  native_id:   string         // system-native ID (may equal kid_id for Logseq; differs for Tana)
  native_sys:  string         // "logseq" | "tana" | "silknotes" | "kidur"
  created_at:  Timestamp      // authoritative for federation (earliest wins)
  content:     CRDT<Text>     // Loro CRDT, sub-node granularity (per-character/per-word)
  // NOTE: with Loro, LoroTree handles structural moves natively at CRDT level — no SQL LWW table needed
  // LoroText manages text; LoroTree manages structure. Both are conflict-free at CRDT level.
  parents:     [BlockID]      // multi-hierarchical: a node has N parents (read from graph)
  children:    [BlockID]      // ordered child list (read from graph)
  tags:        [TagID]        // supertags (like Tana's #tag system)
  fields:      Map<FieldID, FieldValue>  // tag-defined schema fields
  layer:       "immutable" | "mutable"   // SenseMeet-inspired layering
  origin:      OriginURI      // where this node was first created (logseq://..., tana://..., silknotes://...)
  refs:        [BlockID]      // bidirectional references
  holon:       HolonID?       // which holon this belongs to (optional)
}
```

### 2.2 BlockID (Federated Identity)

```
BlockID = UUID v4 (for Logseq-native blocks)
         | UUID v7 (for Kidur-originated blocks — time-sortable, RFC 9562)

Federation rules:
  1. Earliest wins — same kid_id in two registries: earlier created_at is authoritative
  2. Collision → fresh UUID v7 assigned as new kid_id; canonical_for pointer set to winner
     (no suffix model — Tana native IDs are not UUIDs, suffix would break them)
  3. Two-column model — kid_id (always UUID) + native_id (system-native string, e.g. Tana's "2Tk7owX1Q9Vt")
  4. Registry lookup — kidur resolves kid_id OR (native_system, native_id) → canonical location
```

Generation:
- Kidur-originated: `pg_uuidv7` extension (`SELECT gen_uuid_v7()`) or Rust `uuid` crate v1+ with `new_v7()`
- Logseq-native: UUID is its own kid_id — zero overhead
- Tana-native: Tana's short alphanumeric ID stored as `native_id`; Kidur assigns UUID v7 as `kid_id` on first registration

### 2.3 Holon

A self-contained organizational unit with its own identity, infrastructure, and namespace.

```
Holon {
  id:           HolonID
  name:         string
  crypto_addr:  PublicKey      // each holon has a crypto address
  parent:       HolonID?       // holons nest fractally
  children:     [HolonID]
  root_node:    BlockID        // the root node of this holon's tree
  domains:      [string]       // e.g. ["kidur.org", "evobiosys.org"]
  channels:     ChannelConfig  // Matrix room, email, git repo per holon
}
```

Spawning a holon creates: workspace, folder structure, Matrix room, domain registration, git repo.

### 2.4 Tag (Supertag)

```
Tag {
  id:       TagID
  name:     string            // e.g. "project_holon", "simple_sub_system"
  fields:   [FieldDef]        // schema: field name, type, constraints
  extends:  [TagID]           // tag inheritance
}

FieldDef {
  name:     string
  type:     "plain" | "number" | "date" | "url" | "email" | "checkbox"
          | "instance" | "options" | "crdt_text"
  required: bool
}
```

## 3. Data Layer

### 3.1 Storage: Apache AGE (Cypher + SQL over PostgreSQL)

Nodes and edges stored in a property graph. AGE provides Cypher queries over Postgres, giving both graph traversal and relational guarantees.

```sql
-- Graph schema
SELECT * FROM cypher('kidur', $$
  CREATE (n:Node {
    id: 'abc-123',
    content: '...',
    created_at: timestamp('2026-04-05T10:00:00Z'),
    layer: 'immutable',
    origin: 'logseq://graph/HolonicStructure?block-id=abc-123'
  })
$$) AS (v agtype);

-- Multi-hierarchical parent edges
SELECT * FROM cypher('kidur', $$
  MATCH (parent:Node {id: 'parent-1'}), (child:Node {id: 'abc-123'})
  CREATE (parent)-[:CHILD_OF {order: 0}]->(child)
$$) AS (e agtype);

-- Cross-reference edges (bidirectional)
SELECT * FROM cypher('kidur', $$
  MATCH (a:Node {id: 'abc-123'}), (b:Node {id: 'def-456'})
  CREATE (a)-[:REFS]->(b), (b)-[:REFS]->(a)
$$) AS (e agtype);

-- Tag application
SELECT * FROM cypher('kidur', $$
  MATCH (n:Node {id: 'abc-123'}), (t:Tag {name: 'project_holon'})
  CREATE (n)-[:TAGGED]->(t)
$$) AS (e agtype);
```

### 3.1b Structural Ops Layer (SQL — NOT Yrs)

Structural changes (reparent, reorder) use a separate SQL table with Last-Writer-Wins on Lamport clocks.
Yrs does not safely handle concurrent tree reparenting — concurrent moves can create cycles.

```sql
CREATE TABLE structural_ops (
    id            UUID PRIMARY KEY DEFAULT gen_uuid_v7(),
    node_kid_id   UUID NOT NULL REFERENCES block_registry(kid_id),
    actor         TEXT NOT NULL,          -- DID key
    lamport       BIGINT NOT NULL,
    new_parent_id UUID REFERENCES block_registry(kid_id),
    order_frac    DOUBLE PRECISION,       -- fractional index (between siblings)
    applied_at    TIMESTAMPTZ DEFAULT now(),
    dropped       BOOLEAN DEFAULT false,
    drop_reason   TEXT                    -- "would_create_cycle" | null
);
-- Before applying: check is_ancestor(new_parent, node) to detect cycles
-- Higher Lamport wins; concurrent cycle-creating moves are dropped deterministically
```

Block registry (two-column identity model):
```sql
CREATE TABLE block_registry (
    kid_id        UUID PRIMARY KEY,       -- always a UUID (v4 or v7)
    native_id     TEXT NOT NULL,          -- system-native string
    native_system TEXT NOT NULL CHECK (native_system IN ('logseq','tana','obsidian','silknotes','kidur')),
    native_graph  TEXT,                   -- Logseq graph name, Tana workspace ID
    created_at    TIMESTAMPTZ NOT NULL,   -- from origin system (authoritative for earliest-wins)
    registered_at TIMESTAMPTZ DEFAULT now(),
    authoritative BOOLEAN DEFAULT true,
    canonical_for UUID REFERENCES block_registry(kid_id),
    holon_id      TEXT,
    UNIQUE (native_system, COALESCE(native_graph,''), native_id)
);
```

### 3.2 CRDT Layer: Loro (Rust)

Sub-node granularity. A highlight, a word, a character — each can be a nucleus in the graph.
Scope: text content via `LoroText` (Fugue algorithm); tree structure via `LoroTree` (native move with Kleppmann cycle prevention).

Why Loro over Yrs: `LoroTree` has native MovableTree semantics — concurrent reparenting is handled at the CRDT level without a separate SQL structural ops table. Yrs has no built-in tree move.

```rust
// Per-node CRDT document
struct NodeDoc {
    doc: loro::LoroDoc,
    text: loro::LoroText,     // node text content (Fugue algorithm)
    tree: loro::LoroTree,     // structural hierarchy (native move, cycle-safe)
    // granularity: character-level awareness
    // when a user highlights text → that range becomes a new node
    // linked back to the parent node with character offsets
}

// Highlight → nucleus promotion
fn promote_to_nucleus(parent: &BlockID, range: Range<u32>) -> BlockID {
    let new_id = generate_block_id();
    // Extract text from CRDT range
    // Create new LoroTree node with parent reference
    // Insert REFS edge: parent <-> new_node
    new_id
}
```

### 3.3 Sync Protocol

```
                   ┌──────────┐
                   │  kidur   │  ← federation registry
                   │ (AGE+Loro)│
                   └────┬─────┘
                        │
          ┌─────────────┼─────────────┐
          │             │             │
    ┌─────┴─────┐ ┌─────┴─────┐ ┌─────┴─────┐
    │  Logseq   │ │ SilkNotes │ │  Other    │
    │  (graph)  │ │ (Flutter) │ │  clients  │
    └───────────┘ └───────────┘ └───────────┘

Sync flow:
  1. Client pushes changed nodes (BlockID + CRDT state vector)
  2. kidur checks BlockID registry for conflicts (earliest wins)
  3. kidur merges CRDT states (Loro merge is automatic/conflict-free)
  4. kidur broadcasts updates to subscribed clients
  5. Bidirectional refs are maintained: if A refs B, B refs A
```

## 4. API Endpoints

Base: `https://{instance}.kidur.org/api/v1` or `http://localhost:{port}/api/v1`

### 4.1 Nodes

```
POST   /nodes                    Create node(s)
GET    /nodes/{id}               Get node by BlockID
PATCH  /nodes/{id}               Update node (CRDT merge)
DELETE /nodes/{id}               Soft-delete (immutable layer preserved)

GET    /nodes/{id}/children      List children (ordered)
GET    /nodes/{id}/parents       List parents (multi-hierarchical)
GET    /nodes/{id}/refs          List bidirectional references
GET    /nodes/{id}/history       CRDT version history

POST   /nodes/{id}/promote       Promote a text range to a nucleus
POST   /nodes/{id}/refs          Add bidirectional reference
DELETE /nodes/{id}/refs/{ref_id} Remove reference
```

#### Create Node

```http
POST /nodes
Content-Type: application/json

{
  "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",     // optional, server generates if omitted
  "content": "Kidur is the foundation",
  "parents": ["parent-node-id"],                       // multi-hierarchical
  "tags": ["project_holon"],
  "fields": {
    "status": "active",
    "lead": "alice@example.org"
  },
  "origin": "logseq://graph/HolonicStructure?block-id=6ba7b810-...",
  "layer": "immutable"
}
```

Response:
```json
{
  "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "created_at": "2026-04-05T10:00:00Z",
  "federation": {
    "status": "registered",
    "authoritative": true,
    "registry": "kidur.org"
  }
}
```

#### Get Node (with depth)

```http
GET /nodes/{id}?depth=3&include=refs,tags,fields
```

Response:
```json
{
  "id": "6ba7b810-...",
  "content": "Kidur is the foundation",
  "created_at": "2026-04-05T10:00:00Z",
  "parents": ["parent-id-1", "parent-id-2"],
  "children": [
    {
      "id": "child-1",
      "content": "Sub-node content",
      "children": [ ... ]
    }
  ],
  "tags": [{"id": "tag-1", "name": "project_holon"}],
  "fields": {"status": "active"},
  "refs": [{"id": "ref-1", "direction": "bidirectional"}],
  "origin": "logseq://...",
  "layer": "immutable",
  "crdt_version": "sv:AQ..."
}
```

### 4.2 Federation

```
POST   /federation/register           Register a BlockID (claim ownership)
GET    /federation/resolve/{id}       Resolve kid_id → origin + canonical location
GET    /federation/resolve/native     Resolve (native_system, native_id) → kid_id
POST   /federation/push               Push local changes to registry
POST   /federation/pull               Pull remote changes since state vector
GET    /federation/state-vector/{holon}  Current state vector for bootstrap (Phase 0)
GET    /federation/conflicts          List unresolved conflicts
POST   /federation/merge              Force-merge a conflict
```

#### Register Block

```http
POST /federation/register
Content-Type: application/json

{
  "id": "6ba7b810-...",
  "origin": "logseq://graph/HolonicStructure?block-id=6ba7b810-...",
  "created_at": "2026-04-05T10:00:00Z",
  "holon": "evobiosys"
}
```

Response (no conflict):
```json
{
  "status": "registered",
  "authoritative": true
}
```

Response (conflict — earliest wins):
```json
{
  "status": "conflict",
  "resolution": "new_kid_id",
  "your_original_id": "6ba7b810-...",
  "your_new_kid_id": "019526ab-3f2c-7a89-b4d1-2c6e8f1a0d3b",
  "authoritative_kid_id": "6ba7b810-...",
  "authoritative_origin": "tana://node/xNS_ukHt-ANl",
  "authoritative_created_at": "2026-03-01T08:00:00Z"
}
```
(Suffix model removed — incompatible with Tana's non-UUID native IDs. Losers get a fresh UUID v7.)

#### Push (CRDT Sync)

```http
POST /federation/push
Content-Type: application/octet-stream
X-Kidur-Origin: logseq://graph/HolonicStructure
X-Kidur-State-Vector: base64(sv)

[Loro binary frame]
```

The server merges the Loro update, checks for new BlockIDs, resolves conflicts, and returns:

```json
{
  "merged": 42,
  "conflicts": 0,
  "new_registrations": 3,
  "server_state_vector": "base64(sv)"
}
```

### 4.3 Holons

```
POST   /holons                   Spawn a new holon
GET    /holons/{id}              Get holon details
PATCH  /holons/{id}              Update holon metadata
GET    /holons/{id}/tree         Get holon's node tree
POST   /holons/{id}/fork         Fork a holon (forkable principle)
POST   /holons/{id}/merge        Merge a forked holon back
```

#### Spawn Holon

```http
POST /holons
Content-Type: application/json

{
  "name": "silknotes",
  "parent": "evobiosys",
  "spawn": {
    "matrix_room": true,        // create Matrix room
    "git_repo": true,           // create git repository
    "domain": "silknotes.one",  // register domain association
    "folder_structure": [       // holonic file structure
      "holonic/",               // just words, no assets
      "code/",
      "assets/",
      "chronology/",
      "backup/"
    ]
  }
}
```

### 4.4 Tags

```
GET    /tags                     List all tags (supertags)
POST   /tags                     Create tag with schema
GET    /tags/{id}/schema         Get tag field definitions
PATCH  /tags/{id}/schema         Update tag schema
GET    /tags/{id}/instances      List all nodes with this tag
```

### 4.5 Search & Query

```
POST   /query/cypher             Raw Cypher query (power users)
POST   /query/search             Full-text + graph search
GET    /query/nuclei             List all nuclei (promoted highlights) across interfaces
```

#### Search

```http
POST /query/search
Content-Type: application/json

{
  "text": "federation",
  "tags": ["project_holon"],
  "holon": "kidur",
  "depth": 2,
  "include_refs": true,
  "limit": 50
}
```

#### Cypher (direct graph query)

```http
POST /query/cypher

{
  "query": "MATCH (n:Node)-[:TAGGED]->(t:Tag {name: 'project_holon'}) WHERE n.holon = 'evobiosys' RETURN n ORDER BY n.created_at",
  "params": {}
}
```

### 4.6 Interop Bridges

Bridges translate between kidur's node format and tool-native formats.

```
GET    /bridges                  List available bridges
POST   /bridges/logseq/import    Import Logseq graph (EDN → nodes)
POST   /bridges/logseq/export    Export nodes → Logseq graph
POST   /bridges/tana/import      Import Tana paste format
POST   /bridges/tana/export      Export nodes → Tana paste format
POST   /bridges/markdown/import  Import markdown folder → node tree
POST   /bridges/markdown/export  Export node tree → markdown folder
```

#### Logseq Import

```http
POST /bridges/logseq/import
Content-Type: application/json

{
  "graph_path": "/path/to/logseq/graph",
  "holon": "kidur",
  "register_ids": true,         // register all block IDs in federation
  "preserve_origins": true      // set origin to logseq:// URIs
}
```

#### Tana Import (Tana Paste format)

```http
POST /bridges/tana/import
Content-Type: text/plain

- Node content #tag
  - Field:: value
  - Child node
    - Grandchild
```

## 5. Auth Model

Inspired by UCAN (User-Controlled Authorization Networks, from Fission).

```
UCAN {
  issuer:     DID             // decentralized identifier (holon's crypto key)
  audience:   DID             // who this token is for
  capabilities: [
    { resource: "kidur://nodes/*", action: "read" },
    { resource: "kidur://holons/evobiosys/*", action: "write" },
    { resource: "kidur://federation/*", action: "register" }
  ]
  expiry:     Timestamp
  proof:      [UCAN]          // delegation chain
}
```

No central auth server. Each holon is its own authority. Delegation chains allow a holon to grant sub-permissions to collaborators without a central identity provider.

```
         holon:evobiosys (root key)
              │
    ┌─────────┼──────────┐
    │         │          │
  holon:    holon:     holon:
  kidur     idea2life  soaro
    │
  user:jakob (delegated write on kidur://nodes/*)
```

## 6. Propagation (VoT Pipeline Integration)

For the "edit once, propagate everywhere" vision:

```
POST   /propagate/{node_id}     Trigger propagation of a node
GET    /propagate/status         Check propagation queue

Propagation flow:
  Tana (VoT source)
    → kidur sync (CRDT merge)
      → Jekyll site generation
        → Postiz + n8n distribution
          → ActivityPub / Mastodon (POSSE)
```

Each project holon can have its own propagation config:
```json
{
  "holon": "evobiosys",
  "targets": [
    {"type": "jekyll", "repo": "evobiosys.org", "branch": "main"},
    {"type": "activitypub", "instance": "mastodon.social", "account": "@evobiosys"},
    {"type": "caldav", "server": "infomaniak", "calendar": "evobiosys"}
  ]
}
```

## 7. Implementation Phases

### Phase 0: Federation Registry (unblocks logseq-plugin)
- BlockID registration and resolution
- Earliest-wins conflict detection
- REST API for register/resolve/push/pull
- SQLite or Postgres backend (single instance)

### Phase 1: Node CRUD + Graph
- Apache AGE setup
- Node creation, multi-hierarchical parents, bidirectional refs
- Tag/field schema
- Cypher query endpoint

### Phase 2: CRDT Sync
- Loro integration for node content and tree structure
- Push/pull sync protocol
- State vector management

### Phase 3: Bridges
- Logseq import/export (EDN ↔ nodes)
- Tana import (Tana Paste format)
- Markdown import/export

### Phase 4: Holons + Auth
- Holon spawning (Matrix, git, domain)
- UCAN auth delegation
- Fork/merge operations

### Phase 5: Propagation
- VoT pipeline integration
- Jekyll, ActivityPub, CalDAV targets

## Appendix: Key Tana Node References

| Concept | Node ID | Location |
|---------|---------|----------|
| kidur project holon | `Se5YQIKdkQ-g` | Launchpad > SingularStructure > EvoBioSys |
| Published one-pager | `9gJ1UO2QNqNr` | Kidur's First Layer |
| kidur Alpha vision | `NInF1Dzp4zh7` | Full architecture vision |
| Chosen stack (AGE+Loro) | `tFPMeX4j6Buh` | Technical decisions |
| Feature collection | `2jbx3dYtA4H2` | Per-character granularity |
| Integration needs | `EGiUqQUI3stI` | Multi-hierarchical differentiator |
| SenseMeet layers | `b-Z-59wGxiTD` | Immutable/mutable architecture |
| Holonic organizing | `ehJfYvXrnuaz` | Holon spawning workflow |
| Folders are nodes | `gEBO8X0zjOIi` | Key architectural principle |
| Block ID federation | `j3xjpvBXGEZv` | logseq <> tana sync |
| Funding research | `Znnvk6HBYrjO` | NGI Zero, Sovereign Tech Fund |
| NGI Zero deadline | `IgWCps53oMCQ` | June 1, 2026 |
