//! 포인터·키보드를 캔버스 조작으로 해석한다. 한 번에 조작 하나만 진행한다.
pub(super) mod card;
pub(super) mod keyboard;
pub(super) mod pointer;

use super::{model::layout::Rect, CanvasView};
use std::rc::Rc;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Interaction {
    Idle,
    Pan {
        last: (f64, f64),
    },
    Marquee {
        anchor: (f64, f64),
        base: Vec<Uuid>,
        rect: Option<Rect>,
    },
    Move {
        grab: (f64, f64),
        origins: Vec<(Uuid, [f64; 2])>,
        moved: bool,
        press: Press,
    },
    Link {
        from: Uuid,
        end: (f64, f64),
        target: Option<Uuid>,
    },
}

/// 끌지 않고 놓았을 때 선택을 어떻게 바꿀지.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Press {
    Keep,
    Collapse(Uuid),
    Toggle(Uuid),
}

/// 시스템 드래그 임계값을 사용해 터치패드의 미세한 흔들림을 클릭으로 처리한다.
pub(super) fn drag_threshold() -> f64 {
    gtk::Settings::default()
        .map(|settings| f64::from(settings.gtk_dnd_drag_threshold().max(1)))
        .unwrap_or(8.)
}

impl Interaction {
    pub fn is_idle(&self) -> bool {
        matches!(self, Interaction::Idle)
    }
    pub fn follows_pointer(&self) -> bool {
        matches!(
            self,
            Interaction::Marquee { .. }
                | Interaction::Link { .. }
                | Interaction::Move { moved: true, .. }
        )
    }
    pub fn hides_port(&self) -> bool {
        matches!(
            self,
            Interaction::Pan { .. }
                | Interaction::Marquee { .. }
                | Interaction::Move { moved: true, .. }
        )
    }
}

pub(super) fn attach(v: &Rc<CanvasView>) {
    pointer::attach(v);
    keyboard::attach(v);
}
