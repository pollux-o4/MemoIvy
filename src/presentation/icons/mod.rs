use gtk::{gdk::prelude::*, prelude::*};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

type Key = (&'static str, i32, i32);
thread_local! { static CACHE: RefCell<HashMap<Key, gtk::gdk_pixbuf::Pixbuf>> = RefCell::new(HashMap::new()); }

pub const CLOSE: &str = include_str!("window/close.svg");
pub const MINIMIZE: &str = include_str!("window/minimize.svg");
pub const MAXIMIZE: &str = include_str!("window/maximize.svg");
pub const RESTORE: &str = include_str!("window/restore.svg");
pub const PALETTE: &str = include_str!("actions/palette.svg");
pub const MORE: &str = include_str!("actions/more.svg");
pub const ADD: &str = include_str!("actions/add.svg");
pub const TRASH: &str = include_str!("actions/trash.svg");
pub const KEYBOARD: &str = include_str!("actions/keyboard.svg");
pub const SOURCE: &str = include_str!("actions/source.svg");
pub const UNDO: &str = include_str!("canvas/undo.svg");
pub const REDO: &str = include_str!("canvas/redo.svg");
pub const ZOOM_IN: &str = include_str!("canvas/zoom-in.svg");
pub const ZOOM_OUT: &str = include_str!("canvas/zoom-out.svg");
pub const FIT: &str = include_str!("canvas/fit.svg");
pub const CHECK: &str = include_str!("canvas/check.svg");
pub const LINK: &str = include_str!("canvas/link.svg");
pub const OPEN: &str = include_str!("canvas/open.svg");
pub const SIGNAL: &str = include_str!("canvas/signal.svg");
pub const HELP: &str = include_str!("canvas/help.svg");
pub const ALIGN_LEFT: &str = include_str!("align/left.svg");
pub const ALIGN_CENTER_X: &str = include_str!("align/center-x.svg");
pub const ALIGN_RIGHT: &str = include_str!("align/right.svg");
pub const ALIGN_TOP: &str = include_str!("align/top.svg");
pub const ALIGN_CENTER_Y: &str = include_str!("align/center-y.svg");
pub const ALIGN_BOTTOM: &str = include_str!("align/bottom.svg");
pub const DISTRIBUTE_X: &str = include_str!("align/distribute-x.svg");
pub const DISTRIBUTE_Y: &str = include_str!("align/distribute-y.svg");
pub const TIDY: &str = include_str!("align/tidy.svg");
pub const FLOW: &str = include_str!("align/flow.svg");
pub const CANVAS_EMPTY: &str = include_str!("illustrations/canvas-empty.svg");
pub const LIBRARY_EMPTY: &str = include_str!("illustrations/library-empty.svg");

fn render(svg: &str, width: i32, height: i32) -> gtk::gdk_pixbuf::Pixbuf {
    // 아이콘은 알파만 쓰고 색은 CSS `color`로 입힌다.
    let bytes = svg.replace("COLOR", "#000000");
    let loader = gtk::gdk_pixbuf::PixbufLoader::new();
    loader.set_size(width, height);
    loader
        .write(bytes.as_bytes())
        .expect("내장 SVG를 읽을 수 없습니다");
    loader.close().expect("내장 SVG를 렌더링할 수 없습니다");
    loader.pixbuf().expect("내장 SVG가 비어 있습니다")
}

fn cached(svg: &'static str, width: i32, height: i32) -> gtk::gdk_pixbuf::Pixbuf {
    CACHE.with(|cache| {
        cache
            .borrow_mut()
            .entry((svg, width, height))
            .or_insert_with(|| render(svg, width, height))
            .clone()
    })
}

fn area(width: i32, height: i32) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_halign(gtk::Align::Center);
    area.set_valign(gtk::Align::Center);
    area
}

/// 현재 CSS 글자색을 따르는 20px 아이콘. 호버·비활성·테마 변경을 스타일만으로 표현한다.
pub fn image(svg: &'static str) -> gtk::DrawingArea {
    let icon = area(20, 20);
    let pixbuf = Rc::new(RefCell::new(None::<gtk::gdk_pixbuf::Pixbuf>));
    let paint = pixbuf.clone();
    icon.set_draw_func(move |area, cr, _, _| {
        if let Some(p) = paint.borrow().as_ref() {
            let scale = 20.0 / f64::from(p.width());
            cr.scale(scale, scale);
            cr.set_source_pixbuf(p, 0.0, 0.0);
            let mask = cr.source();
            let color = area.color();
            cr.set_source_rgba(
                f64::from(color.red()),
                f64::from(color.green()),
                f64::from(color.blue()),
                f64::from(color.alpha()),
            );
            let _ = cr.mask(&mask);
        }
    });
    let update = move |area: &gtk::DrawingArea| {
        let size = 20 * area.scale_factor();
        *pixbuf.borrow_mut() = Some(cached(svg, size, size));
        area.queue_draw();
    };
    update(&icon);
    icon.connect_notify_local(Some("scale-factor"), move |area, _| update(area));
    icon
}

/// 자체 색을 가진 삽화.
pub fn illustration(svg: &'static str, width: i32, height: i32) -> gtk::DrawingArea {
    let picture = area(width, height);
    picture.set_draw_func(move |area, cr, _, _| {
        let scale = area.scale_factor();
        let p = cached(svg, width * scale, height * scale);
        cr.scale(1. / f64::from(scale), 1. / f64::from(scale));
        cr.set_source_pixbuf(&p, 0.0, 0.0);
        let _ = cr.paint();
    });
    picture
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_001_all_icons_decode_at_both_scales() {
        for svg in [
            CLOSE,
            MINIMIZE,
            MAXIMIZE,
            RESTORE,
            PALETTE,
            MORE,
            ADD,
            TRASH,
            KEYBOARD,
            SOURCE,
            UNDO,
            REDO,
            ZOOM_IN,
            ZOOM_OUT,
            FIT,
            CHECK,
            LINK,
            OPEN,
            SIGNAL,
            HELP,
            ALIGN_LEFT,
            ALIGN_CENTER_X,
            ALIGN_RIGHT,
            ALIGN_TOP,
            ALIGN_CENTER_Y,
            ALIGN_BOTTOM,
            DISTRIBUTE_X,
            DISTRIBUTE_Y,
            TIDY,
            FLOW,
        ] {
            for scale in [1, 2] {
                let p = render(svg, 20 * scale, 20 * scale);
                assert_eq!((p.width(), p.height()), (20 * scale, 20 * scale));
            }
        }
        assert_eq!(render(CANVAS_EMPTY, 168, 112).width(), 168);
        assert_eq!(render(LIBRARY_EMPTY, 96, 80).width(), 96);
    }
}
