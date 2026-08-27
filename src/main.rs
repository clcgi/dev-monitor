#[cfg(not(target_arch = "wasm32"))]
use dioxus::desktop::LogicalSize;
use dioxus::prelude::*;

mod components;
mod screens;
mod services;

const MAIN_CSS: &str = include_str!("../assets/main.css");

#[cfg(not(target_arch = "wasm32"))]
fn window_config(title: &str) -> dioxus::desktop::Config {
    dioxus::desktop::Config::new().with_window(
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

    use_effect(move || {
        let css = MAIN_CSS.replace('`', "\\`").replace("${", "\\${");
        document::eval(&format!(
            "if(!document.getElementById('ga-css')){{var s=document.createElement('style');\
             s.id='ga-css';s.textContent=`{}`;document.head.appendChild(s);}}",
            css
        ));
    });

    use_effect(move || {
        let cls = if *is_light.read() { "light" } else { "" };
        document::eval(&format!("document.body.className = '{}';", cls));
    });

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

    rsx! {
        screens::main_window::MainWindow {
            show_auth_reminder,
        }
    }
}
