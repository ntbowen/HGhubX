//! Upstream lines and mirrors published by the upstream address pages.
//! Publisher scripts are parsed as data; remote JavaScript is never executed.

mod jsparse;
mod mirror;
mod probe;

pub use mirror::{fixed, is_fixed, is_trusted_mirror, mirrors, verify_published_origin, FIXED};
pub(crate) use probe::healthy_video_data;
pub use probe::{auto_pick, health, probe, try_main, Hit};

#[cfg(test)]
mod tests {
    use super::jsparse::js_array;
    use super::mirror::{
        absolute, merge_publishers, parse_mirrors, script_src, PublishedPage, FIXED, MIRROR_MAX,
        PUBLISH_PAGES,
    };
    use super::probe::{auto_candidates, healthy_video_data, pick_from_hits, Hit};
    use serde_json::{json, Value};


    fn origins(items: &[Value]) -> Vec<&str> {
        items
            .iter()
            .filter_map(|item| item["url"].as_str())
            .collect()
    }

    #[test]
    fn literal_arrays_support_comments_quotes_and_trailing_commas() {
        let js = r#"/* before */ var urls = ['EXAMPLE.COM/', "two.example",];
          const subdomains = [ 'one', // line comment
          'two', ]; var untouched = 'https://example.com/';"#;
        let items = parse_mirrors(js).unwrap();
        assert_eq!(
            origins(&items),
            [
                "https://one.example.com",
                "https://two.example.com",
                "https://one.two.example",
                "https://two.two.example"
            ]
        );
        assert_eq!(
            js_array("let urls = ['https://host/'];", "urls")
                .unwrap()
                .unwrap(),
            ["https://host/"]
        );
    }

    #[test]
    fn fake_declarations_in_comments_strings_templates_and_regex_are_ignored() {
        for js in [
            r#"// var urls=['bad.example'];"#,
            r#"/* var urls=['bad.example']; */"#,
            r#"const text = "var urls=['bad.example'];";"#,
            r#"const text = `var urls=['bad.example'];`;"#,
            r#"const regex = /var urls=['bad.example'];/;"#,
            r#"var urlsExtra=['bad.example'];"#,
            r#"function notRun() { var urls=['bad.example']; }"#,
        ] {
            assert!(parse_mirrors(js).is_err(), "{js}");
        }
    }

    #[test]
    fn dynamic_or_malformed_arrays_fail_closed_including_subdomains() {
        for js in [
            r#"var urls=['one.example', getHost()];"#,
            r#"var urls=['one.example' + '.evil'];"#,
            r#"var urls=['one.example'].concat(other);"#,
            r#"var urls=['one.example']; var urls=['two.example'];"#,
            r#"var urls=['one.example']; var subdomains=getSubs();"#,
            r#"var urls=['one.example']; var subdomains=['one', 2];"#,
            r#"var urls=['one.example']; /* unfinished"#,
        ] {
            assert!(parse_mirrors(js).is_err(), "{js}");
        }
    }

    #[test]
    fn hostnames_are_canonical_bounded_and_not_paths_or_literal_ips() {
        let items = parse_mirrors(r#"var urls=['Example.COM', 'example.com/', '127.0.0.1', 'localhost', 'x.local', 'x.internal', 'x.home.arpa', 'https://evil.example', 'evil.example/path', 'a.example:443', '-a.example', 'valid.example'];"#).unwrap();
        assert_eq!(
            origins(&items),
            ["https://example.com", "https://valid.example"]
        );
        let js = format!(
            "var urls=[{}];",
            (0..100)
                .map(|n| format!("'host{n}.example'"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(parse_mirrors(&js).unwrap().len(), MIRROR_MAX);
    }

    #[test]
    fn publish_script_is_same_origin_and_exact_filename() {
        let page = PUBLISH_PAGES[0];
        for html in [
            "<script src='publish.js?v=2'></script>",
            "<script src=publish.js></script>",
            r#"<script src='/analytics.js'></script><script src="/publish.js"></script>"#,
        ] {
            let script = script_src(page, html).unwrap();
            assert!(script.starts_with("https://huangguoai.pages.dev/publish.js"));
        }
        for target in [
            "http://huangguoai.pages.dev/publish.js",
            "https://other.example/publish.js",
            "//other.example/publish.js",
            "javascript:publish.js",
            "https://user@huangguoai.pages.dev/publish.js",
            "/publish.js#fragment",
            "/publish.js/extra",
            "/publish.js.evil",
        ] {
            assert!(absolute(page, target).is_err(), "{target}");
        }
        assert!(script_src(page, "<script src='analytics.js'></script>").is_err());
    }

    fn page(source: &str, hosts: &[&str]) -> Result<PublishedPage, String> {
        Ok(PublishedPage {
            source: source.into(),
            script: format!("{source}publish.js"),
            items: hosts
                .iter()
                .map(|host| json!({"host": host, "url": format!("https://{host}")}))
                .collect(),
        })
    }

    #[test]
    fn publisher_failure_retains_other_success_and_current_is_display_only() {
        let value = merge_publishers(
            [
                Err("timeout".into()),
                page(PUBLISH_PAGES[1], &["a.example"]),
            ],
            "https://restored.example",
        )
        .unwrap();
        assert_eq!(
            origins(value["items"].as_array().unwrap()),
            ["https://a.example"]
        );
        assert_eq!(value["recommended"][0]["url"], "https://restored.example");
        assert_eq!(value["errors"].as_array().unwrap().len(), 1);
        assert!(merge_publishers([Err("first".into()), Err("second".into())], FIXED[0].1).is_err());
    }

    #[test]
    fn publisher_lists_interleave_and_deduplicate() {
        let value = merge_publishers(
            [
                page(PUBLISH_PAGES[0], &["a.example", "c.example"]),
                page(PUBLISH_PAGES[1], &["b.example", "a.example"]),
            ],
            "https://a.example",
        )
        .unwrap();
        assert_eq!(
            origins(value["items"].as_array().unwrap()),
            [
                "https://a.example",
                "https://b.example",
                "https://c.example"
            ]
        );
        assert_eq!(value["recommended"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn health_requires_real_unwrapped_feed_data() {
        let valid = json!({"items": [{"id": 12, "title": "video"}], "pagination": {"page": 1, "pages": 2, "size": 1, "total": 2}});
        assert!(healthy_video_data(&valid));
        assert!(healthy_video_data(
            &json!({"items": [], "pagination": {"page": 1, "pages": 0, "size": 1, "total": 0}})
        ));
        for invalid in [
            Value::Null,
            json!("<html>200 OK</html>"),
            json!({}),
            json!({"status": 0, "msg": "denied"}),
            json!({"status": 1, "data": valid}),
            json!({"items": []}),
            json!({"items": [{}], "pagination": {"page": 1, "pages": 1, "size": 1, "total": 1}}),
        ] {
            assert!(!healthy_video_data(&invalid), "{invalid}");
        }
    }

    fn hit(url: &str, ok: bool, ms: u64) -> Hit {
        Hit {
            tier: "mirror",
            url: url.into(),
            ok,
            reachable: true,
            ms,
            status: Some(200),
            error: None,
        }
    }

    #[test]
    fn all_down_never_returns_a_line_to_switch_to() {
        for hits in [vec![], vec![hit("https://bad.example", false, 1)]] {
            let selected = pick_from_hits(&hits);
            assert_eq!(selected["reason"], "all-down");
            assert_eq!(selected["base"], "");
            assert_eq!(selected["tier"], "none");
        }
        let selected = pick_from_hits(&[
            hit("https://html.example", false, 1),
            hit("https://slow.example", true, 20),
            hit("https://fast.example", true, 8),
        ]);
        assert_eq!(selected["base"], "https://fast.example");
    }

    #[test]
    fn auto_candidates_require_publication_and_exclude_ai_landing_page() {
        let items = [
            // The .ai navigation page must stay out of candidates even if a
            // publish page lists it; it is no longer a fixed line.
            json!({"url": "https://huangguoai.ai"}),
            json!({"url": "https://published.example"}),
            json!({"url": "https://untrusted.example"}),
            json!({"url": "https://published.example"}),
        ];
        let candidates = auto_candidates(&items, "https://restored.example", |origin| {
            origin == "https://published.example"
        });
        assert_eq!(candidates, ["https://published.example"]);
        assert!(auto_candidates(&[], "https://restored.example", |_| true).is_empty());
    }
}
