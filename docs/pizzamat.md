# Pizzamat

Pizzamat is available in the default TUI build and in MCP-enabled builds. It
uses the same MQTT broker and credentials as the rest of subraumctl, through
the existing EDI bridge. Orders remain on the Pizzamat service.

## Setup

```sh
export SUBRAUMCTL_PIZZAMAT_USER='your-existing-bot-identity'
subraumctl tui
# Or, with an MCP-enabled build:
subraumctl mcp
```

Open **Pizzamat** in the sidebar. The existing `--view` option can also open it
directly: `subraumctl tui --view pizzamat`; no feature flag is needed.
The normal `--broker`, `--username`, `--password` and their environment variables
apply to Pizzamat too. No separate broker configuration is needed.

`--pizzamat-user` is the equivalent identity flag. Identity is the exact key
Pizzamat uses for personal orders and defaults, not the MQTT login. Set it to
the same identity used by your chat bot to manage those orders. MCP uses the
startup identity for every call; tool arguments cannot select another user.
The TUI's first action edits identity.

Commands use the production `pizza-*` names regardless of how subraumctl was
built. Use `--pizzamat-debug` for a debug Pizzamat server using `puzza-*` names.
`--pizzamat-timeout` defaults to 15 seconds waiting for a reply (1–120),
allowing time for the order-start announcements. No requests are retried
automatically: a timeout can happen after an order change, so check the list
before repeating a write. MQTT connection setup uses the normal `--connect-timeout`.

To preview commands without sending them:

```sh
subraumctl --dry-run --pizzamat-user alice tui --view pizzamat --no-connect
```

The existing `--no-connect` flag starts the TUI disconnected. Live Pizzamat
requests require connection, just like other MQTT controls; press `c` to connect.

## TUI

Select an action with Up/Down and press Enter. Enter advances through text
fields; Escape cancels. All characters, including `q`/`Q`, are accepted in text
fields; Ctrl+C still quits. PgUp/PgDn scroll the response; Home returns to its
top. Safe mode confirms writes and previews the command, arguments and identity.
Reads run immediately. Pizzamat shares the TUI's MQTT connection; the normal
connect/disconnect/reconnect controls apply. Disconnecting cancels pending
requests, and reconnecting does not repeat orders.

The typical sequence is **Show delivery services → Start order → Add food to my
order → Finish my order**. **End ordering** marks the shared list as ordered.
It does not submit an order to a restaurant. **Reopen ordering** clears that
marker. **Reset everyone's orders** clears the session, including its service.

**Remove matching food** removes all personal items containing the given
case-sensitive substring. **Remove my entire order** only removes your entry.
Adding/removing food marks your order uncommitted; finish it again afterwards.
**Join** also marks an existing order uncommitted. Set your usual order per
service, then add `default` to use it. Service management supports quoted names,
descriptions and URLs; use `-` for optional empty fields.

Replies are displayed verbatim, including backend rejections such as “already
ordered”. After a write, the TUI also fetches the current orders. Reply receipt
does not itself mean the requested change was accepted; read the response.

## MCP

The `pizzamat` permission group defaults to `w`; `r` exposes only list/services,
and `x` hides and rejects all Pizzamat tools. The read tools expose participant
names and food choices, as returned by Pizzamat.

| Tool | Arguments |
|---|---|
| `pizzamat_list`, `pizzamat_services` | none |
| `pizzamat_start` | `{"service":"pizza-place"}` |
| `pizzamat_join`, `pizzamat_commit`, `pizzamat_clear` | none |
| `pizzamat_add` | `{"food":"Margherita"}` or `{"food":"default"}` |
| `pizzamat_remove` | `{"food":"Margherita"}` (substring) |
| `pizzamat_lock`, `pizzamat_unlock`, `pizzamat_reset` | none |
| `pizzamat_set_default` | `{"service":"pizza-place","food":"Margherita"}` |
| `pizzamat_service_add` | `{"name":"pizza-place","realname":"Pizza Place","description":"...","order_url":"https://...","menu_url":"https://..."}` |
| `pizzamat_service_remove` | `{"service":"pizza-place"}` |

Only `name` is required for service-add; omitted optional fields are cleared.
Responses contain `command`, `submitted`, `dry_run` and the backend's `message`.
`submitted: true` means a reply was received, not that the backend accepted the
change. `--dry-run` returns the validated command/identity/arguments without
connecting. No broker address or credentials are included in tool results.

## Protocol and development

Commands publish non-retained JSON to `edi.cmd.pizza-<operation>` (or
`edi.cmd.puzza-<operation>` in debug mode). The envelope contains `cmd`, `args`,
`user`, a unique `src: subraumctl.recv.<request-id>`, and `dst: null`.
Before publishing, subraumctl subscribes to the matching reply topic
`edi.msg.subraumctl.send.<request-id>`. Replies contain `msg`, `user` (the reply
destination), and `data`; only the message is returned to the caller. Retained
and unrelated replies are ignored. Temporary subscriptions are removed after
completion, cancellation or timeout. An absent service is reported as a reply
timeout, since an MQTT publish alone does not confirm that Pizzamat handled it.

The local `pizzamat/` and `edi-rust/` folders are ignored protocol references,
not build dependencies or bundled services. Run the normal test suites with
`cargo test` and `cargo test --features mcp`. The MQTT integration test can be
run against a disposable broker as described in `tests/pizzamat_mqtt.rs`.
