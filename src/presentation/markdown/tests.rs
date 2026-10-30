use super::*;

#[test]
fn canvas_001_summary_hides_markup_and_counts_tasks() {
    let summary =
        summary("\n# 출시 **계획**\n- [x] 범위\n- [ ] [노트](https://e.com)\n---\n`코드`");
    assert_eq!(summary.title, "출시 계획");
    assert_eq!(summary.preview, "범위 노트 코드");
    assert_eq!(task_progress("- [x] 범위\n- [ ] 노트\n일반"), (1, 2));
    assert_eq!(super::summary("").title, "");
    assert_eq!(super::summary("<b>raw</b>").title, "raw");
}

fn slice(text: &str, range: Range<i32>) -> String {
    text.chars()
        .skip(range.start as usize)
        .take((range.end - range.start) as usize)
        .collect()
}

#[test]
fn fmt_001_source_ranges_and_unicode() {
    let text = "# 제목 📝\n## 소제목\n**굵게** *기울임* ~~취소~~\n- 항목\n3. 순서\n- [ ] 할 일\n- [x] 완료\n[한글😀](https://example.com?a=1&b=2)";
    let doc = parse(text);
    for style in [
        Style::Heading1,
        Style::Heading2,
        Style::Bold,
        Style::Italic,
        Style::Strike,
        Style::Link,
    ] {
        assert!(
            doc.spans.iter().any(|s| s.style == style),
            "missing {style:?}"
        );
    }
    assert_eq!(slice(text, doc.links[0].range.clone()), "한글😀");
    assert_eq!(slice(text, doc.tasks[0].range.clone()), "[ ]");
    assert_eq!(slice(text, doc.tasks[1].range.clone()), "[x]");
    assert!(!doc.tasks[0].checked);
    assert!(doc.tasks[1].checked);
    for m in doc.markers {
        assert!(!slice(text, m.range).is_empty());
    }
}

#[test]
fn fmt_001_incomplete_escaped_and_unsupported_stay_literal() {
    for text in [
        "**unfinished",
        "[unfinished](",
        "\\*escaped*",
        "`**code**`",
        "```\n**code**\n```",
        "<b>raw</b>",
        "### outside scope",
        "plain & < >",
        "![**image alt**](https://example.com/image.png)",
    ] {
        let doc = parse(text);
        assert!(doc.markers.is_empty(), "unexpected hiding: {text}");
    }
    let doc = parse("**한글 *nested😀* bold**");
    assert_eq!(doc.markers.len(), 4);
    let doc = parse("[a `](` b](https://example.com/a_(b))");
    assert_eq!(doc.links.len(), 1);
    assert_eq!(doc.links[0].range, 1..9);
}

#[test]
fn fmt_003_list_items_keep_marker_and_content_ranges() {
    let text = "- 항목\n  - 안쪽\n\n1. 번호\n12) 둘\n\n- [x] 끝남\n- [ ] 할 일\n- \n> - 인용";
    let doc = parse(text);
    let view = |item: &ListItem| {
        (
            slice(text, item.marker.clone()),
            slice(text, item.content.clone()),
            item.depth,
            item.kind,
        )
    };
    let items: Vec<_> = doc.items.iter().map(view).collect();
    assert_eq!(
        items,
        [
            ("- ".to_owned(), "항목".to_owned(), 0, ListKind::Bullet),
            ("  - ".into(), "안쪽".into(), 1, ListKind::Bullet),
            ("1. ".into(), "번호".into(), 0, ListKind::Ordered),
            ("12) ".into(), "둘".into(), 0, ListKind::Ordered),
            ("- [x] ".into(), "끝남".into(), 0, ListKind::Task(true)),
            ("- [ ] ".into(), "할 일".into(), 0, ListKind::Task(false)),
            ("- ".into(), "".into(), 0, ListKind::Bullet),
        ]
    );
    assert!(doc.markers.is_empty(), "list markers are drawn, not hidden");
    let done: Vec<_> = doc
        .spans
        .iter()
        .filter(|s| s.style == Style::TaskDone)
        .map(|s| slice(text, s.range.clone()))
        .collect();
    assert_eq!(done, ["끝남"]);
    assert!(parse("```\n- 코드\n```").items.is_empty());
    assert!(
        parse("- - -\n* * *").items.is_empty(),
        "rules are not lists"
    );
    // 쓰는 동안: 문단 바로 뒤, 빈 항목, 빈 체크도 그 자리에서 항목이다. 공백 전에는 아니다.
    let kinds = |text: &str| parse(text).items.iter().map(|i| i.kind).collect::<Vec<_>>();
    assert_eq!(kinds("문단\n- "), [ListKind::Bullet]);
    assert_eq!(kinds("문단\n- [ ] "), [ListKind::Task(false)]);
    assert_eq!(kinds("문단\n2. "), [ListKind::Ordered]);
    assert!(kinds("문단\n-").is_empty());
    assert!(kinds("-5도").is_empty());
    assert_eq!(kinds("- [ ]"), [ListKind::Bullet]);
    assert_eq!(parse("문단\n- [ ] ").tasks.len(), 1);
    // 네 칸 들여쓰기: 기호가 본문으로 새지 않는다.
    let text = "- 바깥\n    - 안쪽\n1. 하나\n    1. 둘";
    let items: Vec<_> = parse(text)
        .items
        .iter()
        .map(|i| (slice(text, i.content.clone()), i.kind))
        .collect();
    assert_eq!(
        items,
        [
            ("바깥".to_owned(), ListKind::Bullet),
            ("안쪽".into(), ListKind::Bullet),
            ("하나".into(), ListKind::Ordered),
            ("둘".into(), ListKind::Ordered),
        ]
    );
    // 탭 뒤 체크: 토글 범위는 괄호부터다. 안 그러면 Ctrl+Enter가 원문을 망가뜨린다.
    let text = "  - \t[ ] 탭";
    let doc = parse(text);
    assert_eq!(slice(text, doc.tasks[0].range.clone()), "[ ]");
    assert_eq!(slice(text, doc.items[0].content.clone()), "탭");
    // 중첩 체크는 각자 항목에 붙는다.
    let text = "- [ ] 위\n\t- [x] 아래";
    let doc = parse(text);
    assert_eq!(doc.items[0].kind, ListKind::Task(false));
    assert_eq!(doc.items[1].kind, ListKind::Task(true));
    assert_eq!(slice(text, doc.items[0].content.clone()), "위");
    // 번호 + 체크는 번호를 숨기지 않는다.
    let text = "1. [ ] 일";
    let doc = parse(text);
    assert_eq!(doc.items[0].kind, ListKind::Ordered);
    assert_eq!(slice(text, doc.items[0].marker.clone()), "1. ");
    assert_eq!(doc.tasks.len(), 1);
    // 인용문 안의 체크가 앞 항목을 바꾸면 표식 범위가 여러 줄로 번진다.
    let text = "- 앞 항목\n\n> - [ ] 인용 할 일";
    let doc = parse(text);
    assert_eq!(doc.items.len(), 1);
    assert_eq!(doc.items[0].kind, ListKind::Bullet);
    assert_eq!(slice(text, doc.items[0].marker.clone()), "- ");
    assert_eq!(doc.tasks.len(), 1);
}

#[test]
fn canvas_004_summary_keeps_list_markers_for_cards() {
    let s = summary(
        "- [ ] 머리 할 일\n- [x] 끝남\n  - 안쪽\n3. 셋째\n문단 하나\n문단 둘\n```\n- 코드\n```\n---",
    );
    assert_eq!(s.title, "머리 할 일");
    assert_eq!(s.title_marker, Some(LineMarker::Task(false)));
    let got: Vec<_> = s
        .lines
        .iter()
        .map(|l| (l.marker.clone(), l.depth, l.text.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            (Some(LineMarker::Task(true)), 0, "끝남"),
            (Some(LineMarker::Bullet), 1, "안쪽"),
            (Some(LineMarker::Number("3.".into())), 0, "셋째"),
            (None, 0, "문단 하나"),
            (None, 0, "문단 둘"),
            (None, 0, "- 코드"),
        ]
    );
    assert_eq!(s.remaining, 0);
    assert_eq!(
        summary("```한 줄```\n- 항목").lines[0].marker,
        Some(LineMarker::Bullet)
    );
    let many = summary(&format!("제목\n{}", "- 항목\n".repeat(50)));
    assert_eq!(many.lines.len(), 12);
    assert_eq!(many.remaining, 38);
}

#[test]
fn canvas_004_fit_preview_spends_a_line_budget() {
    let line = |marker: Option<LineMarker>, text: &str| PreviewLine {
        marker,
        depth: 0,
        text: text.into(),
    };
    // 문단만 있으면 지금처럼 한 행으로 이어 세 줄까지 쓴다.
    let (rows, more) = fit_preview(&[line(None, "가"), line(None, "나")], 3);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].line.text.as_str(), rows[0].max_lines),
        ("가 나", 3)
    );
    assert_eq!(more, 0);
    // 항목은 한 줄씩, 앞선 문단은 뒤 항목 몫을 남긴다.
    let items = [
        line(None, "설명"),
        line(Some(LineMarker::Bullet), "하나"),
        line(Some(LineMarker::Bullet), "둘"),
        line(Some(LineMarker::Bullet), "셋"),
    ];
    let (rows, more) = fit_preview(&items, 3);
    assert_eq!(
        rows.iter().map(|r| r.max_lines).collect::<Vec<_>>(),
        [1, 1, 1]
    );
    assert_eq!(more, 1);
    assert_eq!(fit_preview(&items[..2], 3).0[0].max_lines, 2);
    assert_eq!(fit_preview(&[], 3), (Vec::new(), 0));
}

#[test]
fn fmt_003_bare_and_angle_urls_become_links() {
    let urls =
        |text: &str| -> Vec<String> { parse(text).links.into_iter().map(|l| l.url).collect() };
    assert_eq!(
        urls("보기 https://a.example/x_y_z."),
        ["https://a.example/x_y_z"]
    );
    assert_eq!(urls("https://a.example입니다"), ["https://a.example"]);
    assert_eq!(
        urls("(https://a.example/b_(c))"),
        ["https://a.example/b_(c)"]
    );
    assert_eq!(urls("**https://a.example**"), ["https://a.example"]);
    let doc = parse("https://a.example/_b_/c");
    assert_eq!(doc.links[0].url, "https://a.example/_b_/c");
    assert!(doc.markers.is_empty(), "URL characters are not syntax");
    assert_eq!(urls("<https://a.example/b>"), ["https://a.example/b"]);
    assert_eq!(
        urls("[이름](https://a.example) https://b.example"),
        ["https://a.example", "https://b.example"]
    );
    for text in [
        "`https://a.example`",
        "```\nhttps://a.example\n```",
        "xhttps://a.example",
        "https://",
        "http 없음",
        "![그림](https://a.example/i.png)",
        "<a href=\"https://a.example\">x</a>",
    ] {
        assert!(urls(text).is_empty(), "{text}");
    }
    let text = "한글 https://a.example 끝";
    let link = &parse(text).links[0];
    assert_eq!(slice(text, link.range.clone()), "https://a.example");
    let text = "[이름](https://a.example \"제목\")";
    let link = &parse(text).links[0];
    assert_eq!(slice(text, link.url_range.clone()), "https://a.example");
    assert_eq!(slice(text, link.range.clone()), "이름");
    assert_eq!(slice(text, link.owner.clone()), text);
}

#[test]
fn fmt_003_link_info_shows_the_real_destination() {
    let info = link_info("https://www.Example.com/a/b/?q=1", "이름");
    assert_eq!(info.host, "www.example.com");
    assert_eq!(info.rest, "/a/b/?q=1");
    assert!(info.warning.is_none());
    assert_eq!(link_info("https://a.example/", "x").rest, "");
    let mail = link_info("mailto:a@example.test?subject=x", "메일");
    assert!(mail.mail);
    assert_eq!(mail.host, "a@example.test");
    assert!(link_info("http://a.example", "x").warning.is_some());
    // 호스트 위장: 사용자 정보, 역슬래시, 이름과 다른 목적지, 방향 제어 문자.
    let info = link_info("https://google.com@evil.example/x", "x");
    assert_eq!(info.host, "evil.example");
    assert!(info.warning.is_some());
    assert_eq!(
        link_info("https://evil.example\\@good.example", "x").host,
        "evil.example"
    );
    assert!(link_info("https://evil.example", "bank.com")
        .warning
        .is_some());
    assert!(link_info("https://evil.example", "https://bank.com/login")
        .warning
        .is_some());
    assert!(link_info("https://docs.bank.com", "bank.com")
        .warning
        .is_none());
    assert!(link_info("https://www.bank.com", "bank.com")
        .warning
        .is_none());
    assert!(link_info("https://bank.com:8443", "bank.com")
        .warning
        .is_none());
    // 파일·제품 이름은 도메인으로 보지 않는다. 경보가 잦으면 아무도 읽지 않는다.
    for label in [
        "README.md",
        "Node.js",
        "report.pdf",
        "v1.2 문서",
        "Socket.IO 문서",
    ] {
        assert!(
            link_info("https://a.example/x", label).warning.is_none(),
            "{label}"
        );
    }
    // 명시적 주소 표기·포트·경로 안의 위장은 놓치지 않는다.
    assert!(link_info("https://evil.example", "bank.com:443")
        .warning
        .is_some());
    assert!(link_info("https://evil.example/bank.com", "bank.com")
        .warning
        .is_some());
    assert!(link_info("https://evil.example", "www.bank.test")
        .warning
        .is_some());
    // 비ASCII·빈 호스트.
    assert!(link_info("https://\u{0430}pple.com", "x").warning.is_some());
    assert!(link_info("https:///evil.example", "x").warning.is_some());
    assert!(link_info("https://", "x").warning.is_some());
    // 서식 문법이 든 이름도 평문으로 판별한다.
    assert_eq!(plain_line("**bank.com**"), "bank.com");
    assert!(link_info("https://a.example", "v1.2 문서")
        .warning
        .is_none());
    assert!(link_info("https://xn--80ak6aa92e.com", "x")
        .warning
        .is_some());
    assert!(link_info("https://a.example/\u{202E}gpj.exe", "x")
        .rest
        .contains('\u{FFFD}'));
}

#[test]
fn fmt_001_link_protocols() {
    for url in [
        "https://example.com",
        "HTTP://example.com",
        "hTtPs://한글.example/😀",
        "MAILTO:a@example.test",
        "mailto:a@example.test",
    ] {
        assert!(safe_link(url));
    }
    for url in [
        "javascript:alert(1)",
        "file:///tmp/test",
        "data:text/html,test",
        "https://example.com\n",
        "hTtPs://example.com\0",
        "https😀",
        "mailto",
    ] {
        assert!(!safe_link(url));
    }
    assert!(parse("[bad](javascript:alert) [file](file:///tmp/test)")
        .links
        .is_empty());
}

#[test]
fn fmt_003_item_at_matches_source_boundaries() {
    let text = "앞😀\n- 한글\n\n  - 안쪽\n- ";
    let doc = parse(text);
    for offset in -1..=text.chars().count() as i32 + 1 {
        let expected = doc
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| item.marker.start <= offset && offset <= item.content.end);
        assert_eq!(
            doc.item_at(offset).map(|(index, _)| index),
            expected.map(|(index, _)| index),
            "offset {offset}"
        );
    }
    assert!(parse("").item_at(0).is_none());
}
