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
top. Pizzamat actions run immediately, including when safe mode is on. For
actions with fields, the final Enter submits the command; Escape cancels the
unfinished form. Pizzamat shares the TUI's MQTT connection; the normal
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

## Checking a timeout locally

Use `mosquitto_sub` and `mosquitto_pub` (the `mosquitto-clients` package on many
distributions) against the same MQTT broker as subraumctl. The commands below
use the default host/port; add your normal authentication/TLS options if needed.
This checks the deployed bridge rather than the simulated service used by tests.

Start the subscriber first, in terminal A. `-R` ignores retained messages and
`-W 30` stops after 30 seconds. The broad subscription also reveals replies
that arrive under a different topic mapping:

```sh
mosquitto_sub -h mqtt.subraum.c3pb.de -p 1883 -t '#' -v -R -W 30
```

In terminal B, send a read-only list request:

```sh
mosquitto_pub -h mqtt.subraum.c3pb.de -p 1883 \
  -t 'edi.cmd.pizza-list' \
  -m '{"cmd":"pizza-list","args":"","user":"joni","src":"subraumctl.recv.bridgecheck","dst":null}'
```

The client currently expects a reply on `edi.msg.subraumctl.send.bridgecheck`
containing JSON with `msg` and `user: "subraumctl.send.bridgecheck"`.

- **The command appears, followed by that reply:** the round trip works. Check
  the TUI traffic detail for its actual command, `src`, and reply topic.
- **A reply arrives on a different topic:** the client's topic mapping needs
  to match that route. Record the exact topic and payload shape.
- **Only the outgoing command appears:** MQTT accepted the publish, but that
  does not prove delivery through the bridge. Repeat the same read-only request
  using `-t 'edi/cmd/pizza-list'` to check slash-separated command routing. Start
  the subscriber again if its 30-second window has expired.
- **Neither command nor reply appears:** check that both terminals use the
  same broker/vhost and credentials, and check subscription/publish permissions
  (`-d` enables Mosquitto client diagnostics).

For a debug Pizzamat deployment, repeat with **both** `cmd` and the topic's
command name changed to `puzza-list`. Keep the test read-only: there is no need
to add/reset orders to test routing.

If neither command spelling/topic produces a reply, check Pizzamat's service
logs while sending the list request: did it receive `pizza-list`/`puzza-list`,
and did it publish a reply for `subraumctl.send.bridgecheck`? Receipt without an
MQTT reply points toward reply bridging/permissions or a handler failure;
no receipt points toward command routing, permissions, or the service being
offline. A broker publish acknowledgement alone is not confirmation from EDI.

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
