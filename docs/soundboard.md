# Soundboard

Der Tab **Soundboard** sucht verfügbare Samples mit `sobfind <Suchtext>` und
löst einen ausgewählten Treffer mit `sob <Sample>` aus. Beide Befehle gehen
direkt an EDI über AMQP, wie die Pizzamat-Anbindung. Es werden keine lokalen
Shell-Befehle ausgeführt. Die bisherigen vier SOB-Aktionen unter **Alerts**
bleiben unverändert auf MQTT.

## Bedienung

```sh
subraumctl tui --view soundboard
# Offline-Vorschau ohne Netzwerkzugriffe:
subraumctl --dry-run tui --view soundboard --no-connect
```

- Im Suchfeld Text eingeben und **Enter** drücken. Erst Enter sendet die Suche;
  es gibt keine automatische Suche beim Tippen und kein Polling.
- **Esc** beendet die Texteingabe, **/** öffnet sie erneut. Im Suchfeld gelten
  `q`, `Q`, `c`, `d`, `r` usw. als Text; **Ctrl+C** beendet weiterhin die App.
- **↑/↓**, **PgUp/PgDn**, **Home/End** wählen einen Treffer bzw. das Suchfeld.
- **Enter** auf einem Treffer löst genau diesen Sample-Namen aus. Leerzeichen
  bleiben erhalten. Im standardmäßig aktiven Safe Mode erscheint vorher die
  Bestätigung, mit **Cancel** als Vorauswahl.
- **Ctrl+T** fokussiert außerhalb der Texteingabe das Traffic-Fenster.

Suchtext, Ergebnisse, leere Trefferlisten, laufende Anfragen und Fehler werden
angezeigt. Es ist höchstens eine Anfrage gleichzeitig aktiv. Fehlgeschlagene
Befehle werden nicht automatisch wiederholt. `--no-connect` deaktiviert den
automatischen MQTT-Verbindungsaufbau, verhindert aber keine expliziten
Soundboard-Anfragen. `--dry-run` verhindert auch AMQP-Zugriffe.

## Verbindung

| Option | Environment | Standard |
|---|---|---|
| `--soundboard-amqp-url` | `SUBRAUMCTL_SOUNDBOARD_AMQP_URL` | `amqp://amqp.subraum.c3pb.de:5672/%2f` |
| `--soundboard-timeout` | `SUBRAUMCTL_SOUNDBOARD_TIMEOUT` | 15 Sekunden (1–120) |

Die Einstellungen sind unabhängig vom MQTT-Broker sowie den Heizungs- und
Pizzamat-Endpunkten. Das Timeout umfasst Verbindungsaufbau, Publish und
Suchantwort gemeinsam; das Schließen kann zusätzlich bis zu zwei Sekunden
benötigen.

## EDI-Protokoll

Exchange `cmd`, Routing-Key `sobfind` bzw. `sob`:

```json
{"cmd":"sobfind","args":"Suchtext","user":"subraumctl","src":"subraumctl.recv.<private-queue>","dst":null}
```

Vor dem Publish wird eine private exklusive Auto-delete-Queue an Exchange
`msg` gebunden. Die Antwortadresse ersetzt `recv` durch `send`. Antworten
werden über Exchange und Routing-Key zugeordnet, nicht über das JSON-Feld
`user`. Der String `msg` enthält die Kopfzeile `Matched <Zahl> files:` und
anschließend einen Treffer pro Zeile (`\n`); die Kopfzeile und leere Zeilen
werden übersprungen. Maximal 64 KiB pro Antwort und 500 Treffer werden
akzeptiert; weitere Treffer werden sichtbar als gekürzt markiert.

`sob` wartet nur auf Mandatory-Routing und die Publisher-Bestätigung des
Brokers, nicht auf eine möglicherweise ausbleibende Dienstantwort. Die
Bestätigung bedeutet **nicht**, dass ein Sample hörbar abgespielt wurde.
Bei einem Timeout nach dem Publish kann der Befehl bereits ausgeführt worden
sein — deshalb kein automatisches Replay.

## Tests

Tests decken Eingabevalidierung, EDI-Payloads, Antwortzuordnung und -grenzen,
Dry-run ohne Netzwerk, Sucheingabe, Trefferwahl, Safe Mode und Darstellung ab.
Die Protokolltests verwenden synthetische Antworten; ein Live-Test am
Soundboard-Dienst ist nicht Bestandteil von `nix flake check`.
