use gtk::{gdk::prelude::*, prelude::*};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

thread_local! { static CACHE: RefCell<HashMap<(&'static str, &'static str, i32), gtk::gdk_pixbuf::Pixbuf>> = RefCell::new(HashMap::new()); }

pub const CLOSE: &str = include_str!("window/close.svg");
pub const MINIMIZE: &str = include_str!("window/minimize.svg");
pub const MAXIMIZE: &str = include_str!("window/maximize.svg");
pub const RESTORE: &str = include_str!("window/restore.svg");
pub const PALETTE: &str = include_str!("actions/palette.svg");
pub const MORE: &str = include_str!("actions/more.svg");
pub const ADD: &str = include_str!("actions/add.svg");

fn render(svg: &str, color: &str, scale: i32) -> gtk::gdk_pixbuf::Pixbuf {
    let bytes = svg.replace("COLOR", color);
    let loader = gtk::gdk_pixbuf::PixbufLoader::new();
    loader.set_size(20 * scale, 20 * scale);
    loader
        .write(bytes.as_bytes())
        .expect("내장 SVG를 읽을 수 없습니다");
    loader.close().expect("내장 SVG를 렌더링할 수 없습니다");
    loader.pixbuf().expect("내장 SVG가 비어 있습니다")
}
pub fn image(svg: &'static str, white: bool) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(20);
    area.set_content_height(20);
    area.set_halign(gtk::Align::Center);
    area.set_valign(gtk::Align::Center);
    let pixbuf = Rc::new(RefCell::new(None::<gtk::gdk_pixbuf::Pixbuf>));
    let paint = pixbuf.clone();
    area.set_draw_func(move |_, cr, _, _| {
        if let Some(p) = paint.borrow().as_ref() {
            cr.save().expect("cairo save");
            let scale = 20.0 / f64::from(p.width());
            cr.scale(scale, scale);
            cr.set_source_pixbuf(p, 0.0, 0.0);
            let _ = cr.paint();
            cr.restore().expect("cairo restore");
        }
    });
    let update = Rc::new(move |area: &gtk::DrawingArea| {
        let color = if white {
            "#ffffff"
        } else if gtk::Settings::default()
            .as_ref()
            .is_some_and(super::theme::is_dark)
        {
            "#e2e6ec"
        } else {
            "#39414b"
        };
        let key = (svg, color, area.scale_factor());
        *pixbuf.borrow_mut() = Some(CACHE.with(|cache| {
            cache
                .borrow_mut()
                .entry(key)
                .or_insert_with(|| render(svg, color, key.2))
                .clone()
        }));
        area.queue_draw();
    });
    update(&area);
    let change = update.clone();
    area.connect_notify_local(Some("scale-factor"), move |area, _| change(area));
    if let Some(settings) = gtk::Settings::default() {
        let mut handlers = Vec::new();
        for property in ["gtk-application-prefer-dark-theme", "gtk-theme-name"] {
            let weak = area.downgrade();
            let change = update.clone();
            handlers.push(settings.connect_notify_local(Some(property), move |_, _| {
                if let Some(area) = weak.upgrade() {
                    change(&area);
                }
            }));
        }
        let handlers = RefCell::new(Some(handlers));
        area.connect_destroy(move |_| {
            if let Some(handlers) = handlers.borrow_mut().take() {
                for handler in handlers {
                    settings.disconnect(handler);
                }
            }
        });
    }
    area
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_001_all_icons_decode_at_both_scales() {
        for svg in [CLOSE, MINIMIZE, MAXIMIZE, RESTORE, PALETTE, MORE, ADD] {
            for scale in [1, 2] {
                let p = render(svg, "#39414b", scale);
                assert_eq!((p.width(), p.height()), (20 * scale, 20 * scale));
            }
        }
    }
}
