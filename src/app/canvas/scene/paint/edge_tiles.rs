use gtk::{cairo, gdk, glib, graphene, prelude::*};
use std::collections::HashMap;

/// 확대된 월드 좌표에 고정한 타일 한 변(논리 픽셀).
pub(super) const TILE: f64 = 256.;

/// 연결선 텍스처 타일. 화면 이동만 있으면 그려 둔 타일을 옮겨 Cairo 래스터화와 업로드를 건너뛴다.
/// 배율·위치·선택·테마가 바뀌면 키가 달라져 전부 버린다.
#[derive(Default)]
pub(in crate::app::canvas) struct EdgeTiles {
    key: Option<u64>,
    tiles: HashMap<(i64, i64), Option<gdk::Texture>>,
}

impl EdgeTiles {
    /// 같은 키가 이전 프레임에서 이어질 때만 타일을 쓴다.
    /// 확대·카드 끌기처럼 매 프레임 바뀌는 동안에는 곧 버릴 타일을 만들지 않는다.
    pub(super) fn stable(&mut self, key: u64) -> bool {
        if self.key == Some(key) {
            return true;
        }
        self.key = Some(key);
        self.tiles.clear();
        false
    }

    #[cfg(test)]
    pub(in crate::app::canvas) fn is_empty(&self) -> bool {
        self.tiles.values().all(Option::is_none)
    }

    /// `offset`은 확대된 월드 좌표에서 화면 왼쪽 위의 위치다.
    /// `paint`는 타일 원점으로 옮긴 문맥에 그리고, 그린 것이 없으면 false를 돌려준다.
    pub(super) fn draw(
        &mut self,
        snapshot: &gtk::Snapshot,
        offset: (f64, f64),
        viewport: (f64, f64),
        scale: i32,
        paint: impl Fn(&cairo::Context, (i64, i64)) -> bool,
    ) {
        let first = |at: f64| (at / TILE).floor() as i64;
        let (x0, y0) = (first(offset.0), first(offset.1));
        let (x1, y1) = (first(offset.0 + viewport.0), first(offset.1 + viewport.1));
        // 다음 이동에 드러날 한 칸만 남겨 오래 돌아다녀도 메모리가 늘지 않는다.
        self.tiles.retain(|&(ix, iy), _| {
            (x0 - 1..=x1 + 1).contains(&ix) && (y0 - 1..=y1 + 1).contains(&iy)
        });
        for iy in y0..=y1 {
            for ix in x0..=x1 {
                let tile = self
                    .tiles
                    .entry((ix, iy))
                    .or_insert_with(|| raster(ix, iy, scale, &paint));
                if let Some(texture) = tile {
                    snapshot.append_texture(
                        texture,
                        &graphene::Rect::new(
                            (ix as f64 * TILE - offset.0) as f32,
                            (iy as f64 * TILE - offset.1) as f32,
                            TILE as f32,
                            TILE as f32,
                        ),
                    );
                }
            }
        }
    }
}

fn raster(
    ix: i64,
    iy: i64,
    scale: i32,
    paint: &impl Fn(&cairo::Context, (i64, i64)) -> bool,
) -> Option<gdk::Texture> {
    let size = TILE as i32 * scale;
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, size, size).ok()?;
    {
        let cr = cairo::Context::new(&surface).ok()?;
        cr.scale(f64::from(scale), f64::from(scale));
        cr.translate(-(ix as f64) * TILE, -(iy as f64) * TILE);
        if !paint(&cr, (ix, iy)) {
            return None;
        }
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let bytes = glib::Bytes::from(&surface.data().ok()?[..]);
    // Cairo ARGB32는 기기 바이트 순서의 미리 곱한 알파다.
    let format = if cfg!(target_endian = "little") {
        gdk::MemoryFormat::B8g8r8a8Premultiplied
    } else {
        gdk::MemoryFormat::A8r8g8b8Premultiplied
    };
    Some(gdk::MemoryTexture::new(size, size, format, &bytes, stride).upcast())
}
