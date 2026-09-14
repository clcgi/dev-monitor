pub const CYAN: &str = "#17703C";
pub const AMBER: &str = "#906E10";
pub const CORAL: &str = "#C00000";
pub const VIOLET: &str = "#16559A";
pub const ICE: &str = "#2E8B8B";
pub const DIM: &str = "#6B7176";
pub const GHOST: &str = "#C7CCD0";

pub const BG: &str = "#F5F5F5";
pub const CARD: &str = "#FFFFFF";
pub const CARD_SOFT: &str = "#F7F8F9";
pub const CARD_MUTED: &str = "#F4F5F6";
pub const BORDER: &str = "#E3E6E8";
pub const RULE: &str = "#E3E6E8";
pub const ROW_RULE: &str = "#EDEFF1";
pub const DASH: &str = "#D9DCDF";

pub const TEXT: &str = "#1A1A1A";
pub const TEXT_STRONG: &str = "#1A1A1A";
pub const TEXT_BODY: &str = "#4A4A4A";
pub const TEXT_SOFT: &str = "#33373B";
pub const TEXT_CELL: &str = "#33373B";

pub const SELECTED_BG: &str = "#FBEDE2";
pub const OK_BG: &str = "#E8F5EC";
pub const OK_BD: &str = "#4CAF50";
pub const OK_TINT: &str = "#E8F5EC";
pub const WARN_BG: &str = "#FDF4E3";
pub const WARN_BD: &str = "#DC7C38";
pub const BAD_BG: &str = "#FBEAEA";
pub const BAD_BD: &str = "#DC3545";
pub const BAD_INK: &str = "#C00000";
pub const INFO_BG: &str = "#E8EFFC";

// Daylight brand: the SBM logo's orange and blue.
pub const ORANGE: &str = "#F36F27";
pub const ORANGE_MID: &str = "#DC7C38";
pub const ORANGE_DEEP: &str = "#B85C28";
pub const BLUE: &str = "#16559A";
pub const TEAL: &str = "#2E8B8B";
pub const GRAD_DOC: &str = "linear-gradient(135deg,#F36F27,#DC7C38 46%,#B85C28)";
pub const GRAD_WAIT: &str = "linear-gradient(135deg,#DC7C38,#B85C28)";
pub const GRAD_DONE: &str = "linear-gradient(135deg,#2E8B8B,#1B6B6B)";
pub const GRAD_BAD: &str = "linear-gradient(135deg,#DC3545,#A31220)";

pub const MONO: &str = "var(--font-mono)";
pub const SANS: &str = "var(--font-text)";

/// Keyframes, prefixed so nothing in the Developer Monitor can collide with them.
pub const KEYFRAMES: &str = "\
@keyframes cdwm-pulse{0%,100%{opacity:1;transform:scale(1)}50%{opacity:.35;transform:scale(.82)}}\
@keyframes cdwm-fade{0%,100%{opacity:1}50%{opacity:.45}}\
@keyframes cdwm-sweep{0%{transform:translateX(-100%)}100%{transform:translateX(400%)}}\
@keyframes cdwm-caret{0%,49%{opacity:1}50%,100%{opacity:.15}}\
@keyframes cdwm-spin{to{transform:rotate(360deg)}}\
.cdwm-root ::-webkit-scrollbar{width:10px;height:10px}\
.cdwm-root ::-webkit-scrollbar-track{background:transparent}\
.cdwm-root ::-webkit-scrollbar-thumb{background:#D9DCDF;border-radius:9px}\
.cdwm-root button:hover{filter:brightness(.97)}\
.cdwm-logo svg{width:132px;height:auto;display:block}\
.cdwm-root .cdwm-trace:hover{background:#B85C28 !important;border-color:#B85C28 !important;filter:none}\
.cdwm-row:not(.cdwm-selected):hover{background:#F7F8F9 !important}\
@media (prefers-reduced-motion: reduce){.cdwm-root *{animation:none !important}}";
