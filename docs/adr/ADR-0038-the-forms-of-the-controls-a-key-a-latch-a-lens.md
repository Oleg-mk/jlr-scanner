# ADR-0038: The forms of the controls — a key, a latch, a lens

- Status: **Accepted, 2026-09-19** — on the owner's word («так, ти абсолютно
  все вірно зрозумів, робимо»), from his own sketch of four forms.
- Scope: the interface only. Nothing below reaches a protocol, a transport
  or the knowledge base; the architecture guard is untouched.

## Context

On the plate every control is the same capsule: a button that does
something, a switch that stays where it is put, and a badge that only
reports are drawn alike — a hairline pill with a word in it. The owner,
walking the running window on 2026-09-19: «все це оформлено однаково, і
для того щоб зрозуміти що в якому стані потрібно концентрувати увагу». He
drew four forms — a raised pill, a sunk pill, a pill with a rim round a
flat face, a pill in a bezel — and named them: the first two a switch,
released and engaged; the third an indicator; the fourth a button.

The plate already has a physical grammar — a sheet with windows cut into
it, lit from above, keys milled rather than printed (2026-09-13). The
sketch is the same grammar carried into the controls: what can be pressed
stands proud, what is engaged sits lower, what can only be looked at is a
window.

## Decision

Three classes of control, each with a form of its own, and depth used for
these three and nothing else.

1. **A key — a momentary button.** A pill standing in a bezel, the double
   rim of the owner's fourth form. It sinks only while the pointer is down
   and comes back up. The one action that carries a block forward is inked
   in the accent; the rest in the plate's ink. A key that cannot be pressed
   lies flat, lit lower, as the disabled rule of 2026-09-13 already says.
   Examples: *Start*, *Stop*, *Read*, *Survey modules*, *Check all modules*,
   *Save*, *Choose these*, *Detect adapter*, *New session*.

2. **A latch — a switch that stays.** Released, a raised pill; engaged, the
   same pill sunk into the surface, and a lamp lit inside it. Two channels
   for one state, because depth alone is a thin difference on a dense
   screen and none at all to a tired eye. The state is the markup's own:
   `aria-pressed` or `aria-expanded` is the switch's position, and the sheet
   draws whatever the attribute says. Examples: *Instrument panel*, *Show
   all / Only informative*, *Quantities / Everything*, the captions under the
   dials, *Show / Put away* on the four long reads, the help language, the
   service mode — whose engaged colour stays red, as `ADR-0036` says.

3. **A lens — an indicator.** A recessed window with a rim, flat inside,
   the dot carrying the state as it does today. It does not react to the
   pointer, and it must not look as if it would. Examples: every status
   badge (*Connected*, *Reading…*, *Not started*, *Synthetic*), the bench
   badge with its scenario, and the count ring on a key.

4. **Not moved.** Fields and selects stay the sunk capsules they are; the
   language puck is already a physical slider; the map's nodes are a map;
   the marks in table rows (*Table*, *Chart*) and everything inside tables
   stays flat under hairlines. A screen with forty modules must not become
   an embossed carpet: depth is reserved for the three classes above.

The palette does not change: the plate's ink, the accent for the main
action and the needles, red for the bench and the service mode, amber and
red only where a limit is crossed.

## Inventory, at the day of the decision

| Control | Class | Where |
| --- | --- | --- |
| Start, Stop, Read, Survey modules, Check all modules, Mileage, Module passports, Configuration (CCF), Save, Clear marks, Mark all for the chart, Choose these / Choose all, Decode VIN, Detect adapter, Connect, Disconnect, New session | key | every panel, the header |
| Instrument panel; Show all / Only informative; Quantities / Everything; Show / Put away the passports, the readings, the configuration; the fault list's show/hide; the help language eng/rus; the dial captions; the service mode | latch | the live read panel, the four long reads, the module panel, the header |
| Library, Adapter, Bench badges in the header; the section badges; every panel badge; the count ring | lens | the header, the section headings, the panels |
| Fields, selects, the bench scenario box, the language puck, the map nodes, the table marks, the session steps | unchanged | — |

## Consequences

- One layer at the foot of the stylesheet, scoped to the plate, draws the
  three forms; no markup changes, because the state attributes the forms
  read were already there. The bench badge keeps its red and gains the
  lens's depth; the service switch keeps its red and gains the latch's.
- A new control declares its class by what it is: a `button` with no state
  attribute is a key, one with `aria-pressed` or `aria-expanded` is a latch,
  a `status-badge` is a lens. Nothing else is needed to fall into the
  grammar, and nothing else is allowed to leave it.
- The forms are judged at the screen, with the owner, the way every visual
  decision of this plate was; what he corrects there is corrected in the
  same layer.

## Built, 2026-09-19 — `IMPLEMENTED`

The layer, the same afternoon; the walk-through with the owner is what
decides whether it stays as drawn.
