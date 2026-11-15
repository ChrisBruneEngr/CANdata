# Math signals

Date: 2026-11-15

## Goal

Let the user define new signals as equations of existing ones, for example

```
Motor_Power = Motor_Speed * Motor_Torque
```

and plot them like any decoded signal. Equations are typed as plain math text
(`+ - * / ^`, brackets, numbers, signal names and, later, functions), checked
as they are typed, and saved for reuse.

## Decisions

- **Plain signal names.** An equation names signals exactly as they appear in
  the DBC/LDF (`Motor_Speed`), with no channel or message prefix. DBC and LDF
  signal names are already identifiers (`[A-Za-z_][A-Za-z0-9_]*`), so no
  quoting is needed.
- **Global Math signals.** Math signals are not tied to a library or version.
  They are kept in one file, `math_signals.json`, in the app folder next to
  `signal_sets.json` (`crate::config::paths::app_dir()`). Names are resolved
  against the active library every time the equation is checked or evaluated.
- **Time alignment: sample-and-hold by default, resample as an option.** Each
  Math signal chooses its own time mode (see *Evaluation*). Resample mode
  offers hold or linear interpolation.
- **Exportable.** Math signals move between machines as `.cvmath` files (see
  *Export and import*).
- **Hand-written parser, no new dependency.** It is small, and it gives
  messages that point at the exact column of the error.
- **Delivered in phases**, one feature branch and PR each (see *Phases*).

## The Math screen

A new `AppView::MathView`, opened by a **Math** button in the top bar to the
right of **Library** (`ui/components/top_bar.rs`). It gets its own bottom
indicator, like Library. The view lives in `ui/views/math_view.rs`.

```
┌───────────────────────────────────────┬──────────────────────────┐
│ Math signals   [New] [Delete] [⇅]    │ Signals     [search____] │
│ ┌───────────────────────────────────┐ │ ▸ CH1  Powertrain        │
│ │ Motor_Power        kW         ✓   │ │   ▸ EngineMsg (0x100)    │
│ │ Wheel_Slip         %          ⚠   │ │       Motor_Speed        │
│ │ ...                               │ │       Motor_Torque       │
│ └───────────────────────────────────┘ │ ▸ CH2  Chassis           │
│                                       │                          │
│ Name [Motor_Power__]  Unit [kW__]     │ Functions   (phase 4)    │
│ ┌───────────────────────────────────┐ │   abs(x)                 │
│ │ Motor_Speed * Motor_Torque / 9549 │ │   sqrt(x)                │
│ │                                   │ │   min(a, b)              │
│ └───────────────────────────────────┘ │   ...                    │
│ ✓ OK   – or –                         │                          │
│ ✗ Unknown signal 'Motr_Speed' (col 1) │                          │
│                                       │                          │
│ Time  (•) Sample and hold             │                          │
│       ( ) Resample at [100__] Hz      │                          │
│           [Hold ▾ / Linear]           │                          │
│                     [Revert] [Save]   │                          │
└───────────────────────────────────────┴──────────────────────────┘
```

- **Math signal list (top left).** One row per Math signal: name, unit, and a
  status mark. ✓ means the equation is valid for the active library. ⚠ means it
  names a signal the active library does not have, or has more than once. Click
  a row to load it into the editor. To rename, edit the Name field and Save.
- **Editor (below the list).** Name, Unit, a multi-line equation input
  (gpui-component `InputState`), a status line, the time mode, and Save /
  Revert. The equation is checked on every change. Save is disabled while the
  equation has an error or the name is invalid. Leaving a row with unsaved
  changes asks whether to discard them.
- **Signal browser (right).** The signals of the active library, with search,
  laid out as chosen in Settings → Plot signal list: grouped by channel and
  message, or one alphabetical list (shared names marked with channel and ID). It reuses the item building from `plot_sidebar.rs`
  (`extract_signal_items`) where it can. Double-clicking a signal inserts its
  plain name at the editor cursor.
- **Functions list (right, phase 4).** Double-clicking a function inserts
  `name()` and puts the cursor between the brackets.
- **No active library.** The browser shows a hint to activate one. Math signals
  can still be edited, but every equation is marked ⚠.

## Data model

New module `library/math_signals.rs`, following the pattern of `signal_sets.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathSignal {
    pub name: String,
    #[serde(default)]
    pub unit: String,
    pub equation: String,
    #[serde(default)]
    pub time_mode: TimeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum TimeMode {
    #[default]
    SampleAndHold,
    Resample { rate_hz: f64, #[serde(default)] interpolation: Interpolation },
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Interpolation {
    #[default]
    Hold,
    Linear,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MathSignalStore {
    #[serde(default)]
    pub signals: Vec<MathSignal>,
}
```

Only the equation text is saved. It is parsed again on load, so a file edited
by hand, or one from a newer version, is still checked.

**Name rules.** A Math signal name must be an identifier, must be unique among
Math signals (ignoring case), and must not be a function name. It may match a
decoded signal name. In that case the plot sidebar tells them apart by group
(see below). Equations always refer to decoded signals until phase 5.

## Equation language

Grammar, lowest precedence first:

```
expr    := term   (('+' | '-') term)*
term    := unary  (('*' | '/') unary)*
unary   := '-' unary | power
power   := primary ('^' unary)?          // right-associative: 2^3^2 = 2^9
primary := NUMBER | NAME | NAME '(' args? ')' | '(' expr ')'
args    := expr (',' expr)*
NUMBER  := 12 | 1.5 | .5 | 1e-3 | 2.5E+4
NAME    := [A-Za-z_][A-Za-z0-9_]*
```

Whitespace is ignored. Comparisons (`< <= > >= == !=`) and `if(cond, a, b)`
are added in phase 4, with comparisons at a lower precedence than `+ -`.

The parser (`math/parser.rs`) produces an AST:

```rust
enum Expr {
    Number(f64),
    Signal(String),
    Neg(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
```

Every error carries a character range, so the status line can show the column
and, later, the editor can underline it. The errors:

| Error                         | Example                 |
| ----------------------------- | ----------------------- |
| Unexpected character          | `Speed $ 2`             |
| Missing operand               | `Speed *`               |
| Unbalanced bracket            | `(Speed + 1`            |
| Unknown signal                | `Motr_Speed`            |
| Ambiguous signal (2+ matches) | `Speed` in CH1 and CH2  |
| Unknown function / wrong args | `sqrt(a, b)`            |
| Empty equation                |                         |

Syntax is checked on its own (`parse`). Names are checked separately
(`resolve`) against the active library, so an equation for another vehicle
still parses and is shown with ⚠ instead of ✗.

Only syntax errors (✗, including an empty equation) stop Save. Name problems
(⚠) don't, because Math signals are global and an equation may be meant for
another library. A name that is only wrong in capitals gets a hint:
`Unknown signal 'motor_speed' (did you mean 'Motor_Speed'?)`.

**Duplicate signal names.** With plain names, a name found in more than one
message or channel of the active library cannot be resolved. It is reported as
*ambiguous*, together with where it was found. Qualified names are left out on
purpose. If duplicates turn out to be common, a qualified form can be added
later without changing equations that already work.

## Evaluation

New module `math/eval.rs`. Inputs: the AST, plus the decoded `(time, value)`
points of each signal it names. Output: a `Series` with the Math signal's name
and unit.

**Sample and hold (default).** The output times are the union of the input
times. At each time, every input holds its most recent value. Output starts at
the first time where every input has a value. Signals from the same message
line up exactly, and nothing is invented between real samples.

**Resample at N Hz.** The output times are a fixed grid with a step of `1/N`
seconds. The grid runs from the first time where every input has a value to the
last input sample. Inputs are sampled on the grid with the chosen
interpolation:

- *Hold:* the last value at or before the grid time.
- *Linear:* a straight line between the samples either side of the grid time.
  After an input's last sample, its last value is held. This gives evenly spaced output, which phase 4's
derivative and integral functions will need. The rate must be > 0. The editor
warns when a rate would produce more than about 10 million points over the
loaded log.

**Numbers.** Arithmetic is `f64`. Division by zero, `sqrt` of a negative and
similar give NaN or ±inf. Those points are dropped from the plotted series and
are not treated as errors.

**Where it runs.** In `chart_view::extract_and_update_series_data`, after
decoded series are built. A selected Math signal finds its input signals in
the decoded log. If they are not decoded already, it decodes them, then
evaluates.

## Plotting and signal sets

- The plot sidebar gets a **Math** group above the channels, listing Math
  signals valid for the active library. Invalid ones are shown greyed, with
  the reason as a tooltip.
- A plotted Math signal's id in `selected_signals` is `MATH:<name>`, next to
  the existing `CAN:` / `LIN:` ids.
- `SignalSetEntry` gets an optional field `math_name: Option<String>`
  (`#[serde(default, skip_serializing_if = "Option::is_none")]`). When set, the
  entry refers to that Math signal and `channel_id` / `msg_id` are ignored.
  Older files load unchanged. Older app versions skip the entries they cannot
  resolve.
- Renaming a Math signal updates the entries in saved signal sets that refer
  to it. Deleting one asks first if a signal set uses it.

## Export and import

The ⇅ menu on the Math screen offers **Export…** and **Import…**, using the
native file dialogs (`rfd`), as library export does.

- **Export** writes the selected Math signals (or all of them) to one `.cvmath`
  file. It is a JSON document with a format tag and version, like `.cvlib`:

  ```json
  { "format": "candata-math", "version": 1,
    "signals": [ { "name": "Motor_Power", "unit": "kW",
                   "equation": "Motor_Speed * Motor_Torque / 9549",
                   "time_mode": { "mode": "SampleAndHold" } } ] }
  ```

- **Import** reads a `.cvmath` file and adds its signals. A signal whose name
  is already used is offered as *Replace*, *Keep both* (imported as
  `Name_2`), or *Skip*. Imported equations are checked like any other.
- The code lives in `library/math_signals.rs` (file format) and
  `app/commands/math_transfer.rs` (dialogs), mirroring `bundle.rs` and
  `library_transfer.rs`.

## Phases

1. **Math screen and storage.** `MathView`, the top-bar button, the
   `math_signals.json` store with load and save, list / new / rename / delete,
   the editor fields, the signal browser and insert on double-click. The status
   line shows only whether the equation is empty.
2. **Parser and checking.** `math/parser.rs`, name resolution against the
   active library, the live status line, ✓ / ⚠ in the list, and Save disabled
   on errors. Unit tests for precedence, associativity, number formats, and
   every error type and column.
3. **Evaluation and plotting.** `math/eval.rs` with both time modes, the time
   mode controls in the editor, the Math group in the plot sidebar, and
   `math_name` in signal sets. Unit tests for alignment (different rates,
   late-starting inputs, resample grid edges, hold and linear, NaN dropping).
4. **Export and import.** `.cvmath` files, with name conflict handling. Unit
   tests for round trip, bad files and conflicts.
5. **Functions.** The functions list. `abs sqrt exp ln log10 sin cos tan min
   max clamp`, comparisons and `if`. Then the time-based functions
   (`derivative`, `integral`, `lowpass`), which use the point history.
6. **Math of Math (optional).** Equations may name other Math signals. Evaluate
   in dependency order, and report loops (`A` uses `B` uses `A`) as an error.

## Settled questions

- Math signals are exportable (see *Export and import*).
- Resample mode offers linear interpolation as well as hold.
