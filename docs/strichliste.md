# Strichliste 0.11.0

Optional CLI and TUI integration for the deployed legacy 0.11.0 variant, including its
transfer/comment extension and `weightedCountOfPurchases` user field.

## Build and configure

```sh
cargo build --release --features strichliste
# Or include restricted MCP tools:
cargo build --release --features mcp,strichliste
```

Nix packages: `.#subraumctl-strichliste` and `.#subraumctl-mcp-strichliste`.
The same `strichliste` feature enables both CLI commands and the TUI tab.
Default builds do not include Strichliste commands, the tab, or connection options.

Supply the API base URL and **one** account selector:

```sh
export SUBRAUMCTL_STRICHLISTE_URL='https://your-server/strich-api'
export SUBRAUMCTL_STRICHLISTE_USER_ID=27
# Alternatively, instead of USER_ID:
# export SUBRAUMCTL_STRICHLISTE_USER='your username'
```

Equivalent global flags are `--strichliste-url`, `--strichliste-user-id` and
`--strichliste-user`. Names are matched exactly, case-insensitively; ambiguous
names fail and require a numeric id. Numeric names work with `--strichliste-user`.
The endpoint has no built-in default and is used only on a Strichliste operation.
HTTP requests time out after five seconds; override with `--strichliste-timeout`
or `SUBRAUMCTL_STRICHLISTE_TIMEOUT` (1–120 seconds).

## CLI

```sh
subraumctl strichliste balance
subraumctl strichliste rank
subraumctl strichliste pay 1.25 --comment 'Mate'
subraumctl strichliste add 10 --comment 'cash deposit'
subraumctl strichliste send 0.01 --comment 'test'
subraumctl strichliste pay 3.141592653589793
```

Amounts are positive numeric **euros**. `pay` debits, `add` credits, and `send`
debits your account and credits the selected recipient in a single API request.
Fractional cents are preserved as floating-point values, not rounded to two
decimal places; `pi` is not a literal expression understood by the CLI.
Zero, negative, non-finite and overflowing amounts are rejected before IO.

`send` requires a terminal: enter part of a recipient's name, then choose their
number from the results. Your own account is excluded. Enter `0` at the number
prompt to search again, or empty input/EOF at either prompt to cancel without a
transaction. More than 20 matches require a narrower search.

Rank is `1 + number of users with a strictly higher weightedCountOfPurchases`.
Ties share a rank (1, 2, 2, 4); balances do not affect rank. Missing/invalid scores
fail the operation instead of producing an incomplete rank.

`--json` emits a JSON result; `--quiet` suppresses human results. Interactive
prompts go to stderr. `--dry-run` performs no HTTP requests, including no user
lookup: it previews values/comments, with recipient selection left unresolved.

## TUI

With the same URL and account environment variables configured:

```sh
cargo build --features strichliste
./target/debug/subraumctl tui --view strichliste
```

Choose **Strichliste** in the sidebar or start directly with `--view strichliste`.
The tab shows your balance in euros and your rank by weighted purchase count.
It reads once when first opened; `--no-connect` suppresses that initial read.
**Refresh balance and rank** works manually, independently of MQTT.

- Use **Up/Down**, then **Enter** to choose **Pay**, **Add credit**, or **Send**.
- Pay/add prompt for a positive euro amount and an optional comment.
- Send prompts for part of a recipient's name. Select a match with
  **Up/Down**, **PageUp/PageDown**, **Home/End**, then **Enter**. Your own account is
  excluded. **/** starts a new search. Then enter the amount and comment.
- **Esc** cancels a form without submitting. Text fields accept global shortcut
  characters as literal text; **Ctrl+C** still quits.
- With the application's default safe mode enabled, a confirmation previews the
  amount, comment and recipient before submitting. Cancel is selected by default.
  Disabling safe mode makes completion of the comment field submit immediately.

Requests run in the background; repeated activation while busy does not enqueue
duplicate payments. Successful writes refresh balance and rank. Failed reads
keep previous values labelled `(previous)`. Unconfirmed writes are never retried.
`--dry-run` previews operations without HTTP or invented balances/search results.

TUI payments use your comment and the existing safe-mode setting; the MCP write
opt-in and forced MCP comment apply only to MCP.

## Restricted MCP

Build with `--features mcp,strichliste` and supply the same account configuration.
The `strichliste` permission group defaults to `r`:

| Tool | Arguments | Result |
|---|---|---|
| `strichliste_balance` | `{}` | `balance_eur` |
| `strichliste_rank` | `{}` | `rank`, `weighted_count_of_purchases` |
| `strichliste_pay` | `{"amount":1.25}` | `submitted`, signed `value_cents` |
| `strichliste_add` | `{"amount":10}` | `submitted`, signed `value_cents` |

Pay and add are **disabled by default**. Enable them with both:

```toml
[tools]
strichliste = "w"
```

```sh
subraumctl --strichliste-mcp-writes mcp --config permissions.toml
# Environment alternative: SUBRAUMCTL_STRICHLISTE_MCP_WRITES=true
```

Both writes always set `comment` to exactly `triggered by mcp`. Tool arguments
cannot change the account, endpoint or comment, or add recipient fields.
Transfers, user search/listing and transaction history are never registered.
Disabled tools are hidden from discovery and rejected on direct invocation.
`strichliste = "x"` also disables balance and rank.

The backend may read the user list to resolve your name or calculate rank; only
your balance/rank fields reach MCP. Raw user and transaction responses are not
returned, and HTTP errors omit response bodies and endpoint details.
`--dry-run` also supports MCP and preserves the same permissions.

## API contract and verification

The implementation uses the legacy `/user` routes, not the newer PHP API:

- `GET <base>/user`: an object with `overallCount`, `limit`, `offset`, and an
  `entries` array of user objects (`id`, `name`, `balance`,
  `weightedCountOfPurchases`). `limit` and `offset` may be null for an unpaginated
  response. Bare arrays from older variants are also accepted.
- Paginated lists are fetched with `limit`/`offset` until `overallCount` entries
  have been read. Inconsistent counts, offsets, duplicate ids, or stalled pages
  fail instead of giving incomplete ranks or ambiguous account matches. The
  configured HTTP timeout also bounds the complete list fetch.
- `GET <base>/user/<id>`: one user object; `balance` is already in **euros** and
  is preserved without conversion or rounding.
- `POST <base>/user/<id>/transaction`: JSON with `value` in **cents** and `comment`.
  A successful 2xx response acknowledges the submission.

Numeric balances and scores may also be returned as numeric strings.
Rejected reads report the relative route and HTTP status without response bodies.
Based on the provided deployed-server example, `pay 0.01` posts `value: -1`.
A one-cent transfer from user 27 to user 30 uses:

```json
{
  "value": -1,
  "comment": "to Other: test",
  "toUserId": 30,
  "toUserComment": "from Me: test"
}
```

Comments use the account names returned by the API. There is no client-side
two-transaction transfer fallback. Automatic retries and redirects are disabled.
On an unconfirmed submission, check the account before retrying.

Tests use local HTTP fixtures, the supplied user-list wrapper and balance sample,
and the earlier transfer contract. The private deployment has not been contacted.
