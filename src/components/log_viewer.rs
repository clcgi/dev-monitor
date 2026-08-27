use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct LogViewerProps {
    pub logs: Vec<String>,
}

#[component]
pub fn LogViewer(props: LogViewerProps) -> Element {
    // We need to automatically scroll to the bottom when logs are updated
    // But for a simple implementation, we can just rely on flex-direction column-reverse
    // or manual scrolling. We'll render them inside a scrollable div.
    
    rsx! {
        div {
            class: "log",
            style: "flex: 1; min-height: 200px; display: flex; flex-direction: column; overflow-y: auto;",
            id: "log-container",
            
            for line in props.logs.iter() {
                div { "{line}" }
            }
        }
    }
}
