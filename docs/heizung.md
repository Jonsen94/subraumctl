# Heating (Heizung)

```sh
subraumctl heizung rooms
subraumctl heizung status                  # all rooms
subraumctl heizung get hack make           # selected rooms
subraumctl heizung status hack --watch --interval 5
subraumctl heizung set hack make 21.5 -d 120
subraumctl heizung set --all 20             # default: 180 minutes
subraumctl --json heizung status
subraumctl --dry-run heizung set hack 21
```

Rooms: `make`, `hack`, `cook`, `elab`, `shit`, `storage`. Reads default to
all six; writes require room names or explicit `--all`. Duplicate rooms are
rejected. `get` is an alias for `status`.

The service reports **target temperature** and **time until standby**, not
measured room temperature or valve state. JSON reads contain a `rooms` array
with `room`, `target_celsius`, and `remaining_seconds`. Missing, malformed or
unexpected room replies fail rather than inventing values. Watch polls after
each completed request, with a default interval of five seconds.
Ctrl+C stops watching after any in-flight bounded request finishes cleanup.

Targets must be 15–25 °C in 0.1 °C steps, matching the firmware limits. Duration
is in whole minutes, defaults to the service's 180 minutes, and is limited to
0–35791 to fit the firmware's signed 32-bit millisecond timer. Zero expires
the timer; the firmware returns to its room-specific standby temperature on
its next update. A write acknowledgement means the service accepted the
setting, not that the room has reached that temperature. The service applies
settings to each room master and mirrors them to that room's other controllers.

## Connection and protocol

Like the Pizzamat integration, heating uses **direct EDI AMQP**, independently
of the MQTT `--broker`. No MQTT heating status topic is assumed.

| Option | Environment | Default |
|---|---|---|
| `--heizung-amqp-url` | `SUBRAUMCTL_HEIZUNG_AMQP_URL` | `amqp://amqp.subraum.c3pb.de:5672/%2f` |
| `--heizung-timeout` | `SUBRAUMCTL_HEIZUNG_TIMEOUT` | `15` seconds (1–120) |

Each operation publishes JSON to exchange `cmd`, routing key `heizung`:

```json
{"cmd":"heizung","args":"set hack make 21.5 -d 120","user":"subraumctl","src":"subraumctl.recv.<unique-queue>","dst":null}
```

Reads send `get <rooms...>`. A private exclusive queue is bound to exchange
`msg` with the source's `recv` replaced by `send`, before publishing. The
matching reply's `msg` is decoded into bounded structured fields; arbitrary
backend text and connection credentials are never returned. Publisher confirms
and mandatory routing detect rejected/unrouted commands. Connection and reply
phases each use the configured timeout, followed by at most two seconds for
cleanup. Requests are never retried automatically: a timeout after publishing
can mean a setting was applied. Read status before retrying.

`--dry-run` validates inputs and prints the command/arguments without connecting,
including for status/watch. `--json` emits one object per operation (one per
room for the static catalogue); `--quiet` suppresses human output.

## MCP

The `heizung` permission group defaults to `w`. Set `heizung = "r"` for reads
only or `heizung = "x"` to hide and disable every heating tool.

- `heizung_rooms`: static room catalogue, no arguments.
- `heizung_get`, `heizung_status`: `{"rooms":["hack","make"]}`; omitted/empty
  `rooms` reads all six. One-shot only.
- `heizung_set`: `{"rooms":["hack"],"temperature":21.5,"duration_minutes":120}`
  or `{"all":true,"temperature":20}`. Omitted duration defaults to 180.

Tools cannot change the endpoint, identity, routing, or timeout. Unknown fields
are rejected. The same validation and wire protocol are used by CLI and MCP.

## Verification

`cargo test` and `cargo test --features mcp` cover validation, reply decoding,
offline CLI behavior, MCP permissions, dry runs and credential redaction.
The ignored `heizung_amqp` integration test requires a **disposable** RabbitMQ:

```sh
docker run --rm -d --name heizung-test -p 127.0.0.1:25672:5672 rabbitmq:4-alpine
SUBRAUMCTL_TEST_AMQP_URL=amqp://127.0.0.1:25672/%2f \
  cargo test --test heizung_amqp -- --ignored
docker stop heizung-test
```
