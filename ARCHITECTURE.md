# Kidur Architecture

## Crate dependency graph

```
kidur-cli  ─►  kidur-log      ─►  kidur-core
               kidur-supertag  ─►  kidur-core
               kidur-crdt      ─►  kidur-core
```

`kidur-core` has zero intra-workspace deps. It is the stable foundation everything builds on. Never introduce a back-edge.

## Data model

- **Node**: id, parent_id, sort_order, content, supertag, fields, visibility, timestamps
- **Edge**: typed reference between nodes (`owner`, `references`, `blocks`, …)
- **Supertag**: TOML-defined schema with named, typed fields
- **FieldValue**: `Text | Number | Bool | Reference | Geo | Timestamp | Email | Url | Enum | MultiSelect | RichText`

## Storage

The `.jsonl` mutation log is the canonical source of truth. Indexes are disposable and rebuilt from it on startup — the same pattern as Obsidian (plain `.md` files) and Logseq classic.

```
kidur.jsonl
  └─ LogEntry { seq: u64, ts: DateTime<Utc>, mutation: Mutation }
       └─ Mutation: CreateNode | UpdateNode | DeleteNode | CreateEdge | DeleteEdge
```

**Write path:** validate against supertag registry → append `LogEntry` → update in-memory index

**Read path:** query in-memory index (no I/O)

## CRDT

Loro handles per-node rich text and movable-tree snapshots. Snapshots are serialized to bytes and stored as a field on the node. The `CrdtDoc` trait in `kidur-crdt` decouples the rest of the codebase from Loro directly.

## Out of scope (v0.2)

- Auth, capabilities, UCAN
- Federation, cross-instance sync
- HTTP API (planned v0.3)
- Rich UI (TipTap / ProseMirror)
