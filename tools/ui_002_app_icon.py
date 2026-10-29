"""검증: UI-002. 설치된 앱 메뉴의 실제 아이콘 조회·로드를 확인한다."""
import gi

gi.require_version("Gtk", "3.0")
from gi.repository import Gio, Gtk

app = Gio.DesktopAppInfo.new("io.github.lsm.LinuxStickyMemo.desktop")
assert app is not None, "앱 메뉴 항목을 찾을 수 없습니다"
settings = Gio.Settings.new("org.gnome.desktop.interface")
theme = Gtk.IconTheme.new()
theme.set_custom_theme(settings.get_string("icon-theme"))
icon = app.get_icon()
assert icon is not None, "앱 메뉴에 아이콘이 지정되지 않았습니다"
info = theme.lookup_by_gicon(icon, 128, Gtk.IconLookupFlags.FORCE_SVG)
assert info is not None, f"아이콘 조회 실패: {icon.to_string()}"
image = info.load_icon()
assert image.get_width() > 0 and image.get_height() > 0
print(f"UI-002 PASS: {info.get_filename()}")
