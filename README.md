# DEV Ingestion Monitor

A native desktop application built with [Dioxus](https://dioxuslabs.com/) and [Tokio](https://tokio.rs/) for testing and monitoring the DEV ingestion workflow in the Central Document Warehouse (CDW). 

This application provides a controlled UI for automatically discovering, triggering, and monitoring the execution of the existing workflow scripts without requiring any changes to the scripts themselves.

## Features

- **Automated Script Discovery**: Dynamically scans `../CentralDocumentWarehouse/tools/` for `.py` and `.sh` scripts.
- **Environment Targeting**: A strict environment selection requirement ensures scripts are explicitly run against `sandbox`, `dev`, or `stg`. The application seamlessly passes this to the scripts by exporting `CDW_ENV` and sourcing `deploy/00-variables.sh`.
- **Live Monitoring**: Streams `stdout` and `stderr` directly into a responsive, terminal-like log viewer in the UI.
- **Background Execution**: Prevents UI blocking by executing scripts asynchronously via `tokio::process::Command`.
- **Authentication Reminders**: Reminds users to authenticate with Azure (`az login`) every 12 hours.
- **Multi-Palette Design System**: Features 4 distinct semantic color palettes, dynamically switching between light and dark modes based on the system theme.

## Prerequisites

- **Rust & Cargo**: Required for building and running the application.
- **Azure CLI**: Required for script authentication (`az login`).
- **Directory Structure**: This application expects to be located side-by-side with the `CentralDocumentWarehouse` directory.
  ```text
  CDW/
  ├── CentralDocumentWarehouse/
  │   ├── tools/
  │   └── deploy/
  └── dev-monitor/      <-- (You are here)
  ```

## Running the Application

### Desktop Application (Default)
To build and run the native desktop application locally, use Cargo:

```bash
cd dev-monitor
cargo run
```

### Web Application (Localhost)
Dioxus allows running the application directly inside your web browser on a local development server. To serve the application over localhost, use the Dioxus CLI (`dx`):

```bash
cd dev-monitor
dx serve --platform web
```
*Note: Ensure you have the `dioxus-cli` installed (`cargo install dioxus-cli`). When running on the web platform, certain system-level features like script execution (`tokio::process`) will be simulated or disabled due to browser security sandboxing.*

## Styling — Tailwind CSS v4

The UI is migrating from a hand-written stylesheet to Tailwind v4 utilities.
**Both stylesheets are live during the migration**: `assets/tailwind.css` is injected
first, `assets/main.css` second so its rules still win for components not yet
converted. As a component moves, its rules are deleted from `main.css`.

```bash
npm install          # once; Node is a BUILD dependency only
npm run build:css    # input.css -> assets/tailwind.css
npm run watch:css    # rebuild on change, while you work
```

**`cargo run` still needs nothing but Rust.** The generated CSS is baked in with
`include_str!`, and `assets/tailwind.css` is committed for exactly that reason —
a contributor without Node can build and run the app, they just cannot restyle it.

**Re-run `npm run build:css` after changing markup.** Tailwind scans `src/**/*.rs`
for class names and emits only what it finds; a new utility that has not been
generated is simply an unknown class, with no error anywhere.

## Theme Configuration

The application supports four interconnected color palettes. The default is **Electric Autumn**. The application automatically detects your system's light/dark mode and applies the correct shade variations for the active palette.

**Pick one from the palette button in the top bar.** Previously this required editing
the source and rebuilding — and in fact could not work at all: the app only ever set
`light` on `<body>`, while every light rule is written `body.theme-x.light` and needs
both classes. Light mode was therefore inert, and 8 of the 9 palette blocks were
unreachable. Both are fixed; the palettes themselves are unchanged.

The available palette classes are:
1. `theme-electric-autumn` (Default)
2. `theme-warm-editorial`
3. `theme-seasonless-blue`
4. `theme-digital-romance`

**Example: Switching to Warm Editorial**
Modify the body class:
```html
<body class="theme-warm-editorial">
```

### Forcing Light Mode
If you wish to force the application into light mode regardless of the system theme, simply append the `.light` class alongside your chosen theme:
```html
<body class="theme-electric-autumn light">
```

## Icons — Phosphor

Icons are Phosphor web-font classes (`ph ph-database`), referenced from Rust in
`workflow_stepper.rs` and elsewhere. The font is imported at the top of `input.css`.

**Open finding: they are fetched from `unpkg.com` at runtime.** This is a desktop
application that currently cannot draw its own icons without internet access — while a
complete vendored copy already sits in `assets/phosphor/`, unused. Two ways to close it:

| Option | Cost |
|---|---|
| Point `input.css` at `assets/phosphor/*/style.css` | Free, but relative font URLs resolve against the document, not the injected `<style>` — needs checking against the desktop asset root |
| Inline `Phosphor.woff2` as a `data:` URI | Always works offline, adds **~192 KB** (base64 of 144 KB) to the binary |

Not chosen here: it trades binary size against a network dependency, which is a call
for whoever owns the distribution.
