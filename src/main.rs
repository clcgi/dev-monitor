#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::LogicalSize;
use dioxus::prelude::*;

mod components;
mod screens;
mod services;

/// The Tailwind build output. Baked in rather than loaded at runtime so
/// `cargo run` needs nothing but Rust -- Node builds this file, it does not run
/// the app. Regenerate with `npm run build:css` after changing markup or
/// `input.css`.
const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");

/// The four palettes, in the order the picker offers them. The strings are the
/// body classes `input.css` defines; a name here with no matching block there
/// silently renders the default, so the two lists are one contract.
pub const THEMES: [(&str, &str); 4] = [
    ("theme-electric-autumn", "Electric Autumn"),
    ("theme-warm-editorial", "Warm Editorial"),
    ("theme-seasonless-blue", "Seasonless Blue"),
    ("theme-digital-romance", "Digital Romance"),
];

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
    let system_light = dark_light::detect() != dark_light::Mode::Dark;
    let mut is_light = use_signal(|| system_light);
    // The palette was unreachable before: the CSS defined four, and nothing in
    // the app ever set the class that selects one.
    let theme = use_signal(|| THEMES[0].0.to_string());
    use_context_provider(|| theme);

    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            let detected =
                tokio::task::spawn_blocking(|| dark_light::detect() != dark_light::Mode::Dark)
                    .await
                    .unwrap_or(*is_light.read());
            if detected != *is_light.read() {
                is_light.set(detected);
            }
        }
    });

    // Auth reminder coroutine: every 12 hours
    let mut show_auth_reminder = use_signal(|| true); // Show on startup
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
            show_auth_reminder.set(true);
        }
    });

    let stylesheet: &str = TAILWIND_CSS;
    let theme_class = if *is_light.read() {
        format!("{} light", theme.read())
    } else {
        theme.read().clone()
    };

    rsx! {
        // The stylesheet is RENDERED, not injected by script.
        //
        // It used to go in through `document::eval`, and that had two failure
        // modes stacked on each other: the returned `Eval` future was dropped
        // so only the last call of a loop survived, and the CSS was interpolated
        // into a JS template literal where `.sm\:px-4` -- Tailwind's escaping for
        // a variant -- was read as a backslash escape and arrived as `.sm:px-4`,
        // matching nothing. Neither produced an error; the UI simply rendered
        // unstyled, which pointed at Tailwind rather than at the injector.
        //
        // A `style` element in the tree has neither problem: Dioxus puts the
        // text in verbatim, there is no JavaScript in the path, and it exists
        // before the first paint rather than one effect later.
        style { "{stylesheet}" }

        // The theme lives on a RENDERED wrapper, not on <body> set by script.
        //
        // `document::eval` was writing `document.body.className`, which meant
        // the palette depended on a script running -- the same mechanism that
        // had already failed silently for the stylesheet. As an attribute it is
        // part of the tree: it cannot be dropped, and it is correct on the first
        // paint rather than one effect later.
        //
        // `cdw-root` carries the legacy variable aliases; `theme-*` and `light`
        // select the palette. h-screen so the wrapper fills the window, since
        // the variables must be on an ancestor of everything that reads them.
        div {
            class: "cdw-root {theme_class} h-screen",
            screens::main_window::MainWindow {
                show_auth_reminder,
            }
        }
    }
}
