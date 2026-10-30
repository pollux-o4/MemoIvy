mod chrome;
pub(in crate::app) mod focus;
mod menus;
pub(in crate::app) mod shortcuts;
pub(in crate::app) use chrome::header;
pub(in crate::app) use menus::{
    lazy_menu_for, menu_for, menu_popover, menu_row, message, source_row,
};
pub(in crate::app) const COLORS: [(&str, &str); 5] = [
    ("yellow", "노랑"),
    ("pink", "분홍"),
    ("blue", "파랑"),
    ("green", "초록"),
    ("purple", "보라"),
];
