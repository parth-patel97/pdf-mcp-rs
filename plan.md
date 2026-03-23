# SDS Real-Time Search Engine — Plan

## Problem
20M+ Safety Data Sheet (SDS) PDFs. Need real-time search across all of them. No vector DB. No complex indexing pipeline.

---

## Core Constraint (Physics)
Raw PDF scan at 20M scale is impossible in real-time. The minimum viable pre-step:

```
One-time:  PDF → extract text → store as flat sharded text files
Query:     SIMD scan text shards in parallel → return matching PDF paths
```

This is the only architecture that satisfies "real-time" + "no vector DB" at this scale.

---

## System Components

```
┌─────────────────────────────────────────────────────────────┐
│  1. EXTRACTOR  (one-time, offline)                          │
│     PDF → zlib decompress → parse SDS sections → .txt      │
│     Output: 20M flat text files, sharded into N partitions  │
├─────────────────────────────────────────────────────────────┤
│  2. BLOOM BUILDER  (one-time, per shard)                    │
│     Build bloom filter per shard for instant skip           │
├─────────────────────────────────────────────────────────────┤
│  3. QUERY COORDINATOR  (runtime)                            │
│     Accept query → fan out to all shard workers → merge     │
├─────────────────────────────────────────────────────────────┤
│  4. SHARD SCANNER  (runtime, hot path)                      │
│     Bloom check → SIMD scan text → return matches           │
├─────────────────────────────────────────────────────────────┤
│  5. RESULT API  (runtime)                                   │
│     Return: PDF path, matched section, snippet              │
└─────────────────────────────────────────────────────────────┘
```

---

## Where Assembly/SIMD Is Used

| Component | SIMD Role | Expected Gain |
|---|---|---|
| PDF decompression | Intel ISA-L AVX-512 | 3–6x over zlib |
| Bloom filter check | AVX2 popcount intrinsics | skip ~80% of shards |
| Text search (hot path) | `aho-corasick` AVX2 | 32 bytes/cycle vs 1 |
| CAS number parsing | AVX2 pattern match | batch validate format |

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust |
| PDF parsing | `pdf-extract` (handles CIDFont/LaTeX PDFs) |
| Decompression | `libz-ng-sys` (zlib-ng, SIMD) / Intel ISA-L via FFI |
| String search | `aho-corasick` + `memchr` (auto AVX2) |
| Bloom filter | Custom AVX2 implementation |
| Concurrency | `tokio` + `rayon` (async I/O + CPU parallelism) |
| API | `axum` |
| Storage | Flat files on NAS/S3, sharded by hash |

---

## Build Order

```
Phase 1 — Extractor
  PDF → text extraction pipeline (pdf-extract)
  Output: sharded flat text corpus

Phase 2 — SIMD Scanner
  Bloom filter (AVX2)
  aho-corasick shard scan
  Benchmark: chars/sec per core

Phase 3 — Coordinator
  Fan-out to shard workers (rayon parallel)
  Merge results, return PDF paths + snippets

Phase 4 — API
  Axum HTTP endpoint
  Query → response with matches
```

---

## Target Performance

```
Corpus:           20M PDFs × 10KB avg text = 200GB text
Per core (AVX2):  ~5 GB/s scan throughput
16 cores:         ~80 GB/s
Scan time:        ~2.5s on 1 server
10 servers:       ~250ms
+ Bloom filter:   ~50ms  ← real-time
```

---

## Out of Scope
- Vector embeddings
- Elasticsearch / Solr / Lucene
- Pre-built inverted index
- Any cloud ML service
- GPU acceleration
