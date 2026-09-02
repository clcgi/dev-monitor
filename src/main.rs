#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::LogicalSize;
use dioxus::prelude::*;

mod components;
mod screens;
mod services;

use components::update_banner::UpdateUi;

/// The Tailwind build output.
const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");

/// The four palettes.
pub const THEMES: [(&str, &str); 4] = [
    ("theme-electric-autumn", "Electric Autumn"),
    ("theme-warm-editorial", "Warm Editorial"),
    ("theme-seasonless-blue", "Seasonless Blue"),
    ("theme-digital-romance", "Digital Romance"),
];

/// The palette the app starts in, and currently the only one it uses.
pub const DEFAULT_THEME: &str = "theme-seasonless-blue";

#[cfg(not(target_arch = "wasm32"))]
fn window_config(title: &str) -> dioxus::desktop::Config {
    dioxus::desktop::Config::new().with_resource_directory(std::env::current_dir().unwrap().join("assets")).with_window(
        dioxus::desktop::WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(1024.0, 768.0)),
    )
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let cfg = window_config(concat!("DEV Monitor ", env!("CARGO_PKG_VERSION")));
        LaunchBuilder::desktop().with_cfg(cfg).launch(App);
    }
    #[cfg(target_arch = "wasm32")]
    {
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    let mut system_is_light = use_signal(|| dark_light::detect() != dark_light::Mode::Dark);
    // None follows the OS; Some(_) is the user's explicit choice.
    let mut theme_preference = use_signal(|| Option::<bool>::None);
    // Must match a block in input.css, or the default palette renders silently.
    let theme = use_signal(|| DEFAULT_THEME.to_string());
    use_context_provider(|| theme);

    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            let detected =
                tokio::task::spawn_blocking(|| dark_light::detect() != dark_light::Mode::Dark)
                    .await
                    .unwrap_or(*system_is_light.read());
            // Written unconditionally: it records what the OS says, which is read only.
            if detected != *system_is_light.read() {
                system_is_light.set(detected);
            }
        }
    });

    // One request at startup, never repeated.
    let mut update = use_signal(|| UpdateUi::Hidden);
    // Outlives the banner. The check runs once per process, so without this a
    // user who misses the five seconds cannot reach Install again until restart.
    let mut pending = use_signal(|| Option::<services::updates::Update>::None);
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        let Some(found) = services::updates::check().await else { return };
        pending.set(Some(found.clone()));
        update.set(UpdateUi::Offered(found));
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        // Only an untouched offer auto-hides. A download in flight, a failure, or
        // an installer already handed over all outlive the five seconds.
        if matches!(*update.read(), UpdateUi::Offered(_)) {
            update.set(UpdateUi::Hidden);
        }
    });

    // Download, then hand the file to the OS installer.
    let install = move |u: services::updates::Update| {
        update.set(UpdateUi::Downloading(u.clone()));
        spawn(async move {
            let Some(installer) = u.installer.clone() else {
                update.set(UpdateUi::Failed(u, "no installer for this platform".into()));
                return;
            };
            match services::updates::download(&installer).await {
                Err(why) => update.set(UpdateUi::Failed(u, why)),
                Ok(path) => match services::updates::launch(&path) {
                    Err(why) => update.set(UpdateUi::Failed(u, why)),
                    Ok(()) => update.set(UpdateUi::Handed(u)),
                },
            }
        });
    };

    // Auth reminder coroutine: every 12 hours
    let mut show_auth_reminder = use_signal(|| true); // Show on startup
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
            show_auth_reminder.set(true);
        }
    });

    let stylesheet: &str = TAILWIND_CSS;
    let is_light = theme_preference.read().unwrap_or(*system_is_light.read());
    let theme_class = if is_light {
        format!("{} light", theme.read())
    } else {
        theme.read().clone()
    };

    rsx! {
        // The stylesheet is RENDERED, not injected by script.
        style { "{stylesheet}" }

        // The theme lives on a RENDERED wrapper, not on <body> set by script.
        div {
            // `relative` so the update banner can position against the window rather than.
            class: "cdw-root {theme_class} relative h-screen",
            components::update_banner::UpdateBanner {
                ui: update.read().clone(),
                current: services::updates::current_version().to_string(),
                on_install: install,
                on_open_page: move |u: services::updates::Update| { let _ = open::that(&u.url); },
                on_dismiss: move |_| update.set(UpdateUi::Hidden),
            }
            screens::main_window::MainWindow {
                show_auth_reminder,
                pending_update: pending.read().clone(),
                on_show_update: move |u| update.set(UpdateUi::Offered(u)),
                theme_preference: *theme_preference.read(),
                system_is_light: *system_is_light.read(),
                on_theme_change: move |pref| theme_preference.set(pref),
            }
        }
    }
}
