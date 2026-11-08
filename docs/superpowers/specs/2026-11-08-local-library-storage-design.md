# Local library storage, export/import (replaces LAN sharing)

Date: 2026-11-08

## Goal

Remove all network access from can-viewer. Libraries are stored only on the
local disk, next to the executable, and move between machines as files.

## Decisions

- **Portable storage only.** Everything lives in the folder that holds the
  executable (`crate::config::paths::app_dir()`): `multi_channel_config.json`,
  `signal_sets.json`, and `libraries/<lib>/<version>/<channel>/<file>`. The
  old fallback to the current working directory is gone. No per-user
  profile folder (`%APPDATA%`, `~/.config`, ...) is used.
- **No migration** of configs found in the working directory or elsewhere.
- **Remote database paths are errors.** A `database_path` that starts with
  `http://` or `https://` is rejected when loading, validating, saving a
  channel, or exporting (`crate::library::is_remote_path`). The Library view
  shows such a path in red. Old configs are not cleaned up automatically.
- **Export / import** replaces Share / Import-from-URL. See below.

## Removed

- `src/view/src/server/` (axum HTTP server, token auth, LAN IP detection,
  `import_from_url` over reqwest).
- `src/view/src/app/commands/server.rs`, the share and import-URL dialogs,
  the status-bar server segment, the per-channel "copy URL" button, and the
  related `CanViewerApp` fields.
- Dependencies: `axum`, `tower-http`, `reqwest`, `if-addrs`, `uuid`, `tokio`
  (direct; `rfd` still pulls its own).
- `src/view/src/config/startup.rs` (unused, looked in the working directory).

## `.cvlib` library file

One JSON document (`crate::library::bundle`):

```json
{
  "format": "can-viewer-library",
  "format_version": 1,
  "exported_at": "2026-11-08 14:03:12",
  "libraries": [
    { "name": "...", "channel_type": "CAN",
      "versions": [
        { "name": "v1.0", "date": "...", "description": "...",
          "default_file": null,
          "channels": [
            { "channel_type": "CAN", "channel_id": 1, "channel_name": "PT",
              "file": { "file_name": "pt.dbc", "content_base64": "..." } } ] } ] } ]
}
```

- File bytes are base64 so any text encoding round-trips exactly.
- `default_file` carries a version's single `path` for versions that have no
  per-channel databases.
- The format allows several libraries per file; **Export** writes the selected
  library only.
- **Import** decodes everything before writing any file, writes the databases
  under `libraries/`, renames on a name clash (`name_1`, `name_2`, ...), adds
  the libraries, saves the config, and selects the first imported library.
- A file with a different `format`, or a newer `format_version`, is rejected.

## Not covered

- Exporting several or all libraries at once (format supports it; no UI).
- Exporting signal sets or channel mappings.
