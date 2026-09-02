//! "A newer version is available", shown briefly at the top of the window.

use dioxus::prelude::*;

use crate::services::updates::Update;

/// What the banner is showing.
///
/// The auto-hide applies to `Offered` ONLY. Once a download starts the banner
/// has to stay: a 25 MB fetch outlives five seconds, and hiding it mid-transfer
/// would leave the app writing a file the user cannot see or act on.
#[derive(Clone, PartialEq, Debug)]
pub enum UpdateUi {
    Hidden,
    Offered(Update),
    Downloading(Update),
    /// Downloaded and handed to the OS installer.
    Handed(Update),
    Failed(Update, String),
}

#[derive(Props, Clone, PartialEq)]
pub struct UpdateBannerProps {
    pub ui: UpdateUi,
    pub current: String,
    pub on_install: EventHandler<Update>,
    pub on_open_page: EventHandler<Update>,
    pub on_dismiss: EventHandler<()>,
}

#[component]
pub fn UpdateBanner(props: UpdateBannerProps) -> Element {
    let (update, detail, busy) = match &props.ui {
        UpdateUi::Hidden => return rsx! {},
        UpdateUi::Offered(u) => (u.clone(), format!("You are running {}", props.current), false),
        UpdateUi::Downloading(u) => (u.clone(), "Downloading…".to_string(), true),
        UpdateUi::Handed(u) => (
            u.clone(),
            if crate::services::updates::must_quit_to_install() {
                "Installer started — close this app to finish".to_string()
            } else {
                "Opened — drag the app into Applications".to_string()
            },
            false,
        ),
        UpdateUi::Failed(u, why) => (u.clone(), why.clone(), false),
    };

    let failed = matches!(props.ui, UpdateUi::Failed(..));
    let handed = matches!(props.ui, UpdateUi::Handed(..));
    // No installer for this platform, or the release did not attach one: the
    // release page is the only useful destination.
    let can_install = update.installer.is_some() && !handed;

    // Computed here: rsx infers a two-branch `if/else` on an attribute but not a
    // three-way chain, and the error points at the whole block rather than the
    // chain.
    let icon = if failed {
        "ph-fill ph-warning-circle text-lg text-danger"
    } else if busy {
        "ph ph-spinner ph-spin text-lg text-brand"
    } else {
        "ph-fill ph-arrow-circle-up text-lg text-brand"
    };

    rsx! {
        div {
            class: "pointer-events-none absolute inset-x-0 top-0 z-50 flex justify-center px-3 pt-2",
            div {
                class: "pointer-events-auto flex w-full max-w-xl items-center gap-3 rounded-lg \
                        border border-brand bg-elevated px-3 py-2 shadow-lg animate-pop",
                i { class: "{icon}" }
                div { class: "min-w-0 flex-1 leading-tight",
                    div { class: "text-xs text-fg",
                        "Version "
                        span { class: "font-semibold text-brand", "{update.version}" }
                        " is available"
                    }
                    div {
                        class: if failed { "truncate text-[10px] text-danger" }
                               else { "truncate text-[10px] text-fg-faint" },
                        title: "{detail}",
                        "{detail}"
                    }
                }
                if can_install {
                    button {
                        r#type: "button",
                        disabled: busy,
                        class: "shrink-0 rounded-md border border-brand-deep bg-brand-deep px-2.5 \
                                py-1 text-[11px] text-white hover:border-brand hover:bg-brand \
                                disabled:cursor-default disabled:opacity-50",
                        onclick: {
                            let u = update.clone();
                            move |_| props.on_install.call(u.clone())
                        },
                        if failed { "Retry" } else { "Install" }
                    }
                }
                // Always reachable. The download can fail, the platform may have
                // no installer, and some people would rather read the notes.
                button {
                    r#type: "button",
                    class: "shrink-0 rounded-md border border-edge px-2.5 py-1 text-[11px] \
                            text-fg-soft hover:bg-active hover:text-fg",
                    title: "Open the release page",
                    onclick: {
                        let u = update.clone();
                        move |_| props.on_open_page.call(u.clone())
                    },
                    "Notes"
                }
                button {
                    r#type: "button",
                    class: "shrink-0 rounded p-1 text-fg-faint hover:bg-active hover:text-fg",
                    title: "Dismiss",
                    onclick: move |_| props.on_dismiss.call(()),
                    i { class: "ph ph-x text-xs" }
                }
            }
        }
    }
}
