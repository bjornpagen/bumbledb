# PRD 13: Set-First Encoded Sinks

## Purpose

Make set semantics cheaper and more explicit by deduplicating encoded projected facts before decoding values.

## Current Problem

`ProjectionSink::consume` decodes projected values and inserts `Vec<Value>` into a set. This repeatedly decodes dictionary values even when duplicate witnesses project to the same fact.

## Required Design

- Default sink stores encoded projected facts in a set.
- Decode values only during final `QueryResultSet` materialization.
- Preserve canonical result ordering.
- Count logical bindings consumed, encoded projected facts inserted, duplicates suppressed, values decoded, and decode time.
- Keep sink/fold boundary private.

## Required Semantics

- Duplicate witnesses never multiply public output.
- Existential variables never multiply projected output.
- Set deduplication happens on encoded typed bytes for speed, but final output still uses typed `Value`.
- String and bytes decoding must use LMDB dictionary visible to the read snapshot.

## Breaking Direction

- Replace materialized sink as default.
- Keep factorized projection only if it is clearly distinct and tested against encoded set sink.
- Do not expose aggregate hooks publicly.

## Passing Criteria

- Duplicate witness tests prove one projected fact remains one fact.
- Dictionary string/bytes projection tests pass.
- Trace shows decode count equals final projected cell count, not logical witness count.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
