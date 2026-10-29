# ADR-0001 · Rust와 네이티브 GTK

- 맥락: Ubuntu에서 Windows Sticky Notes처럼 목록과 독립 메모 창을 사용한다.
- 대안: 웹 UI wrapper, 네이티브 GTK.
- 결정: 사용자 요구에 따라 Rust·GTK4를 사용한다.
- 이유: 네이티브 입력·창 관리와 Rust 저장 로직을 한 앱에서 결합한다.
- 결과: GTK·SVG 로더가 필요하다. 화면 표현은 앱 CSS와 내장 자산으로 관리한다.
- 근거: https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_linux.html
