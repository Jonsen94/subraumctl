# subraumctl — Rust CLI/TUI reimplementation of the Kevyn web UI

## 1. Goal

Reimplement the control functionality of the existing browser interface
(`index.html`, `kevyn.js`, `user.js`) as a Rust program with:

- a `clap` (derive) CLI that exposes every control the web UI has, plus
  descriptive `--help` output and generated man pages, and
- a `ratatui` TUI started with the `tui` subcommand. The TUI is **not** a
  pixel-for-pixel copy of the web UI: it is redesigned as a keyboard-first
  application with a sidebar, focused content area, a traffic pane, a
  toggleable application-log pane, safe mode, and a theme editor.

The existing web files stay untracked and are ignored by Git. They are only a
specification; no part of the web UI is copied into the repository.

The implementation is written but not compiled or executed in this
environment (no `cargo`, `rustc`, `nix`, or network access). Compilation,
tests, formatting and Nix builds are performed by the user afterwards.

### Hard constraints

- MQTT uses **normal TCP MQTT only**. WebSockets are explicitly out of scope;
  the `websockets` feature is not enabled and no `ws://`/`wss://` URI is used
  by default.
- MQTT disconnects must never kill the TUI. The TUI reconnects automatically
  with backoff, resubscribes, and keeps all UI state.
- The TUI has a **safe mode** (on by default) that asks for confirmation before
  every outgoing MQTT publish **and** every smoothiesaur HTTP command.
- The TUI always shows a traffic log of outgoing MQTT messages and incoming
  payloads relevant to the current section (5–10 lines, sized to the terminal).
- `Ctrl+L` toggles a top/bottom split whose bottom pane is the application log.
- Color pickers edit the TUI theme and are keyboard-driven; CLI color options
  accept CSS color names, `#hex`, `rgb(...)`, `hsl(...)`, and bare `r,g,b`.
- The existing web files are never added to Git.
- Commits use `Claude <claude@anthropic.com>` with **no commit trailers**.

---

## 2. Web UI feature inventory

### 2.1 Connection and shell controls

| Web control | Behaviour | Rust equivalent |
|---|---|---|
| `CON` | Connect MQTT client, status `connecting` → `connected`; retry after 5 s | Implicit for one-shot CLI commands; `c` in TUI; automatic reconnect |
| `DIS` | Disconnect, status `disconnected` | `d` in TUI (pauses automatic reconnect); CLI connects/disconnects per command |
| `NOR` / `DRG` | Interaction mode: normal vs. drag-and-drop layout editing | Not applicable. The TUI has a designed fixed layout; intentionally dropped |
| `REL` | Reload the browser page | `r` (reconnect) in the TUI |
| `MNU` | Main menu popup (fullscreen, zoom+, zoom−) | `?` help overlay / Overview section |
| `LOG` | Console popup (ESC, CLR, test) | `Ctrl+L` application-log pane (`tui-logger`); CLI logs to stderr |
| status indicator | Internal `int:status_changed` | `ConnectionState` in TUI header and Overview |
| alert LED | `heimdallr/alert/condition` used as a CSS class | Decoded alert state in TUI header/Overview and `alert led` CLI command |
| fullscreen | `document.body.requestFullscreen()` | Terminal emulator concern; not controlled by the app |
| zoom + / − | CSS zoom of the page | Terminal font size; not controlled by the app |
| URL tab parameter | Remembers the active tab (`level1=…`) | `--view` start option; current section kept in memory only |
| localStorage layouts | Draggable element layout persistence | Not reimplemented (drag mode is dropped) |

### 2.2 Control actions

#### Lighting / DMX (`colored_light`)

99 actions = 11 groups × 9 animations.

- Topic: `edi/cmd/dmx`
- Payload: `{"args":"<group> <animation>"}`
- Groups: `subraum.all`, `hack.all`, `hack.afterglow`, `hack.spots`,
  `hack.floor`, `hack.decke`, `hack.regal`, `make.all`, `make.decke`,
  `make.spots`, `flur`
- Animations: `alert`, `backgrounda`, `backgroundb`, `backgroundc`,
  `backgroundd`, `rainbow`, `white`, `black`, `fullhour`

#### White light

- Warm/cold sliders, range 0–255, integer values.
- Apply sends, for every selected cell:
  - Topic: `light/esp/request/<cell-id>`
  - Payload: `w <warm> c <cold> ` (trailing space preserved from the JS)
- Cell status:
  - Topic: `light/esp/status/<cell-id>`
  - Payload interpreted as boolean (on/off).
- Cell selection is local UI state, not MQTT state.
- Cell IDs (15 total):
  - `LM3_A020A63498E7`, `LM3_A020A6349898`, `LM3_A020A6349701`,
    `LM3_A020A634988B`
  - `LM3_A020A6167801`, `LM3_18FE34FE4C87`, `LM3_5CCF7F01C9DF`,
    `LM3_A020A6349804`, `LM3_A020A6349BE3`, `LM3_A020A61849EF`,
    `LM3_A020A6167E67`, `LM3_5CCF7F01CF21`, `LM3_18FE34FE48FC`,
    `LM3_A020A6349BF4`, `LM3_A020A6349910`

#### Outlets

- Topic: `edi/cmd/act`
- Payload: `{"args":"<outlet> on"}` or `{"args":"<outlet> off"}`
- Outlets:

| Topic name | Web label |
|---|---|
| `make.colors` | make.colors |
| `make.bright` | make.bright |
| `ert` | ert |
| `stationaere_kuehlungsloesung` | hackpad |
| `compressor` | compressor |
| `white-power` | white power |

No state feedback topic exists; the TUI shows the last commanded state only.

#### Alerts

Electromagnetic radiation emitters:

| Button | Topic | Payload |
|---|---|---|
| `.make` | `Rundumleuchte/Gelblicht` | `{"name":"Rundumleuchte","time":"2"}` |
| `.hack` | `Rundumleuchte/Blaulicht` | `{"name":"Rundumleuchte","time":"2"}` |

Gas pressure modulation / sob:

- Topic: `edi.cmd.sob`
- Payload: `{"cmd":"sob","args":"<sound>","user":"mcp","src":"na"}`
- Sounds: `hackbuzzer`, `dingdong`, `chewbacca`, `noot noot`

#### System

- Topic: `edi/cmd/telinit`
- On: `{"args":"4"}`
- Off: `{"args":"0"}` (marked destructive in the TUI)

#### Animation knobs

- Get topic: `knob/get/<name>`
- Set topic: `knob/set/<name>`
- Values are published as decimal strings.
- 29 knobs:

| Name | Min | Max | Open min | Open max |
|---|---:|---:|---|---|
| `cx` | -1000 | 1000 | yes | yes |
| `cy` | -1000 | 1000 | yes | yes |
| `cz` | -1000 | 1000 | yes | yes |
| `d` | 0 | 1 | yes | yes |
| `k` | 0 | 1000 | yes | yes |
| `phase` | 0 | 360 | yes | yes |
| `saturation` | 0 | 1 | no | no |
| `intensity` | 0 | 1 | no | no |
| `attack` | 0 | 1 | no | yes |
| `decay` | 0 | 1 | no | yes |
| `sustain` | 0 | 1 | no | yes |
| `release` | 0 | 1 | no | yes |
| `drop` | 0 | 1 | no | yes |
| `feed` | 0 | 1 | no | yes |
| `chaos` | 0 | 1 | no | yes |
| `order` | 0 | 1 | no | yes |
| `spatial` | 0 | 1 | no | yes |
| `logical` | 0 | 1 | no | yes |
| `harmony` | 0 | 1 | no | yes |
| `twitch` | 0 | 1 | no | yes |
| `exponent` | 0 | 4 | yes | yes |
| `lead` | 0 | 1 | no | yes |
| `pulseLead` | 0 | 1 | no | yes |
| `spread` | 0 | 1000 | no | yes |
| `glow` | 0 | 1000 | no | yes |
| `speedMin` | 0 | 1 | no | yes |
| `speedRange` | 0 | 1 | no | yes |
| `flowRate` | 0 | 2 | no | yes |
| `loss` | 0 | 1 | no | yes |

The web sliders are `immediate` and publish on every change. The TUI instead
edits a knob locally and publishes on `Enter`; this is deliberate: it is more
intentional and avoids a confirmation storm in safe mode. In normal (safe
mode off) use the same explicit `Enter` applies as well.

#### Smoothiesaur

Outgoing commands are **plain HTTP POSTs**, not MQTT:

- URL: `http://172.31.64.137/command`
- Body: `<command>\n`
- Relay commands (`M771I15P` + suffix):

| Relay | On command | Off command |
|---|---|---|
| `fanPort` | `M771I15P010100` | `M771I15P010000` |
| `fanStarboard` | `M771I15P010101` | `M771I15P010001` |
| `pumpPrimary` | `M771I15P010104` | `M771I15P010004` |
| `pumpSecondary` | `M771I15P010103` | `M771I15P010003` |
| `targetLaser` | `M771I15P01010f` | `M771I15P01000f` |
| `airValve` | `M771I15P010107` | `M771I15P010007` |
| `sterilizer` | `M771I15P010105` | `M771I15P010005` |

Laser power:

- Slider 0–255.
- Apply: `M771I19P<hex><hex>` where `<hex>` is the lowercase, zero-padded
  two-digit hex value (e.g. 10 → `0a`, 255 → `ff`).
- Zero: `M771I19P0000` (marked destructive).

Incoming status topics:

| Topic | Meaning / decode |
|---|---|
| `smoothie/relays/<relay>` | On/off (boolean parser) |
| `smoothie/mexicanTemp/relayControl` | Kelvin → °C |
| `smoothie/mexicanTemp/coolantSensors` | Kelvin → °C |
| `smoothie/coolant/reservoir` | Kelvin → °C |
| `smoothie/coolant/primary` | Kelvin → °C |
| `smoothie/coolant/secondary` | Kelvin → °C |
| `smoothie/coolant/flow` | Raw value + smoothed rate (EMA 0.9/0.1) |
| `smoothie/safetyLoop` | Hex bit mask, inverted: bit set → `0`/off, bit clear → `1`/on |
| `smoothie/laserpower` | Space-separated value; 8-bit control, 16-bit control, raw voltage |

Safety-loop bits:

| Mask | Label |
|---:|---|
| 8 | main - port |
| 16 | main - starboard |
| 1 | port hatch - medial |
| 2 | port hatch - lateral |
| 128 | starboard hatch - medial |
| 64 | starboard hatch - lateral |
| 4 | aft - port |
| 32 | aft - starboard |

Laser readouts:

- First value: `v * 100 / 255` percent.
- Second value: `v * 100 / 65535` percent.
- Third value: `volt = (raw + 200) * 6.430107526881721e-05`,
  percent of 2.7 V, volts shown with 4 decimals.

---

## 3. Approved crates

### 3.1 Existing approvals

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime (multi-thread, macros, sync, time, signal) |
| `thiserror` | Library error enums |
| `color-eyre` + `eyre` | Top-level error reporting |
| `clap` (derive + env) | CLI parsing |
| `clap_complete` | Shell completion generation |
| `ratatui` | TUI widgets and layout |
| `crossterm` | Raw mode, alternate screen, key events |
| `paho-mqtt` | MQTT client (`bundled`, `ssl`; **no websockets**) |
| `reqwest` | HTTP client for smoothiesaur (`rustls-tls`, no default features) |
| `serde` + `serde_json` | JSON payload construction and `--json` output |
| `tracing` | Structured logging facade |
| `tracing-subscriber` | Logger with `env-filter`, `fmt`, `tracing-log` bridge |

### 3.2 Newly approved crates

| Crate | Purpose |
|---|---|
| `tui-logger` | Ratatui log pane with tracing support; implements the `Ctrl+L` app-log pane |
| `clap_mangen` | Generate man pages from the clap command tree |
| `csscolorparser` | Parse named colors, `#hex`, `rgb()`, `hsl()`; RGB/HSV/HSL conversion |
| `tui-input` | Single-line input widget for search/filter, exact numbers and hex entry |
| `tui-popup` | Popup/dialog widget for safe-mode confirmation, help, quit, color picker |
| `tui-widget-list` | Scrollable/selectable list widget for sidebar and long lists |
| `tui-scrollview` | Scrollable viewport for help text and traffic details |
| `throbber-widgets-tui` | Animated connection/reconnect spinner in the header |

`tracing` is required by `tracing-subscriber`; `tracing-subscriber`'s
`tracing-log` feature bridges paho-mqtt's internal `log` records into tracing.
`tui-logger`'s tracing integration is enabled with its tracing support feature
(exact feature name to be confirmed during the first build; the intended
feature is `tracing-support`).

Implementation note: the final implementation uses ratatui's built-in
`Block`, `List` and `Paragraph` primitives for popups, lists and scrolling.
The approved optional crates `tui-popup`, `tui-widget-list` and
`tui-scrollview` are therefore not dependencies in the current `Cargo.toml`.
`tui-logger`, `tui-input`, `throbber-widgets-tui`, `csscolorparser` and
`clap_mangen` are used.

No other direct dependencies will be added without asking first.

### 3.3 Proposed `Cargo.toml`

```toml
[package]
name = "subraumctl"
version = "0.1.0"
edition = "2021"
rust-version = "1.82"
description = "MQTT CLI and TUI for the subraum"

[dependencies]
clap = { version = "4", features = ["derive", "env"] }
clap_complete = "4"
clap_mangen = "0.2"
color-eyre = "0.6"
crossterm = "0.28"
csscolorparser = "0.6"
eyre = "0.6"
paho-mqtt = { version = "0.12", features = ["bundled", "ssl"] }
ratatui = "0.29"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
throbber-widgets-tui = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time", "signal"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "tracing-log"] }
tui-input = "0.11"
tui-logger = { version = "0.14", features = ["tracing-support"] }
tui-popup = "0.6"
tui-scrollview = "0.4"
tui-widget-list = "0.11"
```

Version-alignment note: `crossterm` must match ratatui 0.29 (0.28). The
component crates (`tui-logger`, `tui-input`, `tui-popup`, `tui-widget-list`,
`tui-scrollview`, `throbber-widgets-tui`) must be pinned to the releases that
depend on ratatui 0.29; the versions above are the intended lines, but the
first `cargo generate-lockfile`/build may require a minor version adjustment
so that all widget types share the same ratatui version. `Cargo.lock` is
generated by the user and committed.

---

## 4. Project layout

```
.
├── Cargo.toml
├── Cargo.lock                 # generated by the user, committed
├── PLAN.md
├── README.md                  # short usage overview, key bindings, man pages
├── flake.nix
├── flake.lock                 # generated by the user, committed
├── .gitignore
└── src
    ├── main.rs                # clap entry point, dispatch, error setup
    ├── lib.rs                 # module re-exports
    ├── cli.rs                 # clap structs, value enums, long help, man generation
    ├── commands.rs            # CLI command implementations
    ├── action.rs              # Command/Action enums, expand(), wire payloads
    ├── knob.rs                # knob specs and lookup
    ├── white_light.rs         # cell IDs and groups
    ├── decode.rs              # payload decoders (bool, temp, laser, safety, flow)
    ├── mqtt.rs                # paho TCP session, callbacks, reconnect supervisor
    ├── smoothiesaur.rs        # HTTP commands and status decoding
    ├── color.rs               # CSS color parsing and theme roles/presets
    ├── logging.rs             # tracing setup for CLI and tui-logger
    ├── output.rs              # human/JSON/dry-run output helpers
    └── tui
        ├── mod.rs             # terminal guard and event loop
        ├── app.rs             # App state, key precedence, update logic
        ├── events.rs          # crossterm input thread, AppEvent
        ├── keymap.rs          # key binding descriptions for footer/help
        ├── theme.rs           # runtime Theme, color-picker state
        ├── traffic.rs         # wire-traffic ring buffer and per-view filters
        ├── safe_mode.rs       # confirmation state and command preview
        ├── ui.rs              # header/sidebar/content/traffic/log/footer layout
        └── views
            ├── overview.rs
            ├── lighting.rs
            ├── white_light.rs
            ├── outlets.rs
            ├── alerts.rs
            ├── system.rs
            ├── animation.rs
            ├── smoothie.rs
            └── theme.rs
```

---

## 5. Architecture

### 5.1 Actions and command expansion

All outgoing control operations are represented once, in `action.rs`, so the
CLI, TUI, safe-mode preview, dry-run mode and tests share exactly the same
topics and payloads.

```rust
pub enum Command {
    Dmx { group: DmxGroup, animation: DmxAnimation },
    WhiteLight { warm: u8, cold: u8, cells: Vec<CellId> },
    Outlet { outlet: Outlet, on: bool },
    Radiation { target: RadiationTarget },
    Sob { sound: SobSound },
    Telinit { on: bool },
    KnobSet { knob: Knob, value: f64 },
    SmoothieRelay { relay: SmoothieRelay, on: bool },
    SmoothieLaser { power: u8 },
    SmoothieLaserZero,
    RawPublish { topic: String, payload: String, retain: bool },
}

pub enum Action {
    Publish { topic: String, payload: String, retain: bool },
    Http { url: String, body: String },
}
```

`Command::expand(&self, opts) -> Result<Vec<Action>, ActionError>` turns one
high-level command into zero or more wire actions. A white-light command with
N selected cells expands into N publishes.

- DMX / outlet / telinit / sob / radiation → `serde_json::json!`.
- Knobs → `value.to_string()`.
- White light → `format!("w {warm} c {cold} ")`.
- Smoothiesaur → exact `M771…` command strings plus `\n` for HTTP.

Every `Command` also exposes `description()`, `destructive()` and the expanded
actions, so the safe-mode popup and the traffic pane can show a consistent
preview.

### 5.2 MQTT layer (TCP only)

`mqtt.rs` wraps `paho_mqtt::Client` using normal MQTT over TCP:

- `ConnectionOptions`: broker URI (`tcp://` or `ssl://`), client id, username,
  password, keep-alive, clean-session, connect timeout.
- Default broker: `tcp://webmqtt.subraum.c3pb.de:1883` (assumption to confirm
  with the user; overridable by `--broker` / `SUBRAUMCTL_BROKER`).
- Client id default: `subraumctl-<8 hex>` generated from system time and
  process id (no extra random crate).
- `CreateOptionsBuilder` sets `server_uri`, `client_id`,
  `persistence(None)`, and the message callback.
- `ConnectOptionsBuilder` sets keep-alive, clean session, connect timeout,
  optional credentials.
- Message callback pushes `MqttEvent::Message { topic, payload, retained,
  generation }` into a tokio unbounded channel.
- Connected/disconnected callbacks push `MqttEvent::Connected { generation }`
  and `MqttEvent::ConnectionLost { generation, reason }`.
- `start_consuming()` is called after a successful connect.
- QoS 0, retain false for all control publishes. `--retain` is available only
  on the raw `publish` command.
- Subscribe set after every connect:
  - `heimdallr/alert/condition`
  - `light/esp/status/#`
  - `knob/get/#`
  - `smoothie/#`
  - plus additional topics for `subscribe`/`monitor` CLI commands.
- Blocking paho calls (`connect`, `disconnect`, `stop_consuming`) run inside
  `tokio::task::spawn_blocking`; callbacks are non-blocking sends.

#### Automatic reconnect (TUI)

- The TUI always auto-reconnects; there is no `--no-auto-reconnect` option.
- On connection loss or failed connect, the TUI remains fully usable and
  schedules a retry with exponential backoff: 1 s, 2 s, 4 s, 8 s, 16 s,
  30 s cap; reset on success.
- The header shows `connecting`, `connected`, or
  `reconnecting in Ns (attempt k)` with a throbber.
- Each successful reconnect creates a new paho client, increments a generation
  counter, ignores stale events, and resubscribes to all status topics.
- Manual `d` pauses automatic reconnect until `c` (connect) or `r` (reconnect).
  Network loss does not pause it.
- Commands attempted while disconnected are rejected before the safe-mode
  prompt with a visible error: `Not connected — command not sent.` Nothing is
  silently queued or dropped.
- `--no-connect` starts offline; no automatic attempts occur until `c`/`r`.
- The TUI never exits because of MQTT errors.

### 5.3 Smoothiesaur HTTP layer

`reqwest::Client` with a 5 s timeout. `Action::Http` sends the command string
plus `\n` to the configured URL. Any non-2xx status is an error. Requests run
in a spawned task and report `AppEvent::HttpResult`; the TUI never blocks.
Errors are shown in the status line, the traffic pane (`HTTP !`) and the app
log. Safe mode covers HTTP commands as well.

### 5.4 Logging

- `tracing` is the single logging facade.
- CLI mode: `tracing_subscriber::fmt` to stderr with an `EnvFilter` from
  `RUST_LOG` or `-v`/`-vv`/`-vvv`; data goes to stdout.
- TUI mode: `tui-logger`'s tracing subscriber layer feeds the in-app log pane.
  Nothing is written to stderr while the alternate screen is active.
- `-v` mapping: 0 = warn, 1 = info, 2 = debug, 3 = trace.
- paho-mqtt's internal `log` records are bridged through tracing-subscriber's
  `tracing-log` feature; app-level tracing events always appear even if the
  bridge is unavailable.
- The application log ring buffer is owned by `tui-logger`; the traffic pane
  is a separate wire-level ring buffer (see 7.5).

### 5.5 Error handling

- `thiserror` enums: `MqttError`, `HttpError`, `ActionError`, `ColorError`,
  `TuiError`.
- `main` returns `color_eyre::Result<()>`; `color_eyre::install()` runs first.
- CLI errors get `eyre::WrapErr` context and exit with code 1.
- TUI errors are non-fatal: logged, shown in the status line, and followed by
  automatic reconnect where applicable.

---

## 6. CLI design

```
subraumctl [GLOBAL OPTIONS] <COMMAND>
```

### 6.1 Global options

| Option | Default / env | Meaning |
|---|---|---|
| `-b, --broker <URI>` | `SUBRAUMCTL_BROKER`, `tcp://webmqtt.subraum.c3pb.de:1883` | Normal MQTT URI (`tcp://` or `ssl://`); no WebSockets |
| `--client-id <ID>` | generated `subraumctl-<hex>` | MQTT client id |
| `--username <USER>` | `SUBRAUMCTL_USERNAME` | MQTT username |
| `--password <PASS>` | `SUBRAUMCTL_PASSWORD` | MQTT password (prefer env) |
| `--keep-alive <SECS>` | 60 | MQTT keep-alive |
| `--connect-timeout <SECS>` | 5 | Connect timeout |
| `--clean-session` / `--no-clean-session` | clean | MQTT clean session |
| `--wait <SECS>` | 3 | Timeout for get/subscribe/monitor/status |
| `--smoothiesaur-url <URL>` | `SUBRAUMCTL_SMOOTHIESAUR_URL`, `http://172.31.64.137/command` | Smoothiesaur HTTP endpoint |
| `--dry-run` | off | Print wire actions without connecting/sending |
| `--json` | off | Machine-readable output |
| `-q, --quiet` | off | Suppress non-error human output |
| `-v, --verbose` | warn | Repeatable; info/debug/trace |

### 6.2 Subcommands

```
dmx <GROUP> <ANIMATION>
white-light apply --warm <0-255> --cold <0-255> (--cell <ID>... | --all)
white-light cells
white-light status [--watch]
outlet <OUTLET> <on|off>
alert radiation <make|hack>
alert sob <hackbuzzer|dingdong|chewbacca|noot-noot>
alert led [--watch]
system telinit <on|off>
knob list
knob get <NAME>
knob set <NAME> <VALUE>
knob watch [<NAME>...]
smoothiesaur relay <RELAY> <on|off>
smoothiesaur laser <0-255>
smoothiesaur zero
smoothiesaur status [--watch]
status
publish <TOPIC> <PAYLOAD> [--retain] [--payload-file <PATH>]
subscribe <TOPIC> [--count <N>] [--wait <SECS>]
monitor [--kind alert|white-light|knobs|smoothiesaur|all] [--wait <SECS>]
tui [OPTIONS]
man [--output-dir <DIR>] [<NAME>]
completions <bash|zsh|fish|powershell|elvish>
```

Value-enum details:

- DMX groups and animations use the exact strings from the web UI (including
  dots in group names).
- Outlet names use the exact topic strings; kebab-case aliases are added
  (`make-colors`, `white-power`).
- Knob names use the exact camelCase topic names; kebab-case aliases are added
  (`pulse-lead`, `speed-min`, `speed-range`, `flow-rate`).
- Smoothiesaur relays use kebab-case (`fan-port`, `pump-primary`, …) and map
  to the camelCase status topics.
- `sob noot-noot` maps to the payload argument `noot noot`.

### 6.3 `tui` options

```
tui [--view <SECTION>]
    [--no-connect] [--no-safe-mode]
    [--theme <dark|light|high-contrast>]
    [--background-color <COLOR>]
    [--foreground-color <COLOR>]
    [--accent-color <COLOR>]
    [--selection-color <COLOR>]
    [--success-color <COLOR>]
    [--warning-color <COLOR>]
    [--error-color <COLOR>]
    [--muted-color <COLOR>]
    [--border-color <COLOR>]
    [--alert-color <COLOR>]
```

- `--view` accepts `overview`, `lighting`, `white-light`, `outlets`, `alerts`,
  `system`, `animation`, `smoothie`, `theme`.
- Safe mode is **on by default**; `--no-safe-mode` starts with it off. Safe
  mode is a TUI feature; non-interactive CLI commands are explicit by nature.
- Theme options apply to the TUI only. `--theme` selects a preset; individual
  `--*-color` flags override it. Color arguments accept:
  - CSS names: `teal`, `rebeccapurple`, `orange`, `warm`-like names are not
    invented; only real CSS color names plus the aliases below.
  - `#rgb`, `#rrggbb`, `#rrggbbaa` (alpha is ignored with a warning because
    terminals have no alpha).
  - `rgb(0,128,128)`, `rgba(...)`, `hsl(...)`, `hsla(...)`.
  - Bare `0,128,128` (treated as RGB).
  - Bare `0af` / `00aaff` (treated as hex).
- Invalid colors produce a clear `ColorError` and exit code 2/1 as appropriate.

### 6.4 Help and man pages

The `--help` output is part of the product, not an afterthought:

- The root command has a long overview: what the program controls, the MQTT
  broker default, safe-mode note, and examples.
- Every subcommand has `long_about` / `long_help` describing what it does, the
  exact topic and payload it sends, valid values/ranges, and 1–3 examples.
- `after_help` sections show topics, examples, and safety notes.
- `help_template` groups usage, commands, arguments and options consistently.
- `man` subcommand uses `clap_mangen`:
  - `subraumctl man` prints the root man page to stdout.
  - `subraumctl man dmx` prints the DMX man page.
  - `subraumctl man --output-dir DIR` writes `subraumctl.1` and
    `subraumctl-<subcommand>.1` files.
- The Nix package installs the generated man pages under
  `$out/share/man/man1` and shell completions under the usual share paths.

Examples:

```sh
subraumctl dmx subraum.all rainbow
subraumctl white-light apply --warm 128 --cold 200 --all
subraumctl white-light apply --warm 128 --cold 200 \
  --cell LM3_A020A63498E7 --cell LM3_18FE34FE4C87
subraumctl outlet make.colors on
subraumctl alert radiation make
subraumctl alert sob noot-noot
subraumctl alert led --watch
subraumctl system telinit on
subraumctl knob set cx 42
subraumctl knob get phase
subraumctl knob watch cx cy cz
subraumctl smoothiesaur relay pump-primary on
subraumctl smoothiesaur laser 128
subraumctl smoothiesaur status --watch
subraumctl publish edi/cmd/dmx '{"args":"subraum.all rainbow"}'
subraumctl subscribe 'knob/get/#' --count 10
subraumctl monitor --kind all
subraumctl tui --view animation --accent-color teal
subraumctl tui --theme high-contrast --error-color 'rgb(255,80,80)'
subraumctl man --output-dir ./man
subraumctl completions zsh
```

### 6.5 Output and dry-run

- Default action output (one line per wire action):
  - `published edi/cmd/dmx: {"args":"subraum.all rainbow"}`
  - `sent M771I19P0101 to http://172.31.64.137/command`
- `--json`: one object per line, e.g.
  `{"transport":"mqtt","topic":"…","payload":"…","retain":false}` or
  `{"transport":"http","url":"…","body":"…"}`.
- `--dry-run`: same lines/objects, no connection and no HTTP request. This is
  the primary offline test path.
- `subscribe`: `topic payload` per message; JSON lines with `--json`.
- `monitor`: human-readable decoded lines, e.g. `alert: Code: Green`,
  `knob cx = 42`, `white-light LM3_… = on`,
  `smoothie reservoir = 23.45 °C`.
- `status`: broker, client id, connection result, and the current
  `heimdallr/alert/condition` if one arrives within `--wait`.
- Logs go to stderr; machine-readable data goes to stdout.

---

## 7. TUI design

The TUI is deliberately **not** a copy of the web UI. It is a keyboard-first
dashboard designed around clear navigation, explicit actions, and visible
feedback.

### 7.1 Design principles

1. **One obvious place for each job.** A persistent sidebar separates
   sections; the content area shows only the current section.
2. **No hidden state.** Every pending action, connection state, safe-mode
   state and error is visible in the header, status line or traffic pane.
3. **Explicit sends.** Buttons and knobs are edited locally and sent with
   `Enter`; safe mode can therefore confirm one meaningful command instead of
   every keystroke.
4. **Preview before send.** Every action shows the exact topic/payload and,
   in safe mode, a confirmation dialog.
5. **Never lose the user.** MQTT disconnects do not close the app or reset the
   section; the TUI reconnects automatically and keeps state.
6. **Keyboard-first.** Every interaction has a key; the footer always shows
   the currently relevant bindings; `?` opens the full cheat sheet.
7. **Readable in a terminal.** 16-color-safe defaults, truecolor when the
   terminal supports it, no reliance on mouse input.

### 7.2 Screen layout

Normal (application log hidden):

```
┌ subraumctl  ● Connected  tcp://…  Alert: ● Code: Green  SAFE ┐
├────────────────┬───────────────────────────────────────────────┤
│  Overview      │  Lighting                      / filter       │
│  Lighting    > │  ┌───────────────────────────────────────────┐ │
│  White Light   │  │ Groups              Animations            │ │
│  Outlets       │  │ > subraum.all       alert                 │ │
│  Alerts        │  │   hack.all          background a          │ │
│  System        │  │   hack.afterglow    background b          │ │
│  Animation     │  │   ...               ...                   │ │
│  Smoothiesaur  │  └───────────────────────────────────────────┘ │
│  Theme         │  Send: edi/cmd/dmx ← {"args":"…"}             │
│                ├───────────────────────────────────────────────┤
│                │ Traffic — Lighting                            │
│                │ 12:01:02 ↑ edi/cmd/dmx {"args":"subraum…"}    │
│                │ 12:01:05 ↓ light/esp/status/… on              │
│                │ … 5–10 lines, auto-sized                      │
├────────────────┴───────────────────────────────────────────────┤
│ Last action: sent DMX subraum.all rainbow                       │
│ Tab focus  [ ] section  ↑↓ move  ←→ value  Enter send  ? help  │
│ Ctrl+L log  Ctrl+T traffic  S safe  q back  Q quit              │
└────────────────────────────────────────────────────────────────┘
```

- Header (1 line): app name, connection status/spinner, broker, alert LED and
  label, safe-mode indicator.
- Sidebar (fixed ~22–26 columns): the sections listed below. The active
  section is highlighted; focus is indicated separately.
- Content: section-specific controls, with a one-line subtitle and a send
  preview.
- Traffic pane: always visible when the terminal is at least 80×24; height
  `clamp(terminal_height / 6, 5, 10)`.
- Status line (1 line): last action, error, or safe-mode prompt hint.
- Footer (2 lines): context-sensitive key bindings. The first line is global,
  the second is specific to the focused zone/section.
- Minimum supported size: 80×24. Below that, a centered "terminal too small"
  message is shown instead of a broken layout.

Application log open (`Ctrl+L`):

```
┌ header ────────────────────────────────────────────────────────┐
│ sidebar │ content                                             │
│         ├─────────────────────────────────────────────────────┤
│         │ Traffic — current section (5–10 lines)              │
├─────────┴─────────────────────────────────────────────────────┤
│ Application log (tui-logger)                        35% height │
│ 12:01:02 INFO  mqtt: connected                                │
│ 12:01:03 DEBUG mqtt: subscribed light/esp/status/#            │
│ 12:01:09 WARN  http: smoothiesaur request failed              │
├───────────────────────────────────────────────────────────────┤
│ status line                                                   │
│ footer key bindings                                           │
└───────────────────────────────────────────────────────────────┘
```

The log pane splits the body top/bottom, with the app log at the bottom. The
traffic pane stays visible above it.

### 7.3 Navigation and focus

There are four focus zones: **Sidebar**, **Content**, **Traffic**, and
**Application log** (when open).

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle focus zones |
| `Ctrl+T` | Jump to the traffic pane (and back) |
| `Ctrl+G` | Jump to the application log (and back; opens it if hidden) |
| `Ctrl+L` | Toggle the application-log pane |
| `[` / `]` | Previous/next section from anywhere |
| `↑` / `↓` or `k` / `j` | Move within the focused zone |
| `←` / `→` or `h` / `l` | Change the focused value/selection |
| `Enter` | Activate the focused item / send / apply |
| `Space` | Toggle the focused checkbox/cell/selection |
| `q` | Close popup/input/log pane; else return to Overview; in Overview open the quit popup |
| `Esc` | Close popup/input/log pane; else return to Overview (never opens the quit popup) |
| `Q` | **Always** quit immediately, from any view, popup, input or pane |
| `Ctrl+C` | Same as `Q` |
| `?` | Open/close the help overlay |
| `/` | Search/filter in the current list (uses `tui-input`) |
| `S` | Toggle safe mode (turning it off requires confirmation) |
| `c` / `d` / `r` | Connect / disconnect (pauses auto-reconnect) / reconnect |

Sidebar behaviour: when focused, `↑/↓` changes the section immediately and the
content updates; `Enter` moves focus to the content. The sidebar highlights the
active section and uses a separate marker for keyboard focus.

Content behaviour: `↑/↓` moves between controls; `←/→` changes values;
`Enter` activates. Sections with long lists support `/` filtering; `Esc`
clears the filter before doing anything else.

Traffic behaviour: `Ctrl+T` focuses; `↑/↓`, `PgUp/PgDn` scroll; `Enter` opens a
detail popup with the full topic/payload; `Esc`/`q` returns focus to content.

Application log behaviour: `tui-logger`'s smart widget handles its own level
filter and scrolling. When focused, arrow keys/`PgUp`/`PgDn` scroll and its
documented filter keys work; `Esc`/`q` returns focus to content (it does not
close the pane; use `Ctrl+L` or `q` while focused to close it, matching the
"q closes the current view" rule).

### 7.4 Sections

#### Overview

- Connection card: status, broker, client id, reconnect attempt/countdown,
  uptime.
- Alert card: colored LED and label from `heimdallr/alert/condition`.
- Safe-mode card: on/off and how to toggle.
- Quick stats: last action, last error, traffic counts.
- Section shortcuts: `Enter` opens the highlighted section.
- This is the "main view": `q` here opens the quit confirmation popup.

#### Lighting

- Two coordinated selectors: a group list (11) and an animation list (9).
- `↑/↓` selects a group; `←/→` selects an animation; the preview line shows
  the exact `edi/cmd/dmx` payload.
- `/` filters groups/animations by name.
- `Enter` sends; safe mode confirms.
- Destructive? No; listed as a normal action.

#### White Light

- Warm/cold gauges (0–255) with exact numeric entry.
- Cell list (15) showing group, cell id, live on/off status and selection.
- `Space` toggles a cell; `a` selects all; `n` clears; `/` filters cells.
- `w`/`c` focuses the warm/cold gauge; `←/→` adjusts by 1,
  `Shift+←/→` by 32, `Home/End` min/max; digits start exact entry.
- `Enter` on Apply sends one white-light command per selected cell; the
  safe-mode confirmation lists the count and the exact payload template.

#### Outlets

- Six rows, each with `on`/`off` actions and the last commanded state.
- `←/→` chooses on/off; `Space` toggles and sends; `Enter` sends the focused
  action.
- Safe mode confirms each send.

#### Alerts

- Radiation emitters: `.make`, `.hack`.
- Sob sounds: `hackbuzzer`, `dingdong`, `chewbacca`, `noot noot`.
- Alert LED state from `heimdallr/alert/condition`.
- `Enter` sends; safe mode confirms.

#### System

- Telinit on/off.
- `on` sends `{"args":"4"}`; `off` sends `{"args":"0"}` and is marked
  **destructive** with a warning color.
- Safe mode confirms; the confirmation is extra important here.

#### Animation

- Searchable/filterable list of all 29 knobs.
- Each row: name, range, current value, a `LineGauge`, and a dirty marker.
- `↑/↓`/`PgUp`/`PgDn` moves; `←/→` adjusts by a baby step (1% of range, at
  least 1); `Shift+←/→`/`H`/`L` big step (25%); `Home`/`End` min/max.
- Digits/`-`/`.` start exact text entry using `tui-input`; `Enter` commits.
- The value is edited locally and marked dirty; `Enter` publishes
  `knob/set/<name>`; `Esc` reverts to the last received/sent value.
- Incoming `knob/get/<name>` values update non-dirty knobs and are visible in
  the traffic pane.
- Safe mode confirms the single publish with the exact value.

#### Smoothiesaur

- Relay table (7 rows): live status, on/off selection, last commanded state.
- Laser gauge (0–255) with Apply and Zero actions, plus the three decoded
  readouts.
- Status panels: 5 temperatures, coolant flow raw/rate, 8 safety-loop bits.
- HTTP pending/error state shown in the status line and traffic pane.
- `Tab`/arrows navigate; `Enter` sends; `z` sends `M771I19P0000` (marked
  destructive).
- Safe mode confirms every HTTP command with the exact URL and body.

#### Theme

- List of theme roles: background, foreground, accent, selection, success,
  warning, error, muted, border, alert.
- Each row shows a color swatch, the role name, and its current hex/name.
- `Enter` opens the keyboard color picker for that role.
- `r` resets the focused role to the preset default; `R` resets the whole
  theme after confirmation.
- No MQTT traffic; the traffic pane shows a friendly empty state.

### 7.5 Traffic pane (wire log)

Always visible in supported terminal sizes, at the bottom of the content area
(above the app log pane and footer). It is distinct from the application log.

- Global ring buffer (e.g. 1000 entries) of wire events. Each entry:
  timestamp, direction, transport, topic/URL, payload/body, status, and the
  set of sections it belongs to.
- Directions:
  - `↑` MQTT publish sent.
  - `↓` MQTT message received.
  - `→` HTTP command sent (smoothiesaur).
  - `!` failed/blocked command (e.g. offline rejection, HTTP error).
  - `◇` dry-run preview.
- The pane filters entries relevant to the current section:
  | Section | Filter |
  |---|---|
  | Overview | all entries |
  | Lighting | `edi/cmd/dmx` |
  | White Light | `light/esp/request/#` (TX), `light/esp/status/#` (RX) |
  | Outlets | `edi/cmd/act` |
  | Alerts | `Rundumleuchte/#`, `edi.cmd.sob`, `heimdallr/alert/condition` |
  | System | `edi/cmd/telinit` |
  | Animation | `knob/set/#` (TX), `knob/get/#` (RX) |
  | Smoothiesaur | `smoothie/#` (RX), HTTP commands (TX) |
  | Theme | none (empty state) |
- Height: `clamp(terminal_height / 6, 5, 10)` lines.
- Latest entries at the bottom; auto-scroll unless the user has scrolled up.
- Payloads are truncated to the pane width with a trailing `…`; `Enter` on an
  entry (when focused) opens a detail popup with the full topic/payload,
  retain flag, timestamp and transport.
- Safe-mode confirmations happen before the entry is added; after sending, the
  entry appears as `↑`/`→`. Offline rejections appear as `!` with the reason.
- The app log pane does not replace the traffic pane; both can be visible.

### 7.6 Application log pane (`Ctrl+L`, `tui-logger`)

- `Ctrl+L` toggles a top/bottom split. The bottom pane is the application log
  rendered by `tui-logger`'s smart widget.
- Height: 35% of the body, minimum 6 lines, maximum 50%; `Ctrl+↑`/`Ctrl+↓`
  resizes when the pane is focused.
- It shows tracing events from the app, MQTT connection lifecycle, publish
  errors, HTTP errors, safe-mode decisions and paho-mqtt logs (bridged via
  tracing-log).
- The smart widget provides level filtering, expansion of multi-line events,
  and scrolling. `Ctrl+G` focuses it; `Esc`/`q` returns to content; `Ctrl+L`
  closes it.
- Initial level from `-v`; the widget can change it live.
- Nothing is written to stderr while the TUI is active.

### 7.7 Safe mode

- **Default: on.** The header shows `SAFE` in the warning color.
- Every outgoing command is confirmed:
  - MQTT publishes, including DMX, white light, outlets, radiation, sob,
    telinit, knobs and raw/CLI-equivalent actions.
  - Smoothiesaur HTTP POSTs.
- Confirmation popup:
  ```
  ┌ Confirm MQTT publish ─────────────────────────────┐
  │ Action:  DMX animation                            │
  │ Topic:   edi/cmd/dmx                              │
  │ Payload: {"args":"subraum.all rainbow"}           │
  │ Retain:  false                                    │
  │                                                    │
  │               [ Cancel ]  [ Send ]                 │
  │ y/Enter send   n/Esc/q cancel   Q quit            │
  └────────────────────────────────────────────────────┘
  ```
  - Default focus is **Cancel**; `Tab`/`←`/`→` switches; `y` sends; `n`,
    `Esc` or `q` cancels; `Q` always quits.
  - Destructive actions (`telinit off`, `smoothiesaur zero`) show a red
    `WARNING: destructive` line.
  - White-light sends show the number of cells and the exact payload.
  - HTTP sends show the URL and the command body.
- `S` toggles safe mode. Turning it **off** requires a separate confirmation
  popup ("Disable safe mode? Commands will be sent without confirmation.").
- If disconnected, the command is rejected before the popup with a visible
  status error. It is not queued and not silently dropped.
- `--no-safe-mode` disables it at startup (for demos/automation).
- The non-interactive CLI has no safe mode; its commands are already explicit,
  scriptable actions.

### 7.8 Auto-reconnect and survivability

- The TUI auto-connects on startup (unless `--no-connect`) and never exits on
  connection loss.
- Reconnect backoff: 1 s, 2 s, 4 s, 8 s, 16 s, 30 s cap; reset after success.
- The header shows a throbber (`throbber-widgets-tui`) and a countdown.
- On success: resubscribe to all status topics, reset backoff, log an info
  event, keep the current section and all local state.
- A generation counter prevents stale callbacks from old clients updating the
  UI.
- Manual `d` pauses automatic reconnect; `c` connects immediately; `r`
  recreates the client. Network failures do not pause it.
- Commands while offline are rejected with `Not connected — command not sent.`
- The status line and app log record every reconnect attempt/failure.

### 7.9 Theme and keyboard color picker

The TUI theme is editable at runtime and configurable from the CLI. Colors are
parsed with `csscolorparser`; alpha is ignored with a warning.

Theme roles:

| Role | Default (dark preset) | CLI flag |
|---|---|---|
| background | terminal default | `--background-color` |
| foreground | terminal default | `--foreground-color` |
| accent | cyan | `--accent-color` |
| selection | blue/dark | `--selection-color` |
| success | green | `--success-color` |
| warning | yellow | `--warning-color` |
| error | red | `--error-color` |
| muted | dark gray | `--muted-color` |
| border | dark gray | `--border-color` |
| alert | magenta/red | `--alert-color` |

Presets: `dark` (default), `light`, `high-contrast`. Individual color flags
override the preset.

Color picker popup (opened from the Theme section with `Enter`):

- Shows a live swatch, the hex value, and component bars.
- Modes: RGB, HSV, HSL; `m` cycles modes.
- `Tab`/`Shift+Tab` selects the component.
- `←`/`→` or `h`/`l` adjusts by 1 (or 1° hue / 1% S/V/L).
- `Shift+←`/`Shift+→` or `H`/`L` adjusts by a big step (16 / 10°).
- `Home`/`End` sets the component to its minimum/maximum.
- Digits start exact numeric input for the focused component (`tui-input`).
- `p` opens a searchable named-color palette (CSS names, `/` filter,
  arrows, `Enter` select).
- `Space` toggles preview between the original and edited color.
- `Enter` applies the color live and closes the picker.
- `Esc` cancels and restores the original color.
- `r` resets the role to the preset default.
- `Q` still quits the whole app.

The theme applies live to the header, sidebar, selection, status, traffic
directions, alert LED and gauges. The Theme section also displays the
equivalent CLI flags/hex values so a theme can be reproduced.

### 7.10 Quit semantics (exact)

- `Q` (uppercase) **always** quits immediately from any view, popup, text
  input, color picker, traffic detail or log pane. No confirmation.
- `q`:
  1. If a popup is open: close it (the quit popup closes/cancels).
  2. Else if text input/search/color editing is active: cancel it.
  3. Else if the application log pane is focused: close the log pane.
  4. Else if the traffic pane is focused: return focus to content.
  5. Else if the active section is not Overview: switch to Overview.
  6. Else (Overview): open the quit-confirmation popup.
- Quit popup: default **No**; `y`/`Enter` on Yes quits; `n`/`Esc`/`q` cancels;
  `Q` quits regardless.
- `Ctrl+C` behaves like `Q` (force quit).
- The terminal is always restored on quit, error or panic.

### 7.11 Event loop and TUI state

- `TerminalGuard` enters raw mode + alternate screen and restores the terminal
  on `Drop` and on panic. The panic hook is chained after `color_eyre`'s hook
  so the terminal is restored before the report is printed.
- Input: a dedicated `std::thread` polls crossterm events (`event::poll` 50 ms)
  and sends `AppEvent::Key`/`AppEvent::Resize` into a tokio unbounded channel.
- Ticks: `tokio::time::interval(100 ms)` drives redraws, reconnect countdowns,
  the throbber and status-line expiry.
- MQTT callbacks, HTTP task results and log events arrive as `AppEvent`s.
- `App::handle_key` applies the precedence in 7.10 before section handling.
- `App::update` handles connection events, message decoding, safe-mode state,
  traffic entries, HTTP results and theme changes.
- Rendering is immediate-mode after each event/tick.
- Quitting sets `App::should_quit`; the loop restores the terminal and returns
  exit code 0.

---

## 8. Message decoders

Shared in `decode.rs`:

- `parse_bool`: accepts `true/on/yes/y` and `false/off/no/n`, else numeric
  non-zero; empty is false; unparsable non-empty text is true (matching the
  JS coercion semantics closely). Used for white-light status and relays.
- `temperature_celsius`: `value - 273.15`, two decimals.
- `laser_control_8`: `v * 100 / 255`, two decimals.
- `laser_control_16`: `v * 100 / 65535`, two decimals.
- `laser_voltage`: `(raw + 200) * 6.430107526881721e-05`, percent of 2.7 V,
  four decimals for volts.
- `safety_bit`: parse leading hex digits, `(mask & bit) != 0` → off (`0`),
  otherwise on (`1`).
- `FlowEstimator`: last value/time, `rate = (new - old) / elapsed_secs`, then
  `estimate = estimate * 0.9 + rate * 0.1`; first sample only primes it.
  Accepts an explicit `Instant` in tests.
- `AlertCondition`: maps payload to a label/colour:
  | Payload | Label | Colour |
  |---|---|---|
  | `green` | Code: Green | success |
  | `blue` | Code: Blue | accent/blue |
  | `contact` | Code: Yellow | warning |
  | `offworld` | Code: Purple | magenta |
  | `intruder` | Code: Red | error |
  | `red` | Code: Red | error |
  | `selfdestruct` | Code: Red | error |
  | `biohazard` | Code: Black | alert/magenta |
  | anything else / empty | unknown/off | muted/dim |
  Optional `dim` token is preserved as a dim style.

Malformed messages are logged at debug/warn level and ignored; they never
crash the TUI or abort a monitor.

---

## 9. Nix flake

Use `flake-parts`, `crane`, `treefmt-nix`, and stable nixpkgs.

`flake.nix` outline:

```nix
{
  description = "subraumctl - MQTT CLI/TUI for the subraum";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      imports = [ inputs.treefmt-nix.flakeModule ];

      perSystem = { config, pkgs, lib, ... }:
        let
          craneLib = inputs.crane.mkLib pkgs;
          src = craneLib.cleanCargoSource ./.;

          commonArgs = {
            inherit src;
            strictDeps = true;
            nativeBuildInputs = with pkgs; [ cmake pkg-config ];
            buildInputs = with pkgs; [ openssl ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          subraumctl = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            postInstall = ''
              mkdir -p $out/share/man/man1
              $out/bin/subraumctl man --output-dir $out/share/man/man1

              mkdir -p $out/share/bash-completion/completions
              $out/bin/subraumctl completions bash \
                > $out/share/bash-completion/completions/subraumctl

              mkdir -p $out/share/zsh/site-functions
              $out/bin/subraumctl completions zsh \
                > $out/share/zsh/site-functions/_subraumctl

              mkdir -p $out/share/fish/vendor_completions.d
              $out/bin/subraumctl completions fish \
                > $out/share/fish/vendor_completions.d/subraumctl.fish
            '';
            meta = {
              description = "MQTT CLI/TUI for the subraum";
              mainProgram = "subraumctl";
            };
          });
        in
        {
          packages.default = subraumctl;
          packages.subraumctl = subraumctl;

          apps.default = {
            type = "app";
            program = "${subraumctl}/bin/subraumctl";
          };

          checks = {
            inherit subraumctl;
            tests = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });
            clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- -D warnings";
            });
          };

          devShells.default = pkgs.mkShell {
            inputsFrom = [ subraumctl ];
            packages = with pkgs; [
              cargo
              clippy
              rust-analyzer
              rustfmt
              cmake
              pkg-config
              openssl
            ];
          };

          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              rustfmt.enable = true;
              nixpkgs-fmt.enable = true;
            };
          };
        };
    };
}
```

Notes:

- No WebSocket support is compiled in; paho-mqtt uses the normal TCP transport.
- The `bundled` feature builds the vendored Paho C library with crane. The
  Nix-provided `paho-mqtt-c` is only needed if the bundled build is disabled.
- `cmake`, `pkg-config` and `openssl` cover Paho + optional TLS (`ssl://`).
- `Cargo.lock` and `flake.lock` cannot be generated here. Generate them once
  with `cargo generate-lockfile` / `nix flake lock` and commit them.
- `nixpkgs` uses a stable branch (`nixos-26.05` at the time of writing); bump
  the branch if the user's stable is newer.
- The man pages and completions are generated by running the built binary in
  `postInstall`; for cross-compilation, pre-generate them instead.
- `treefmt` provides `nix fmt` / `formatter.<system>` and a formatting check.
- Systems are Linux-only for now; adding Darwin would require the appropriate
  Security/CoreFoundation frameworks.

---

## 10. Testing and verification

Written now, executed later by the user.

### Unit tests (no network)

- `action.rs`: topic/payload for every command; white-light expansion over
  multiple cells; exact relay/laser command strings; radiation topic mapping;
  sob `noot noot` mapping; telinit values; knob numeric formatting; destructive
  flags.
- `decode.rs`: boolean parser variants, Kelvin conversion, laser 8/16/voltage,
  safety bit inversion, alert mapping, flow estimator with injected instants.
- `knob.rs`: 29 specs exist, ranges and open flags.
- `white_light.rs`: 15 unique IDs and group assignments.
- `color.rs`: parse CSS names, `#rgb`, `#rrggbb`, `rgb(...)`, `hsl(...)`, bare
  `r,g,b`, bare hex; reject invalid; alpha warning; preset/override merging.
- `traffic.rs`: per-section filters, ring-buffer eviction, TX/RX/HTTP/error
  directions, detail formatting.
- `safe_mode.rs`: every command produces a confirmation preview; offline
  rejection path; destructive markings; safe-mode-off confirmation.
- `tui::app`: pure key-handling tests:
  - `Q` quits from every section, popup, text entry, color picker and pane.
  - `q` precedence (popup → input → log pane → traffic → non-Overview →
    Overview quit popup).
  - Focus-zone cycling and sidebar/content navigation.
  - Slider local-edit/dirty/revert behaviour.
  - Safe-mode confirmation Yes/No and default focus.
  - Color picker modes, component wrapping and cancel/restore.
- `cli.rs`: `Cli::try_parse_from` for each subcommand; `CommandFactory`
  debug assertion; invalid values rejected; `--help`/long help contains the
  expected descriptions and examples.

### Offline command checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- --help
cargo run -- dmx --help
cargo run -- tui --help
cargo run -- --dry-run dmx subraum.all rainbow
cargo run -- --dry-run white-light apply --warm 1 --cold 2 --all
cargo run -- --dry-run smoothiesaur laser 16
cargo run -- man --output-dir /tmp/subraum-man
ls /tmp/subraum-man
```

### Manual integration

- Connect to the real TCP broker (confirm the default URI or pass
  `--broker tcp://host:1883`).
- Verify DMX, outlet, radiation, sob, telinit and knob messages with a second
  MQTT subscriber.
- Verify white-light topics/payloads against one cell before selecting all.
- Verify smoothiesaur HTTP commands against the device if reachable; otherwise
  verify the exact body strings in `--dry-run`.
- TUI smoke test: every section, safe-mode confirmation for MQTT and HTTP,
  offline command rejection, automatic reconnect after killing/restarting the
  broker connection, traffic filtering, `Ctrl+L` log pane, `Ctrl+T` traffic
  focus, theme editing with the color picker, help, quit popup, `Q` from every
  state, terminal resize and restore.
- Verify man pages render: `man -l /tmp/subraum-man/subraumctl.1`.

### Nix checks

```sh
nix build
nix flake check
nix develop
nix fmt
man --help   # after installing the package or via man -l
```

---

## 11. Git hygiene and commit rules

- `.gitignore` contains at least:

```gitignore
# Web UI files: specification only, never committed
/index.html
/kevyn.js
/paho-mqtt.js
/user.css
/user.js
/version.json

# Build output
/target/
/result
/result-*
/.direnv/
```

- The web files are never `git add`ed.
- Commits, when requested, use:

```sh
git -c user.name=Claude -c user.email=claude@anthropic.com commit -m "..."
```

  No `Co-Authored-By`, `Signed-off-by`, or other trailers.
- Planned commit sequence (only after the user asks):
  1. `docs: add implementation plan`
  2. `chore: scaffold cargo project and nix flake`
  3. `feat: add mqtt actions and cli commands`
  4. `feat: add man pages and completions`
  5. `feat: add redesigned ratatui tui`
  6. `feat: add safe mode, traffic pane and theme editor`
  7. `test: add payload, decoder and tui state tests`
  8. `chore: formatting and nix checks`

---

## 12. Implementation order

1. **Scaffold**: `Cargo.toml`, `.gitignore`, `src/main.rs`, `src/lib.rs`,
   `flake.nix`, module stubs, `README.md`.
2. **Domain layer**: `action`, `knob`, `white_light`, `decode`, payloads,
   destructive flags and unit tests.
3. **MQTT layer**: TCP connection options, callbacks, event stream, publish,
   subscribe, reconnect supervisor with backoff and generation handling.
4. **CLI**: clap structs, long help/man pages/completions, dispatch,
   output/dry-run/JSON, `status`/`publish`/`subscribe`/`monitor`.
5. **Smoothiesaur**: HTTP executor, relay/laser commands, status decoders.
6. **Color/theme**: CSS color parsing, theme roles/presets, CLI overrides.
7. **TUI foundation**: terminal guard, input thread, app state, header,
   sidebar, content frame, status line, footer, focus zones, traffic ring
   buffer, `tui-logger` app log pane, popups, quit semantics.
8. **TUI sections**: Overview, Lighting, White Light, Outlets, Alerts, System,
   Animation, Smoothiesaur, Theme.
9. **Safety and resilience**: safe-mode confirmation for MQTT/HTTP, offline
   rejection, automatic reconnect, destructive warnings, help overlay.
10. **Color picker**: keyboard RGB/HSV/HSL editing, named-color palette,
    live theme application.
11. **Polish**: terminal-too-small handling, error surfaces, spinner,
    keybinding audit, consistent styling.
12. **Nix**: finalize flake, man/completion installation, treefmt, checks and
    dev shell; document lockfile generation.
13. **Review**: confirm web files are ignored, all topics/payloads match the
    inventory, and commit only when requested.

---

## 13. Risks, assumptions and out of scope

- **Broker URI assumption**: the default is
  `tcp://webmqtt.subraum.c3pb.de:1883`, derived from the web UI's broker host
  with the normal MQTT port. Confirm or override with `--broker` /
  `SUBRAUMCTL_BROKER`.
- **No WebSockets**: only normal TCP MQTT (`tcp://`, optional `ssl://`) is
  supported. The `websockets` paho feature and all `ws://`/`wss://` handling
  are out of scope.
- **Component crate version alignment**: `tui-logger`, `tui-input`,
  `tui-popup`, `tui-widget-list`, `tui-scrollview` and
  `throbber-widgets-tui` must share ratatui 0.29 with the app. The first
  build may require pinning minor versions; if a component is incompatible,
  fall back to built-in ratatui widgets for that component after asking the
  user.
- **`tui-logger` tracing support**: the intended feature is `tracing-support`
  and the intended layer is its tracing subscriber layer. If the pinned
  version's API differs, adapt the logging module; if it cannot capture
  `tracing`, fall back to a manual ring buffer (only after asking).
- **`tui-popup` compatibility**: if the crate does not render correctly with
  ratatui 0.29, use `Clear` + a centered `Block` for popups.
- **Paho TCP build**: the crate uses the bundled C library plus OpenSSL. If
  the build fails, drop `bundled`, add `paho-mqtt-c` to the flake inputs and
  link the Nix-provided library.
- **Color alpha**: terminals have no alpha; `#rrggbbaa`/`rgba()` alpha is
  ignored with a warning. Theme colors are TUI-only; CLI output is not themed.
- **No offline queue**: commands attempted while disconnected are rejected
  visibly and never queued, to avoid stale hardware commands.
- **No build verification here**: code is written against documented APIs;
  the user performs the first compile and reports API drift.
- **Browser-only features**: drag layout editing, CSS zoom, fullscreen,
  localStorage layout persistence and URL tab persistence are intentionally
  not reimplemented.
- **No status feedback**: DMX, outlets, radiation, sob and telinit have no
  known status topics; the TUI shows last commanded values only, matching the
  web UI.
- **Safe mode is TUI-only**: non-interactive CLI commands remain scriptable
  and do not prompt. The CLI `--dry-run` provides a preview.
- **Out of scope**: WebSockets, authentication beyond MQTT username/password,
  TLS certificate pinning, layout editing, mouse support, persisted TUI
  settings/themes across runs, and web UI hosting.
