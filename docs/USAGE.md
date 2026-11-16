# CANdata User Guide

A practical manual for using CANdata — the BLF log viewer, signal plotter, and DBC/LDF library manager.

---

## Table of Contents

1. [Quick Start](#quick-start)
2. [Window Layout](#window-layout)
3. [Loading BLF Files](#loading-blf-files)
4. [Log View](#log-view)
5. [Plot View](#plot-view)
6. [Math Signals](#math-signals)
7. [Library Management](#library-management)
8. [Export & Import Libraries](#export--import-libraries)
9. [Status Bar Reference](#status-bar-reference)
10. [Keyboard Shortcuts](#keyboard-shortcuts)
11. [Persisted State](#persisted-state)
12. [Troubleshooting](#troubleshooting)

---

## Quick Start

1. **Launch** the app.
2. Click **File → Open BLF…** in the top bar and pick a `.blf` (or `.bin`) file.
   - Files larger than 1 GB trigger a confirmation dialog.
3. The default **Log** view shows decoded messages. Switch to **Plot** in the bottom-left of the status bar to plot signals.
4. On first plot, if no DBC/LDF library is active, the **Select signal library** overlay appears — pick a library version to decode signals.

For a fresh install with no libraries configured yet, go to **Library** (top bar) → **+ New Library**, add a version, then add channels pointing to your `.dbc` (CAN) or `.ldf` (LIN) files.

To compute new signals from decoded ones, see [Math Signals](#math-signals).

---

## Window Layout

```
┌────────────────────────────────────────────────────────────┐
│  File  Library  Math             Settings  Help   _  []  X │ ← Top bar (36px)
├────────────────────────────────────────────────────────────┤
│                                                            │
│                                                            │
│                  Content area                              │ ← Log / Plot / Library / Math
│                                                            │
│                                                            │
├────────────────────────────────────────────────────────────┤
│ Log Plot │ 📂 file │ msgs │ DBC │ LDF │ status │ 📚 lib │ log mode │ ← Status bar (24px)
└────────────────────────────────────────────────────────────┘
```

- **Top bar** — File menu, Library and Math buttons, Settings and Help menus, window controls.
- **Content area** — switches between Log / Plot / Library / Math.
- **Status bar** — file info, message/DBC/LDF counts, status messages, active library, view name, and the Log ⇄ Plot toggle.

---

## Loading BLF Files

Open the **File** dropdown in the top bar. Two options:

### Open BLF…

Replaces the currently loaded file with a single new one. Filtered to `.blf` / `.bin`. Files > 1 GB trigger a confirmation prompt.

### Open Multiple BLF…

Appends multiple files to the current session. Each file is loaded in parallel, then merged on a shared timeline. The total size limit (> 1 GB combined) triggers a confirmation prompt.

### During loading

- The status bar shows progress: `⏳ Loading X/Y files (N failed so far) — M messages`.
- An **❌ Cancel** button appears on the right of the status bar — clicking it cancels pending files (already-loaded ones stay).
- Per-file parse errors don't abort the whole batch — failed files appear in the **Loaded Files** popover with `❌` and an error list.

### Managing loaded files

Click the **file segment** in the status bar (left side) when ≥ 2 files are loaded, or when any file has parse errors, to open the **Loaded Files** popover:

- Each row shows: status icon (`✅` clean / `❌` errors), file name, message count, byte size, deduplicated error list.
- Click **✕** on a row to remove that file.
- **Remove All** (bottom-left) clears everything.
- **Done** (bottom-right) closes the popover.
- The list scrolls when more than 8 files are loaded; the popover shrinks to content size when ≤ 8 files.

---

## Log View

Default view after loading a BLF. Shows one row per CAN/LIN message.

### Columns

| Column   | Behavior |
|----------|----------|
| **#**    | Row index. |
| **TIME** | Absolute (`YYYY-MM-DD HH:MM:SS.ffffff`) when the BLF has a start time; relative seconds otherwise. |
| **CH**   | Channel number. Click ⚙ next to the header to open the channel filter dropdown; click ✓ to clear the filter. |
| **TYPE** | Color-coded: CAN/CAN2 green, CAN_ERR red, CAN_FD/CAN_FD64 purple, CAN_OV amber, LIN/LIN2 blue. |
| **ID**   | Message ID. The small pill next to the label shows the current base: **DEC** (decimal, e.g. `416`; the default) or **HEX** (e.g. `0x1A0`, at least three hex digits). Click the label or the pill to switch. The base applies to the log rows and the ID filter dropdown and resets to DEC when CANdata restarts. Click ⚙ to open the ID filter dropdown; when a filter is active the icon shows ✓, and clicking it clears the filter. |
| **DLC**  | Data length code. |
| **DATA** | Raw payload bytes. |

### Filtering

Two filters — **ID filter** and **Channel filter** — work the same way:

1. Click ⚙ next to the column header to open a dropdown listing every unique value.
2. Click a value to set the filter. For the ID filter you can also type the ID with the number keys; typed IDs are always read as **decimal**, even when the ID column shows HEX (so enter `416`, not `1A0`).
3. ✓ clears the filter. **Escape** closes the dropdown (and clears the filter if pressed again).

Both filters combine with AND when both are set.

### Empty state

Shows *"No log file loaded. Use File → Open BLF... to load a log file."*

---

## Plot View

Switch via the **Plot** button (bottom-left of the status bar). Renders one chart per selected signal.

### Signal sidebar

- Header shows signal count.
- **Search box** filters by signal name, message name, or ID.
- Items are grouped by channel (CAN blue / LIN yellow), then message, then signal. [Math signals](#math-signals) are listed first, under their own heading.
- Channels that have a configured library mapping but aren't loaded show a **Load** button — click to load that library version.
- Drag a signal onto a lane to plot it there, or double-click it to add it to the selected lane.
- A signal's square is filled with its line colour while it is plotted. If it is in several lanes, this is its colour in the first of them.
- Values use the decimal places the DBC's factor and offset call for: none for a factor of 1, one for a factor of 0.1.
- Units follow the signal name in brackets, such as `INV_Motor_Speed [rpm]`. For a DBC unit written as `physical property:unit`, such as `voltage:V`, only the unit is shown.
- Charts update as soon as a signal is added or removed.
- **Settings → Plot signal list** chooses between **Grouped by message** (the default) and **Alphabetical signals only**, one sorted list without message rows. Signals sharing a name show their channel and message ID. Search still matches message names and IDs. The choice is saved.

### Toolbar

- **Reset Zoom** — appears only when zoomed; restores the full time range.

### Zoom

- **Mouse wheel** — zoom in/out centered on the mouse when it's over a plot. Min range is bounded by the smallest gap between adjacent points across all series.
- **Drag to zoom** — click-drag horizontally on a chart to select a region. Selections > 10 px commit the zoom; smaller selections are ignored.
- **Double-click** — resets zoom.
- **Right-drag** — while zoomed in, pans through time at the same zoom level. It stops at the ends of the data.
- **Ctrl + wheel** — scrolls the lanes when they don't all fit. The wheel also scrolls them when the mouse is outside the plot area.

### Cursor

Click a chart to place the cursor, then fine-tune it with the keyboard:

- **← / →** move the cursor one pixel at the current zoom, so zooming in gives finer steps. Hold the key to keep moving; the longer it's held, the faster it goes, up to about a fortieth of the plot width per repeat.
- **Shift + ← / →** jump to the previous or next sample of the selected lane's signals (of every plotted signal if that lane is empty). Holding it speeds up too.
- While zoomed in, the view scrolls to keep the cursor in sight when it reaches either edge. The cursor stops at the ends of the data.
- If there is no cursor yet, the first arrow press puts it in the middle of the view.
- The arrow keys work after you click in the charts. Clicking a text box, such as the signal search, gives the keys back to that box.

The cursor stays where you put it until you move it or click somewhere else:

- Each chart draws a vertical line at the cursor, with a marker on the signal's latest sample at or before that time. The value appears in the chart's header.
- The signal list shows every signal's value at the cursor in a right-aligned column, whether or not it is plotted. A signal with no value has not been received by that time.
- The toolbar shows the cursor's exact time.

The cursor clears when a different log is loaded or the window is maximized or restored.

### Time display

X-axis labels use absolute `YYYY-MM-DD HH:MM:SS.mmm` when the BLF has a start time; otherwise relative seconds from the first message.

### Lanes

Signals are plotted in lanes, which share the available height and scroll once they reach their minimum height. All lanes use the same time axis, and each lane fits one Y range to all of its signals. Each signal has its own colour within a lane.

- Click a lane to select it; the selected lane has a blue border. Clicking inside a plot also sets the cursor, while clicking a lane's header doesn't.
- A signal can be in several lanes, but only once in each. Click **✕** next to a signal in a lane's header to remove it from that lane.
- **+ Add Lane** adds a lane at the bottom and selects it.
- **Delete Lane** deletes the selected lane and stops plotting its signals. The last lane is emptied instead of deleted.
- **↑ Move Up** and **↓ Move Down** swap the selected lane with its neighbour.
- **Points** shows or hides a dot at each sample in the selected lane. It's on for new lanes.
- A signal set saves the lanes with their signals, Y limits and Points setting, and loading it restores them. Math signals are saved by name. Sets saved before lanes existed open with one signal per lane.
- Up to 20 signals are plotted; a warning appears above the lanes beyond that.

### Y-axis range

The **Y** controls above the lanes apply to the selected lane. With **Auto** on, the lane fits everything visible and Min and Max show the range it chose. Turn Auto off to set your own limits: Min and Max start from the current range, and valid values apply as you type. Min must be less than Max; a message appears beside the boxes otherwise, and the lane keeps its last valid range. Anything outside the limits is cut off at the edge of the plot. Limits stay with the lane when it moves.

### Lane header

`{signal name} [{unit}] {value at cursor}` for each signal in the lane, in that signal's colour.

### Empty lanes

An empty lane shows a reminder to add a signal. A lane whose signals have no data in the log says so, and each of those signals is marked *(no data)* in its header.

---

## Math Signals

A Math signal is a new signal worked out from an equation of decoded signals, for example

```
Motor_Power = Motor_Speed * Motor_Torque / 9549
```

Math signals are plotted, read at the cursor and saved in signal sets like any other signal. They are global: one list for the whole app, not one per library. An equation is checked against whichever library is active.

### The Math screen

Click **Math** in the top bar (right of **Library**).

```
┌────────────────────────────────────────────┬────────────────────────────┐
│ Math signals  Import… Export… New Delete   │ Signals    [search____]    │
│ ✓ Motor_Power  kW  Motor_Speed * …         │ Math signals               │
│ ⚠ Wheel_Slip   %   (A - B) / B * 100       │   Motor_Power              │
│                                            │ Channel 1 (CAN)            │
│ Name [Motor_Power] Unit [kW] Decimals [1]  │   Motor (0x100)            │
│ ┌───────────────────────────────────┐      │     Motor_Speed   rpm      │
│ │ Motor_Speed * Motor_Torque / 9549 │      │     Motor_Torque  Nm       │
│ └───────────────────────────────────┘      │                            │
│ ✓ OK                                       │ Functions                  │
│ Time (•) Sample and hold ( ) Resample      │   abs(x)   Absolute value  │
│                       Revert  Save         │   sqrt(x)  Square root     │
└────────────────────────────────────────────┴────────────────────────────┘
```

- **List (top left).** Every Math signal with its unit and equation. The mark in front shows whether it can be computed with the active library: ✓ yes, ⚠ the equation reads fine but a name can't be used with this library, ✗ the equation can't be read. A `•` after the name means the editor has unsaved changes.
- **Editor (below the list).** Click a row to edit its name, unit, decimals, equation and time mode, then click **Save**, or **Revert** to go back to what is saved. If you click another row with unsaved changes, CANdata asks whether to discard them.
- **Signals (right).** Your other Math signals, then the signals of the active library. They are grouped by message or listed alphabetically, following **Settings → Plot signal list**, and the search box filters them. **Double-click** a signal to insert its name into the equation at the cursor.
- **Functions (bottom right).** Double-click a function to insert it with the cursor between its brackets.

**New** adds a Math signal named `Math_1`, `Math_2`, … and puts the cursor in **Name** so you can rename it. **Delete** asks first, and names the Math signals and signal sets that use the one being deleted.

### Decimals

**Decimals**, to the right of **Unit**, sets how many decimal places the plot shows for this Math signal's value at the cursor, in the lane header and the signal list. Enter a whole number from 0 to 9. New Math signals, and ones saved before this setting existed, show 3.

### Names

A Math signal name starts with a letter or `_` and contains only letters, digits and `_`. It must not match another Math signal (capitals don't count as different) or a function name. To rename, edit **Name** and click **Save**: plot lanes, saved signal sets and other Math signals' equations are updated to the new name.

### Writing equations

Use signal names exactly as they appear in the DBC/LDF, with no channel or message prefix. Spaces and line breaks are ignored.

| You write | Meaning |
|---|---|
| `+ - * /` | add, subtract, multiply, divide |
| `^` | power. `2^3^2` is `2^(3^2)` = 512, and `-2^2` is −4 |
| `( )` | brackets |
| `12`, `1.5`, `.5`, `1e-3` | numbers |
| `< <= > >= == !=` | comparisons: 1 when true, 0 when false. Only one at a time without brackets |
| `name(…)` | a function, see below |

Examples:

```
Motor_Speed * Motor_Torque / 9549
(Wheel_FL + Wheel_FR) / 2
if(Motor_Speed > 100, Motor_Torque, 0)
(Gear == 3) * Vehicle_Speed
lowpass(derivative(Vehicle_Speed / 3.6), 0.2)
Motor_Power / 1000          ← uses the Math signal Motor_Power
```

### Checking

The line under the equation updates as you type:

- **✓ OK**: every name is found and the Math signal can be plotted.
- **✗ …** — the equation can't be read, with where the problem is, such as `✗ A value is missing after '*' at column 14` (or `line 2, column 3` for equations on several lines). **Save** is off until it is fixed. An empty equation counts as one.
- **⚠ …** — the equation reads fine, but something can't be used with the active library:
  - `Unknown signal 'motor_speed' (did you mean 'Motor_Speed'?)`. Names must match exactly, including capitals.
  - `'Speed' is in more than one message (Ch1 0x200, Ch2 0x300)`. The name can't be told apart; equations use plain names only.
  - `Unknown function` or `sqrt takes 1 value, not 2 (sqrt(x))`.
  - `Math signal 'B' has a problem: …` when a Math signal it uses can't be computed.
  - `Loop: A → B → A` when Math signals use each other in a circle.
  - `'X' is both a signal and a Math signal`. Rename the Math signal.
- **⚠ Syntax OK. Activate a library to check the signal names.** when no library is active.

A ⚠ doesn't stop **Save**: the equation may be meant for another library.

### Functions

| Function | Gives |
|---|---|
| `abs(x)`, `sqrt(x)`, `exp(x)`, `ln(x)`, `log10(x)`, `round(x)` | the usual value of x |
| `sin(x)`, `cos(x)`, `tan(x)` | trigonometry, x in radians |
| `min(a, b, ...)`, `max(a, b, ...)` | smallest or largest of two or more values |
| `clamp(x, low, high)` | x kept between low and high |
| `if(condition, then, else)` | `then` where the condition is not 0, otherwise `else` |
| `derivative(x)` | change of x per second since the previous point |
| `integral(x)` | running total of x over time, starting at 0 at the first point |
| `lowpass(x, seconds)` | x smoothed by a first-order filter with that time constant. 0 or less passes x through |

### Time: lining up signals

Signals from different messages are sent at different times, so CANdata has to decide when an equation has a value. Choose per Math signal with **Time**:

- **Sample and hold** (default). A point whenever any input gets a new sample; every other input keeps its latest value. Signals from the same message line up exactly.
- **Resample** at a rate in Hz. A point every 1/rate seconds, with each input read by **Hold** (its latest value) or **Linear** (a straight line between the samples either side).

Either way, the Math signal starts once every input has received its first sample. Points that come out undefined, such as a division by zero or the square root of a negative number, are left out. A rate that would give more than 10 million points over the log is refused.

`derivative`, `integral` and `lowpass` use the spacing of the points. With sample and hold that spacing is uneven, so a resample rate usually gives smoother results for them.

### Plotting Math signals

In **Plot**, a **Math signals** group sits at the top of the signal list, in both list layouts, and the search box filters it. Drag a Math signal to a lane or double-click it, the same as any signal. A Math signal that can't be computed with the active library is marked **⚠ see Math**; plotted, it shows *(no data)*.

Saving a Math signal on the Math screen recomputes the plot, so changes show straight away, including in Math signals that use it.

### Export & import Math signals

- **Export…** saves every Math signal to one `.cvmath` file (the default name is `math_signals.cvmath`).
- **Import…** adds the Math signals from a `.cvmath` file. If some names are already used, a bar lists them and offers, for all of them at once:
  - **Replace**: the imported equations overwrite yours. The existing spelling of the name is kept, so plots and signal sets still match.
  - **Keep both**: the imported ones are added as `Name_2`, `Name_3`, ….
  - **Skip**: yours are kept and those imported ones are dropped.
  - **Cancel import**: nothing is added.

A `.cvmath` file is pretty-printed JSON: `format` (`"candata-math"`), `format_version` (currently `1`), `exported_at`, and `signals`, each with its name, unit, equation and time mode. Files from a newer version of CANdata, or with invalid or repeated names, are refused.

---

## Library Management

Switch via the **Library** button in the top bar. Three-column layout: libraries | versions | channels.

### Create a library

1. In the left column, click **+ New Library**.
2. Type a name and press **Enter** (or click ✓). **Escape** cancels.

The new library is selected straight away, and the **Add Version** box opens with the cursor in it, so you can type its first version name next.

### Add a version

1. Select a library on the left.
2. Click **+ Add Version** in the middle column.
3. Type a version name (e.g., `v1.0`) and press **Enter**.

The new version is selected straight away, and the **Add Channel** form opens with the cursor in the Channel ID box, so you can add its first channel next.

### Add a channel (DBC/LDF file)

1. Select a library and a version.
2. Click **+ Add Channel** in the right column.
3. Toggle the type (CAN ⇄ LIN) if needed.
4. Enter channel ID and channel name (the name auto-fills from the file name stem if left empty).
5. Click **Select File…** and pick a `.dbc` (CAN) or `.ldf` (LIN) file.
6. Click ✓ to save or ✕ to cancel. **Enter** saves, **Escape** cancels.

The database file is copied into `libraries/<lib>/<version>/<channel>/` next to the executable, so the original can be moved or deleted.

Database paths must be local files. A path that starts with `http://` or `https://` is shown in red and won't load. Re-add that channel from a local file.

### Activate a version

Click the ▷ button next to a version (becomes ▶ when active). Activating:

- Loads the DBC/LDF databases for all configured channels into memory.
- Updates the active library badge in the status bar (`📚 lib_name / version`).
- Persists the active state to config so it survives restarts.

Click ▶ to deactivate. The library picker overlay reappears in Plot view when no version is active.

### Rename / Delete

- **✎** — rename a library or version. Inline input with ✓ / ✕ confirmation.
- **🗑** — delete a library, version, or channel. No undo.
- **✎ on a channel** — edit its type, ID, name, or database file in place. Click ✓ to save or ✕ to cancel. Choosing a new file replaces the stored copy.

### Apply to Plot

Click **Apply to Plot** at the top of the right column to write the current version's channel mappings into the app config (without activating). This updates `app_config.mappings` and saves to disk.

---

## Export & Import Libraries

Move a library to another machine as a single `.cvlib` file. CANdata makes no network connections.

### Export

1. In the **Library** view, select the library in the left column.
2. Click **📤 Export** (bottom-left).
3. Choose where to save. The default name is `<library name>.cvlib`.

The file holds every version and channel of the library **and a copy of each DBC/LDF file**, so it is all the other machine needs. The export fails if any channel's database file is missing or uses a remote (`http://`) path.

### Import

1. In the **Library** view, click **📥 Import**.
2. Pick a `.cvlib` file.
3. The database files are copied into the local `libraries/` folder, the library appears in the list, and it is selected. If a library with the same name already exists, the imported one gets a `_1`, `_2`, … suffix; nothing existing is overwritten.

### File format

A `.cvlib` file is pretty-printed JSON: `format` (`"can-viewer-library"`), `format_version` (currently `1`), `exported_at`, and `libraries` with each version's channels. Each database file's bytes are stored base64-encoded, so files in any text encoding come back exactly as they were.

---

## Status Bar Reference

The 24-px bar at the bottom of the window.

### Left side

| Segment | Description |
|---|---|
| **Log / Plot toggle** | Switches between Log and Plot views. Active button gets the primary-color background. |
| **File** | `📂 No file loaded — File > Open BLF...` (empty), `📂 <name>` (1 file, no errors), `📂 <name> ⚠️` (1 file with errors), `📂 N files` (≥2 files, no errors), or `📂 N files ⚠️` (≥2 files with errors). Click the segment to open the Loaded Files popover when clickable. ⚠️ is yellow. |
| **BLF progress** | `consumed / total (pct)` while parsing. Yellow below 100%, secondary text at 100%. |
| **msgs** | Total loaded message count. |
| **DBC** | Number of loaded DBC databases. |
| **LDF** | Number of loaded LDF databases. |

### Right side

| Segment | Description |
|---|---|
| **status_msg** | Transient status text (Loading…, Ready, errors). Truncated if long. |
| **Library badge** | `📚 <lib_name> / <version>` when a library version is active. |
| **Cancel** | `❌ Cancel` shown only during loading. Click to abort pending loads. |
| **view name** | `log mode` / `plot mode` / `library mode` / `math mode`. |

### Loaded Files popover

Triggered by clicking the file segment when clickable. See [Loading BLF Files](#loading-blf-files) for details.

---

## Keyboard Shortcuts

CANdata has no global hotkey registry. Shortcuts are per-view.

### Log view

| Key | Action |
|---|---|
| `0`–`9` | Append digit to the ID filter text (works whether or not the dropdown is open). |
| `Backspace` | Delete last digit of the ID filter text; clears the ID filter if the text is empty. |
| `Enter` | Apply the ID filter. The typed text is read as a decimal ID, whatever base the ID column shows; hex digits A–F are not accepted. |
| `Escape` | Close the ID filter dropdown if open; otherwise clear the ID filter. |

### Library view

| Key | Context | Action |
|---|---|---|
| `Enter` | Add Library input | Create the library and close the input. |
| `Enter` | Add Version input | Add the version and close the input. |
| `Enter` | Add Channel input | Save the channel config. |
| `Escape` | Any add/rename input | Cancel without saving. |

### Plot view

- **Double-click** on a chart — reset zoom.
- **Mouse wheel** — zoom in/out.
- **Right-drag** — pan while zoomed in.
- **Ctrl + wheel** — scroll the lanes.
- **← / →** — move the cursor; hold to speed up.
- **Shift + ← / →** — move the cursor to the previous or next sample.

### Window controls

The minimize / maximize / close buttons in the top-right are mouse-only.

---

## Persisted State

### Saved to disk (`multi_channel_config.json`)

CANdata is portable: everything it saves lives in the folder that holds the executable, never in the working directory or your user profile. Copy that folder to move the whole setup. Pretty-printed JSON. Saved on:

- Create / delete library
- Add / delete version
- Add / delete channel
- Activate / deactivate version
- Apply to Plot
- After importing a `.cvlib` library file

Math signals are saved separately, in `math_signals.json`, each time one is created, saved, renamed, deleted or imported. A `math_signals.json` that can't be read is renamed to `math_signals.json.bad` rather than overwritten, so its equations aren't lost.

Contents:

- `libraries` — full library / version / channel hierarchy.
- `mappings` — channel-to-database mappings (type, ID, file path, library ID, version name).
- `active_library_id`, `active_version_name` — the currently active version.

### Library files on disk

DBC/LDF files are copied into `libraries/<lib>/<version>/<channel>/` when you add a channel or import a library. Saved plot signal sets are in `signal_sets.json` and Math signals in `math_signals.json`. All of these sit next to the executable:

```
<exe folder>/
├── candata.exe
├── multi_channel_config.json
├── signal_sets.json
├── math_signals.json
└── libraries/
```

Because of this, install CANdata somewhere you can write to (the default per-user install is fine; `Program Files` for all users is not).

### In-memory only (not persisted)

Loaded BLF messages, decoded DBC/LDF databases, plot zoom state, and selected signals survive a window maximize/restore (saved in `RuntimeState`), but are **not** written to disk — they're lost when the app quits.

---

## Troubleshooting

### No messages after loading

- Check the file size — files > 1 GB may need confirmation.
- Open the Loaded Files popover (click the file segment) and look for `❌` icons — the file may have parse errors.

### ⚠️ appears next to the file name

The file loaded but had parse errors. Click the segment to open the Loaded Files popover and read the error list under the offending file.

### Signals don't decode in Plot view

- A library version must be **activated**, not just configured. Check the status bar's library badge.
- If the **Select signal library** overlay appears, pick a version or click ✕ to dismiss.
- Click **Load** next to unloaded channels in the signal sidebar to load their library version.

### A Math signal shows *(no data)* or **⚠ see Math**

- Open **Math** and select it: the line under the equation says what's wrong, such as an unknown or ambiguous signal name, or a Math signal it uses that has a problem.
- Names must match the DBC/LDF exactly, including capitals, and the library that has them must be active.
- If the equation shows ✓ OK but the plot is still empty, one of its signals has no samples in the loaded log.

### Import fails

- The file must be a `.cvlib` file exported by CANdata (or by can-viewer), or a `.cvmath` file for Math signals. A newer format version than this build understands is rejected.
- Check the status message at the bottom for the reason.

### A channel path is red: "Remote paths are not supported"

The channel points at an `http://` or `https://` URL, probably left over from the old LAN sharing feature. CANdata only reads local files. Edit the channel (✎) and pick a local `.dbc`/`.ldf` file.

### Changes don't persist across restarts

Library / version / channel changes and active state are saved automatically. Loaded BLF files and plot state are not — reload the file after restarting.
