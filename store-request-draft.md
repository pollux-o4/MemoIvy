Category: store-requests
Title: Request: dbus slot (session bus) for memoivy

Hello,

I'm the publisher of the `memoivy` snap (MemoIvy, a GTK4 sticky note app):
https://snapcraft.io/memoivy
Source: https://github.com/pollux-o4/MemoIvy

**Request:** auto-connection / approval of a `dbus` slot on the session bus with the well-known name `io.github.pollux_o4.MemoIvy`.

```yaml
slots:
  memoivy-dbus:
    interface: dbus
    bus: session
    name: io.github.pollux_o4.MemoIvy
```

**Why:** MemoIvy is a `GtkApplication` with application ID `io.github.pollux_o4.MemoIvy`. On startup GLib registers this name on the session bus to provide single-instance behavior and desktop activation. Under strict confinement, AppArmor denies the name registration and the app fails to start. The name matches the app's reverse-DNS ID, `common-id`, and the desktop file (`io.github.pollux_o4.MemoIvy.desktop`), and it is owned by the publisher (GitHub user `pollux-o4`).

Revision 2 (version 0.1.1) with this slot is currently held for manual review. I installed that build locally with `--dangerous` and confirmed it now starts normally without the D-Bus denial.

Thank you!
