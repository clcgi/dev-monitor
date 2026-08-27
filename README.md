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

## Theme Configuration

The application supports four interconnected color palettes. The default is **Electric Autumn**. The application automatically detects your system's light/dark mode and applies the correct shade variations for the active palette.

You can change the active palette by modifying the `<body>` tag class. For example, if it's set in the root `index.html` file or in the main component `src/main.rs`.

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
