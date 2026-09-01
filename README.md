# DEV Ingestion Monitor

A native desktop application built with [Dioxus](https://dioxuslabs.com/) and [Tokio](https://tokio.rs/) for testing and monitoring the DEV ingestion workflow in the Central Document Warehouse (CDW). 

This application provides a controlled UI for automatically discovering, triggering, and monitoring the execution of existing workflow scripts without requiring any changes to the scripts themselves.

## Features

- **Automated Script Discovery**: Dynamically scans `../CentralDocumentWarehouse/tools/` for `.py` and `.sh` scripts.
- **Environment Targeting**: Scripts are explicitly run against `sandbox`, `dev`, or `stg`. The application seamlessly passes this to the scripts by exporting `CDW_ENV` and sourcing `deploy/00-variables.sh`.
- **Live Visual Monitoring**: Streams `stdout` and parses `[CDW_STEP: ...]` and `[CDW_RESULT: ...]` markers to drive an animated 11-step pipeline stepper and results panel.
- **Background Execution**: Prevents UI blocking by executing scripts asynchronously.
- **Authentication Reminders**: Prompts users to authenticate with Azure (`az login`) every 12 hours.
- **Theme Support**: Features 4 semantic color palettes with dynamic light/dark mode based on the system theme.

## Prerequisites

- **Rust & Cargo**: Required for building and running the application.
- **Azure CLI**: Required for script authentication (`az login`).
- **Directory Structure**: 
  ```text
  CDW/
  ├── CentralDocumentWarehouse/
  │   ├── tools/
  │   └── deploy/
  └── dev-monitor/      <-- (You are here)
  ```

## Script Declarations

Every script in `tools/` describes itself in a header comment:
```python
# CDW_SCRIPT: category=Flows; steps=Neo,Authentication,Apim,Landing; summary=...
# CDW_ARG: --apply  Actually delete. WITHOUT IT THIS IS A DRY RUN.
```

- `category`: Groups scripts in the sidebar (`Flows`, `Verification`, `Simulation`, `Maintenance`).
- `steps`: Declares which stages the UI stepper draws. 
- `summary`: Provides a tooltip/description.
- `CDW_ARG:`: Offers a specific flag as a toggle switch in the UI. **No flag is on by default.**

## Workflow Flows

`tools/flow_*.py` drive the real DEV pipeline through the real gateway.

| Flow | Edge | What a pass means |
|---|---|---|
| `flow_1_park` | `LAND → GATE → PEND` | An unregistered business key parks in `PendingMetadata` and the bytes stay in landing |
| `flow_2_promote` | `LAND → GATE → RAW` | A real register key resolves and the bytes **move** into raw |
| `flow_3_extract` | `RAW → EXT → ROLE` | The archive is dispatched, the Container Apps Job runs, and members are catalogued |
| `flow_4_size_guard` | *(API guard)* | A wrong declared size is refused **422 synchronously**, writing nothing |
| `flow_5_quarantine` | `LAND → GATE → QUA` | Bytes contradicting a declared size are quarantined by the reconciler |
| `flow_6_rejected` | `LAND → GATE → REJ` | Both permanently-invalid arrivals are rejected |
| `flow_all` | all of the above | Runs 1–6 in order, one process each |

## Credentials

The flows require two values managed in `deploy/.dev.env` (gitignored):
```sh
export CDW_SUBSCRIPTION_KEY=...   # the APIM subscription key
export CDW_CLIENT_SECRET=...      # the test client's secret
```
The monitor automatically sources `deploy/00-variables.sh` before executing scripts.

## Running the Application

### Desktop Application (Default)
To build and run the native desktop application locally, use Cargo:
```bash
cd dev-monitor
cargo run
```

### Web Application (Localhost)
To serve the application over localhost, use the Dioxus CLI (`dx`):
```bash
dx serve --platform web
```
*(Note: Some system-level features like script execution are disabled in the web browser due to security sandboxing).*

## Building for Multiple Environments (Cross-Platform)

Dioxus relies on native webview libraries (WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux). While `cargo build --release` will compile the raw binary executable, you need the **Dioxus CLI** to package the app into a native installer (e.g., a `.dmg` or `.app` for Mac, or a `.msi`/`.exe` installer for Windows).

First, ensure the Dioxus CLI is installed:
```bash
cargo install dioxus-cli
```

### Mac (macOS)
To create a native Mac `.app` bundle and a distributable `.dmg` disk image:
```bash
dx bundle --release --platform desktop
```
*(Troubleshooting: If you get an error like "Unable to choose binary for bundle", your machine likely has another tool named `dx` installed via Homebrew taking precedence in your PATH. If this happens, run `~/.cargo/bin/dx bundle --release --platform desktop` to force using the Dioxus CLI).*

The output bundles will be placed in `target/dx/dev-monitor/release/bundle/`. 
*(Note: To cross-compile for both Intel and Apple Silicon Macs, you will need to add the respective rust targets and build on macOS).*

### Windows
To create a Windows `.msi` installer or executable bundle, run this on a Windows machine:
```powershell
dx bundle --release --platform desktop
```
*Note: The target machine must have the [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) installed (pre-installed by default on Windows 11).*

### Linux
To bundle for Linux (creates `.deb` or AppImage depending on your setup), run on a Linux machine with `libwebkit2gtk-4.1-dev` installed:
```bash
dx bundle --release --platform desktop
```

## Styling & Themes

The UI uses **Tailwind CSS v4** alongside a legacy hand-written stylesheet. 
To build or watch Tailwind styles:
```bash
npm install
npm run build:css    # Builds input.css -> assets/tailwind.css
npm run watch:css    # Rebuilds on change
```
*(Note: Node.js is only required for modifying styles. You can build and run the Rust app without it).*

The application supports four interconnected color palettes, defaulting to **Seasonless Blue**. It automatically detects your system's light/dark mode. You can switch palettes using the picker in the top bar.

Icons are provided by Phosphor and fetched dynamically from `unpkg.com` at runtime.
