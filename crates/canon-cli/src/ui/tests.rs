// SPDX-License-Identifier: AGPL-3.0-or-later
//! The page's door, checked without a socket: every request goes through
//! `respond`, so every refusal is a function call away.

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";
const PORT: u16 = 4321;

fn ctx(name: &str, log: &canon_core::Log) -> Ctx {
    let root = std::env::temp_dir().join(format!("canon-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(".canon");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("acts.jsonl"), log.render()).unwrap();
    Ctx {
        dir,
        actor: "human:alex".into(),
        token: TOKEN.into(),
        port: PORT,
    }
}

fn get(path: &str, cookie: bool) -> Request {
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let mut headers = vec![("Host".to_string(), format!("127.0.0.1:{PORT}"))];
    if cookie {
        headers.push(("Cookie".into(), format!("other=1; {COOKIE}={TOKEN}")));
    }
    Request {
        method: "GET".into(),
        path: path.into(),
        query: form(query.as_bytes()),
        headers,
        body: Vec::new(),
    }
}

fn post(body: &str, origin: Option<&str>) -> Request {
    let mut r = get("/decide", true);
    r.method = "POST".into();
    if let Some(o) = origin {
        r.headers.push(("Origin".into(), o.into()));
    }
    r.body = body.as_bytes().to_vec();
    r
}

fn acts(c: &Ctx) -> String {
    std::fs::read_to_string(c.dir.join("acts.jsonl")).unwrap()
}

#[test]
fn the_page_answers_only_at_its_own_address() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("host", &log);
    let mut r = get("/", true);
    r.headers[0].1 = format!("rebound.example:{PORT}");
    assert_eq!(
        respond(&r, &c).status,
        421,
        "DNS rebinding arrives under another name"
    );
    r.headers[0].1 = format!("localhost:{PORT}");
    assert_eq!(respond(&r, &c).status, 200);
}

#[test]
fn the_printed_address_is_the_only_way_in() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("token", &log);
    assert_eq!(respond(&get("/", false), &c).status, 403);
    assert_eq!(respond(&get("/?t=wrong", false), &c).status, 403);
    let r = respond(&get(&format!("/?t={TOKEN}"), false), &c);
    assert_eq!(r.status, 303);
    let cookie = r.headers.iter().find(|(k, _)| k == "Set-Cookie").unwrap();
    assert!(cookie.1.contains("HttpOnly") && cookie.1.contains("SameSite=Strict"));
    assert!(
        r.headers.iter().any(|(k, v)| k == "Location" && v == "/"),
        "the token leaves the address bar"
    );
    assert_eq!(respond(&get("/", true), &c).status, 200);
}

#[test]
fn a_decision_from_anywhere_but_the_page_is_refused_and_writes_nothing() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("csrf", &log);
    let before = acts(&c);
    let body = format!("token={TOKEN}&id={}&decision=approve", quiet.as_str());
    assert_eq!(
        respond(&post(&body, Some("https://evil.example")), &c).status,
        403
    );
    let without = format!("id={}&decision=approve", quiet.as_str());
    assert_eq!(respond(&post(&without, None), &c).status, 403);
    let mut sneaky = post(&body, None);
    sneaky
        .headers
        .push(("Sec-Fetch-Site".into(), "cross-site".into()));
    assert_eq!(respond(&sneaky, &c).status, 403);
    assert_eq!(acts(&c), before);
}

#[test]
fn only_what_waits_on_you_can_be_decided_here() {
    let (log, _, pans) = crate::review::tests::house();
    let c = ctx("scope", &log);
    let before = acts(&c);
    // The kitchen is sam's, not alex's.
    let body = format!("token={TOKEN}&id={}&decision=approve", pans.as_str());
    let r = respond(&post(&body, Some(&format!("http://127.0.0.1:{PORT}"))), &c);
    assert!(r.body.contains("not waiting on you"), "{}", r.body);
    assert_eq!(acts(&c), before);
}

#[test]
fn approving_writes_a_position_and_says_where_it_stands() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("approve", &log);
    let body = format!("token={TOKEN}&id={}&decision=approve", quiet.as_str());
    let r = respond(&post(&body, Some(&format!("http://127.0.0.1:{PORT}"))), &c);
    assert_eq!(r.status, 303);
    let last = acts(&c).lines().last().unwrap().to_string();
    assert!(
        last.contains(r#""op":"position""#) && last.contains(r#""actor":"human:alex""#),
        "{last}"
    );
    let page = respond(&get(&format!("/?done={}", quiet.as_str()), true), &c).body;
    assert!(page.contains("is in force"), "{page}");
}

#[test]
fn an_objection_needs_its_reason() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("object", &log);
    let before = acts(&c);
    let origin = format!("http://127.0.0.1:{PORT}");
    let bare = format!(
        "token={TOKEN}&id={}&decision=object&because=",
        quiet.as_str()
    );
    let r = respond(&post(&bare, Some(&origin)), &c);
    assert!(r.body.contains("An objection needs a reason"), "{}", r.body);
    assert_eq!(acts(&c), before);
    let reasoned = format!(
        "token={TOKEN}&id={}&decision=object&because=Ten+is+too+early+on+Fridays",
        quiet.as_str()
    );
    assert_eq!(respond(&post(&reasoned, Some(&origin)), &c).status, 303);
    assert!(acts(&c).contains("Ten is too early on Fridays"));
}

#[test]
fn what_people_wrote_is_shown_and_never_run() {
    let evil = canon_core::Act::new(
        canon_core::ActKind::Assert {
            text: "<script>alert(1)</script> & \"quotes\"".into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let c = ctx("escape", &canon_core::Log::from_acts(vec![evil]));
    let page = respond(&get("/", true), &c).body;
    assert!(!page.contains("<script>"));
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;quotes&quot;"));
}

#[test]
fn an_amendment_strikes_what_goes_and_writes_in_what_comes() {
    assert_eq!(
        amendment(
            "Guests may stay two nights.",
            "Guests may stay three nights."
        ),
        "Guests may stay <del>two</del> <ins>three</ins> nights."
    );
    assert_eq!(
        amendment("a <b>", "a <c>"),
        "a <del>&lt;b&gt;</del> <ins>&lt;c&gt;</ins>"
    );
}

#[test]
fn ids_in_a_history_become_links_and_nothing_else_does() {
    assert_eq!(
        linked("replaced can-0123456789ab, see <can-zz>"),
        "replaced <a href=\"/why/can-0123456789ab\">can-0123456789ab</a>, see &lt;can-zz&gt;"
    );
}

#[test]
fn a_request_larger_than_a_form_is_refused() {
    let big = format!(
        "POST /decide HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nContent-Length: 999999\r\n\r\n"
    );
    assert!(read_request(&mut big.as_bytes()).is_err());
    let ok = format!("GET /why/can-1?x=a+b%21 HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n\r\n");
    let r = read_request(&mut ok.as_bytes()).unwrap();
    assert_eq!((r.method.as_str(), r.path.as_str()), ("GET", "/why/can-1"));
    assert_eq!(r.param("x"), Some("a b!"));
}

fn origin() -> String {
    format!("http://127.0.0.1:{PORT}")
}

fn at(path: &str, body: &str) -> Request {
    let mut r = post(body, Some(&origin()));
    r.path = path.into();
    r
}

#[test]
fn every_write_goes_through_the_same_door() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("door", &log);
    let before = acts(&c);
    for path in [
        "/propose",
        "/replace",
        "/retract",
        "/silence",
        "/lift",
        "/identify",
    ] {
        let mut r = at(path, "text=x&about=x&because=x");
        assert_eq!(respond(&r, &c).status, 403, "{path} without the token");
        r.body = format!("token={TOKEN}&text=x&about=x&because=x").into_bytes();
        r.headers
            .push(("Origin".into(), "https://evil.example".into()));
        r.headers
            .retain(|(k, v)| k != "Origin" || v == "https://evil.example");
        assert_eq!(respond(&r, &c).status, 403, "{path} from another site");
    }
    assert_eq!(acts(&c), before);
}

#[test]
fn a_rule_can_be_proposed_changed_and_taken_away_from_the_page() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("rules", &log);
    let r = respond(
        &at(
            "/propose",
            &format!("token={TOKEN}&text=Bins+go+out+on+Tuesday.&scope=house"),
        ),
        &c,
    );
    assert_eq!(r.status, 303);
    let (_, st) = load(&c).unwrap();
    let bins = st
        .commitments
        .iter()
        .find(|x| x.text == "Bins go out on Tuesday.")
        .unwrap();
    assert_eq!(st.scope_of(&bins.id).map(|s| s.as_str()), Some("house"));
    assert!(
        matches!(bins.status, canon_core::Status::Active),
        "alex holds house"
    );

    // A change needs its reason, and the reason is kept.
    let id = bins.id.as_str().to_string();
    let bare = respond(
        &at(
            "/replace",
            &format!("token={TOKEN}&id={id}&text=Bins+go+out+on+Monday."),
        ),
        &c,
    );
    assert!(bare.body.contains("Nothing was written"), "{}", bare.body);
    let r = respond(
        &at(
            "/replace",
            &format!("token={TOKEN}&id={id}&text=Bins+go+out+on+Monday.&because=Collection+moved"),
        ),
        &c,
    );
    assert_eq!(r.status, 303);
    let (_, st) = load(&c).unwrap();
    let monday = st
        .commitments
        .iter()
        .find(|x| x.text == "Bins go out on Monday.")
        .unwrap();
    assert_eq!(
        st.scope_of(&monday.id).map(|s| s.as_str()),
        Some("house"),
        "the room carries over"
    );
    assert!(acts(&c).contains("Collection moved"));

    // The old wording is not in force, so it cannot be changed again.
    let stale = respond(
        &at("/retract", &format!("token={TOKEN}&id={id}&because=x")),
        &c,
    );
    assert!(
        stale.body.contains("not in force any more"),
        "{}",
        stale.body
    );
    let gone = respond(
        &at(
            "/retract",
            &format!(
                "token={TOKEN}&id={}&because=No+collection",
                monday.id.as_str()
            ),
        ),
        &c,
    );
    assert_eq!(gone.status, 303);
    let (_, st) = load(&c).unwrap();
    assert!(matches!(
        st.get(&monday.id).unwrap().status,
        canon_core::Status::Retracted { .. }
    ));
}

#[test]
fn something_left_unwritten_needs_its_reason_and_can_be_lifted() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("silence", &log);
    let before = acts(&c);
    let bare = respond(&at("/silence", &format!("token={TOKEN}&about=guests")), &c);
    assert!(bare.body.contains("Nothing was written"));
    assert_eq!(acts(&c), before);
    assert_eq!(
        respond(
            &at(
                "/silence",
                &format!("token={TOKEN}&about=guests&because=We+trust+each+other")
            ),
            &c
        )
        .status,
        303
    );
    let (_, st) = load(&c).unwrap();
    let s = st
        .silences
        .iter()
        .find(|s| s.about == "guests")
        .unwrap()
        .act
        .clone();
    // Only a silence that stands can be lifted: the form is not `undo`.
    let grant = log.acts()[0].id.as_str().to_string();
    let not = respond(&at("/lift", &format!("token={TOKEN}&id={grant}")), &c);
    assert!(not.body.contains("not left unwritten"), "{}", not.body);
    assert_eq!(
        respond(
            &at("/lift", &format!("token={TOKEN}&id={}", s.as_str())),
            &c
        )
        .status,
        303
    );
    assert!(load(&c)
        .unwrap()
        .1
        .silences
        .iter()
        .all(|x| x.about != "guests"));
}

#[test]
fn identify_takes_only_a_key_the_page_offered() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("identify", &log);
    let before = acts(&c);
    let r = respond(
        &at(
            "/identify",
            &format!("token={TOKEN}&key=SHA256:not-one-of-yours"),
        ),
        &c,
    );
    assert!(
        r.body.contains("Choose one of the keys listed"),
        "{}",
        r.body
    );
    assert_eq!(acts(&c), before);
}

#[test]
fn a_form_from_the_page_keeps_its_origin_and_a_null_one_is_refused() {
    // Under `Referrer-Policy: no-referrer` a browser sends `Origin: null`
    // on the page's own forms, and every button on the page was refused.
    let policy = SECURITY_HEADERS
        .iter()
        .find(|(k, _)| *k == "Referrer-Policy")
        .unwrap()
        .1;
    assert_eq!(policy, "same-origin");
    // A sandboxed frame or a data: page sends `null` too, so it stays shut.
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("null-origin", &log);
    let body = format!("token={TOKEN}&id={}&decision=approve", quiet.as_str());
    assert_eq!(respond(&post(&body, Some("null")), &c).status, 403);
}
