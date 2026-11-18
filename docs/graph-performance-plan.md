# Graph performance plan

## Findings

The strongest rendering suspect is the combination of gap-heavy datasets and
unconditional redraws. This is supported by the code paths below; the size of
each contribution still needs measurement with representative data.

1. **Each gap creates another full-plot rendering layer.**
   `src/tui/graphs/history.rs:38` splits samples at nulls and missing timestamps.
   `src/tui/graphs/view.rs:174` creates a Ratatui dataset for every segment.
   The locked `ratatui-widgets` version is 0.3.2. Its `Chart` calls
   `Context::marker()` for every dataset; that method finishes the previous
   layer and allocates a fresh grid, even when the marker is unchanged.
   Canvas rendering then walks the entire plot area for each layer.
   Consequently, plot-area work grows with **segment count × plot area**.
   A 600-bucket series with alternating values and nulls can produce about
   300 datasets; four power series can produce about 1,200.

2. **The graph is rebuilt after every event.**
   `src/tui/mod.rs:147–180` draws before each event-loop iteration, with a
   100 ms ticker plus MQTT, input, and worker events. Each processed event
   leads to another full render, even when history has not changed.
   Rendering runs on the same task that processes keyboard input.

3. **Range changes and section switches can wait behind obsolete reads.**
   `src/tui/graphs.rs:104` processes climate and power through one serial
   worker. `refresh_graph()` rejects new work while that domain is busy.
   A changed range is requested only after the previous response arrives
   (`:223–232`). The default HTTP timeout is three seconds, and configurable
   up to 120 seconds.

4. **Every frame repeats history-derived preparation.**
   `src/tui/graphs/view.rs:102` rebuilds a sorted room-name set for each color
   lookup. `:156–172` allocates and scans a vector of all visible values to
   calculate bounds. Legends and dataset lists are also reconstructed.

5. **Every query creates a new HTTP client.**
   `src/influx.rs:88` prevents connection-pool reuse across reads.
   History returns roughly 600 buckets per series, but that limits result
   size, not the amount of raw data the database aggregates for long windows.

## Implementation order

### 1. Establish a reproducible baseline

- Add release-mode rendering benchmarks for both domains and both layouts:
  continuous samples, realistic sparse samples, alternating null/value
  buckets, and many climate locations.
- Exercise 80×24, 120×46, and 240×80 terminals; ensure larger cases actually
  render the plots rather than the small-terminal fallback.
- Measure graph preparation, rasterization, full-frame draw/flush, frame
  frequency, and input-to-visible-response latency separately.
- Time request queueing, HTTP/body transfer, JSON decoding, and history
  conversion. Record series, point, and segment counts.
- Compare a quiet session with a burst of background MQTT events. Use
  deterministic local HTTP fixtures to separate UI cost from server latency.
- Sample live query latency for 1 minute, 1 hour, 1 day, and 30 days when the
  configured endpoint is reachable.

### 2. Remove the per-segment full-plot cost

- Implement a gap-aware plot renderer using Canvas: draw points and adjacent
  line pairs inside each segment without changing markers or finishing a
  layer between segments.
- Use at most one layer per logical series if needed to preserve overlay
  ordering and color behavior. Retain axes, legends, and auto-scaling.
- Preserve null gaps, missing-timestamp gaps, isolated points, negative power,
  authoritative totals, and stable room colors.
- Compare buffer output and benchmark results against the current renderer.
  Do not concatenate segments into one line, which would bridge missing data.

**Acceptance:** hundreds of gaps no longer cause hundreds of full-area layers.
Gap-heavy fixtures should have render cost close to continuous fixtures with
comparable point counts.

### 3. Cache plot work and coalesce redraws

- Precompute stable colors and per-series extrema when history arrives.
- Cache the rendered plot separately from changing status/age labels. Include
  history revision, viewport size, layout, metric, scroll, and theme in cache
  invalidation; invalidate axis geometry if label widths change.
- Batch ready background events with a bounded drain budget and render once
  per batch. Preserve timely input handling under sustained event traffic.
- Schedule frames only for visible changes or animation deadlines. Continue
  polling on schedule; refreshing a seconds-old label should not rasterize
  unchanged curves.
- If profiling still shows excessive point work, add display-resolution
  reduction within segments, preserving extrema and gaps.

**Acceptance:** unchanged history is not rasterized on every timer/MQTT event;
resizing, scrolling, theme changes, and metric changes update correctly.

### 4. Make history requests latest-wins

- Give climate and power independent bounded request scheduling.
- Attach generation IDs to requests/results. Replace pending range requests
  with the newest selection and cancel obsolete client-side reads where
  possible. Ignore obsolete completions without clearing current busy state.
- Briefly debounce repeated range keys; keep explicit refresh responsive.
- Keep showing the last successful graph with its actual range while loading.
- Reuse fresh data when reopening a graph; Enter remains a forced refresh.
- Reuse a long-lived HTTP client configured with the existing timeout.
  Client cancellation must not be assumed to stop database-side query work.

**Acceptance:** a slow climate read does not delay power, and the final range
selection starts without waiting for obsolete reads to finish or time out.
Tests cover rapid range changes, switching away/back, out-of-order completion,
errors, dry-run, and no-connect behavior.

### 5. Address database latency if measurements justify it

- Inspect query timings and plans for long windows and high location counts.
- If aggregation dominates, evaluate existing downsampled retention policies
  or rollups before introducing new database infrastructure.
- Treat lower bucket counts as a response/render optimization, not a guarantee
  of less database scanning. Any resolution change must preserve the shared
  TUI/MCP history contract or be explicitly separated from it.

## Completion criteria

- On documented reference hardware, target release-mode p95 graph rendering
  below 16 ms and p95 key-to-visible-feedback below 50 ms for the fixture matrix,
  including background event bursts.
- Report before/after rendering time, idle CPU, and request latency separately.
- Preserve visual/data correctness: gaps, singleton points, units, labels,
  stale history, and small-terminal behavior.
- Run the graph and shared-history tests, then `cargo test`,
  `cargo test --features mcp`, and `cargo test --features tui-mouse`.

## Dependency source checked

- [Chart rendering, ratatui-widgets v0.3.2](https://github.com/ratatui/ratatui/blob/ratatui-widgets-v0.3.2/ratatui-widgets/src/chart.rs)
- [Canvas marker/layer handling, ratatui-widgets v0.3.2](https://github.com/ratatui/ratatui/blob/ratatui-widgets-v0.3.2/ratatui-widgets/src/canvas.rs)
