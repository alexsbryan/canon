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
        draft: None,
        can_create: Arc::new(AtomicBool::new(log.acts().is_empty())),
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
    r.headers.push((
        "Content-Type".into(),
        "application/x-www-form-urlencoded".into(),
    ));
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
    let before = acts(&c);
    let stale = respond(&post(&body, Some(&format!("http://127.0.0.1:{PORT}"))), &c);
    assert!(stale.body.contains("not waiting on you"));
    assert_eq!(acts(&c), before, "a stale decision is not repeated");
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
        "/setup",
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
fn the_decision_comes_before_its_history_and_the_identity_prompt() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("first", &log);
    let page = respond(&get("/", true), &c).body;
    let decide = page.find("action=\"/decide\"").expect("a decision waits");
    for later in ["History and supporting entries", "Identify yourself"] {
        assert!(decide < page.find(later).expect(later), "{later}: {page}");
    }
    assert!(!page.contains("No source passage"), "{page}");
}

#[test]
fn a_waiting_rule_can_be_decided_from_its_own_history() {
    let (log, quiet, pans) = crate::review::tests::house();
    let c = ctx("history-decide", &log);
    let page = respond(&get(&format!("/why/{}", quiet.as_str()), true), &c).body;
    assert!(page.contains("action=\"/decide\""), "{page}");
    let theirs = respond(&get(&format!("/why/{}", pans.as_str()), true), &c).body;
    assert!(
        !theirs.contains("action=\"/decide\""),
        "the kitchen is sam's: {theirs}"
    );
    let body = format!("token={TOKEN}&id={}&decision=approve", quiet.as_str());
    assert_eq!(respond(&at("/decide", &body), &c).status, 303);
    let settled = respond(&get(&format!("/why/{}", quiet.as_str()), true), &c).body;
    assert!(!settled.contains("action=\"/decide\""), "{settled}");
}

#[test]
fn taking_a_rule_away_says_nobody_is_asked_and_a_silence_says_what_it_is_not() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("say-so", &log);
    let body = format!("token={TOKEN}&id={}&decision=approve", quiet.as_str());
    assert_eq!(respond(&at("/decide", &body), &c).status, 303);
    let path = format!("/why/{}", quiet.as_str());
    let page = respond(&get(&path, true), &c).body;
    assert!(page.contains("goes as soon as you press"), "{page}");
    let gone = format!(
        "token={TOKEN}&id={}&because=No+longer+needed",
        quiet.as_str()
    );
    let r = respond(&at("/retract", &gone), &c);
    let back = r.headers.iter().find(|(k, _)| k == "Location").unwrap();
    let page = respond(&get(&back.1, true), &c).body;
    assert!(page.contains("Nobody else was asked."), "{page}");

    let home = respond(&get("/", true), &c).body;
    assert!(home.contains("does not approve, object to or cancel"));
    assert!(!home.contains("the group decided"), "{home}");
}

#[test]
fn a_reason_given_with_a_proposal_is_kept_and_read_as_a_note() {
    let (log, _, _) = crate::review::tests::house();
    let mut c = ctx("reason", &log);
    c.actor = "agent:drafter".into();
    let body = format!(
        "token={TOKEN}&text=Bins+go+out+on+Tuesday.&scope=house&because=The+collection+moved"
    );
    assert_eq!(respond(&at("/propose", &body), &c).status, 303);
    assert!(acts(&c).contains(r#""source":"The collection moved""#));
    c.actor = "human:alex".into();
    let page = respond(&get("/", true), &c).body;
    assert!(
        page.contains("Noted with it: The collection moved"),
        "{page}"
    );
    assert!(!page.contains("Source reference"), "{page}");
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
        .commitments()
        .iter()
        .find(|x| x.text() == "Bins go out on Tuesday.")
        .unwrap();
    assert_eq!(st.scope_of(bins.id()).map(|s| s.as_str()), Some("house"));
    assert!(
        matches!(bins.status(), canon_core::Status::Active),
        "alex is in charge of house"
    );

    // A change needs its reason, and the reason is kept.
    let id = bins.id().as_str().to_string();
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
        .commitments()
        .iter()
        .find(|x| x.text() == "Bins go out on Monday.")
        .unwrap();
    assert_eq!(
        st.scope_of(monday.id()).map(|s| s.as_str()),
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
                monday.id().as_str()
            ),
        ),
        &c,
    );
    assert_eq!(gone.status, 303);
    let (_, st) = load(&c).unwrap();
    assert!(matches!(
        st.get(monday.id()).unwrap().status(),
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
        .silences()
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
        .silences()
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

#[test]
fn setup_is_deliberate_and_uses_the_launch_target_and_actor() {
    let c = ctx("setup", &Log::from_acts(vec![]));
    std::fs::remove_dir_all(&c.dir).unwrap();
    let page = respond(&get("/", true), &c);
    assert_eq!(page.status, 200);
    assert!(page.body.contains("Create a canon"), "{}", page.body);
    assert!(page.body.contains("human:alex") && page.body.contains("house"));
    assert!(!c.dir.exists(), "looking at setup must write nothing");
    assert_eq!(respond(&at("/setup", "profile=house"), &c).status, 403);
    let invalid = respond(&at("/setup", &format!("token={TOKEN}&profile=wrong")), &c);
    assert_eq!(invalid.status, 422);
    assert!(!c.dir.exists());
    let body = format!("token={TOKEN}&profile=house&path=/ignored&actor=human:other");
    assert_eq!(respond(&at("/setup", &body), &c).status, 303);
    let (log, st) = load(&c).unwrap();
    assert_eq!(log.acts().len(), 2);
    assert!(log.acts().iter().all(|a| a.actor == "human:alex"));
    assert_eq!(st.grants()[0].scope.as_str(), "house");
    assert_eq!(st.grants()[0].actor, "human:alex");
    assert_eq!(
        crate::profile::Profile::load(&c.dir).unwrap(),
        crate::profile::Profile::House
    );
    let before = acts(&c);
    assert_ne!(respond(&at("/setup", &body), &c).status, 303);
    assert_eq!(
        acts(&c),
        before,
        "a repeated setup must not re-found the record"
    );
    assert!(respond(&get("/", true), &c).body.contains("Propose a rule"));
    std::fs::remove_file(store::file(&c.dir)).unwrap();
    assert_eq!(
        respond(&get("/", true), &c).status,
        503,
        "a lost record is not a fresh setup"
    );
}

#[test]
fn history_opens_governance_acts_and_read_failure_is_not_an_empty_queue() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("all-history", &log);
    let grant = &log.acts()[0].id;
    let page = respond(&get(&format!("/why/{grant}"), true), &c);
    assert_eq!(page.status, 200, "{}", page.body);
    std::fs::write(store::file(&c.dir), "broken record\n").unwrap();
    assert_eq!(respond(&get("/", true), &c).status, 503);
    let before = acts(&c);
    let response = respond(
        &at(
            "/decide",
            &format!("token={TOKEN}&id={quiet}&decision=approve"),
        ),
        &c,
    );
    assert_eq!(response.status, 503);
    assert!(!response.body.contains("not waiting on you"));
    assert_eq!(acts(&c), before);
}

#[test]
fn invalid_forms_retain_their_escaped_values_in_the_original_form() {
    let (log, _, _) = crate::review::tests::house();
    let c = ctx("retain", &log);
    let before = acts(&c);
    let response = respond(
        &at(
            "/propose",
            &format!("token={TOKEN}&text=%3Cscript%3Ekeep+me%3C%2Fscript%3E&scope=wrong..room"),
        ),
        &c,
    );
    assert!(
        response
            .body
            .contains("&lt;script&gt;keep me&lt;/script&gt;</textarea>"),
        "{}",
        response.body
    );
    assert!(response.body.contains("value=\"wrong..room\""));
    assert!(response.body.contains("<details class=\"act\" open>"));
    assert!(!response.body.contains("<script>"));
    assert_eq!(acts(&c), before);
}

#[test]
fn a_pending_policy_can_be_read_and_decided_from_the_page() {
    let (log, _, _) = crate::review::tests::house();
    let seat = canon_core::Act::new(
        ActKind::Grant {
            holder: "human:sam".into(),
            scope: canon_core::Scope::new("house").unwrap(),
            horizon: None,
            rationale: String::new(),
        },
        100,
        "human:alex",
    );
    let rule = canon_core::Act::new(
        ActKind::Ratification {
            text: "Both people agree to changes".into(),
            rule: canon_core::Ratify::Joint {
                holders: vec!["human:alex".into(), "human:sam".into()],
            },
            scope: Some(canon_core::Scope::new("house").unwrap()),
            governs: Some(canon_core::Governs::All),
            personal: None,
        },
        300,
        "human:alex",
    );
    let policy = canon_core::Act::new(
        ActKind::Policy {
            text: "Decide disagreements in the smallest affected room".into(),
            rule: canon_core::Rule::Subsidiarity,
            scope: Some(canon_core::Scope::new("house").unwrap()),
        },
        400,
        "human:sam",
    );
    let log = Log::from_acts([log.acts().to_vec(), vec![seat, rule, policy.clone()]].concat());
    let c = ctx("policy", &log);
    let page = respond(&get("/", true), &c);
    assert!(page.body.contains(&policy.id.to_string()), "{}", page.body);
    assert!(page.body.contains("Decide disagreements"));
    let before = log.acts().len();
    let response = respond(
        &at(
            "/decide",
            &format!("token={TOKEN}&id={}&decision=approve", policy.id),
        ),
        &c,
    );
    assert_eq!(response.status, 303, "{}", response.body);
    let (after, st) = load(&c).unwrap();
    assert_eq!(after.acts().len(), before + 1);
    assert!(st.policies().iter().any(|p| p.act == policy.id));
    assert!(review::outcome(&st, &policy.id).starts_with("in force"));
    let receipt = respond(&get(&format!("/?done={}", policy.id), true), &c);
    assert!(receipt
        .body
        .contains("Decide disagreements in the smallest affected room"));
}

#[test]
fn partial_writes_and_unread_outcomes_are_not_reported_as_unwritten_or_applied() {
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("partial", &log);
    let _reset = store::faults::after_appends(1);
    let r = respond(
        &at(
            "/propose",
            &format!("token={TOKEN}&text=Keep+my+proposal&scope=house"),
        ),
        &c,
    );
    let after = store::read(&c.dir).unwrap();
    let partial = after
        .acts()
        .iter()
        .find(|a| matches!(&a.kind, ActKind::Assert { text, .. } if text == "Keep my proposal"))
        .unwrap();
    assert_eq!(after.acts().len(), log.acts().len() + 1);
    assert!(r.body.contains("some entries may have been recorded"));
    assert!(
        r.body.contains(&format!("href=\"/why/{}\"", partial.id)),
        "{}",
        r.body
    );
    assert!(!r.body.contains("Nothing was written"));
    assert!(r.body.contains("Keep my proposal</textarea>"));
    let c = ctx("unread-outcome", &log);
    let _reset = store::faults::read_after_write();
    let r = respond(
        &at(
            "/decide",
            &format!("token={TOKEN}&id={quiet}&decision=object&because=Keep+my+reason"),
        ),
        &c,
    );
    assert!(
        r.body.to_lowercase().contains("injected read failure"),
        "{}",
        r.body.split("</style>").last().unwrap()
    );
    assert!(!r.body.contains("Nothing was written"));
    assert!(r.body.contains("Keep my reason"));
    std::fs::write(store::file(&c.dir), "broken\n").unwrap();
    assert!(matches!(
        crate::cmds::took(&c.dir, &quiet),
        crate::cmds::Took::Unknown(_)
    ));
}

#[test]
fn a_failed_founding_does_not_publish_an_empty_usable_record() {
    let c = ctx("failed-setup", &Log::from_acts(vec![]));
    std::fs::remove_file(store::file(&c.dir)).unwrap();
    let _reset = store::faults::after_appends(1);
    let response = respond(&at("/setup", &format!("token={TOKEN}&profile=house")), &c);
    assert_eq!(response.status, 422);
    assert!(!store::exists(&c.dir));
    assert!(!c.dir.join(".initializing").exists());
    assert_eq!(
        respond(&at("/setup", &format!("token={TOKEN}&profile=house")), &c).status,
        303
    );
    assert_eq!(load(&c).unwrap().0.acts().len(), 2);
}

#[test]
fn malformed_form_framing_and_nonforms_are_refused_before_writing() {
    for extra in [
        "Content-Length: 80",
        "Content-Length: wrong",
        "Content-Length: 1\r\nContent-Length: 2",
        "Transfer-Encoding: chunked",
        "",
    ] {
        let wire = format!("POST /setup HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n{extra}\r\n\r\nx");
        assert!(read_request(&mut wire.as_bytes()).is_err(), "{extra}");
    }
    let (log, quiet, _) = crate::review::tests::house();
    let c = ctx("form-type", &log);
    let mut r = at(
        "/decide",
        &format!("token={TOKEN}&id={quiet}&decision=approve"),
    );
    r.headers.retain(|(k, _)| k != "Content-Type");
    r.headers.push(("Content-Type".into(), "text/plain".into()));
    let before = acts(&c);
    assert_eq!(respond(&r, &c).status, 415);
    assert_eq!(acts(&c), before);
}

#[test]
fn an_agent_founder_does_not_gain_a_seat_and_setup_cannot_replace_a_damaged_record() {
    let mut c = ctx("agent-setup", &Log::from_acts(vec![]));
    std::fs::remove_file(store::file(&c.dir)).unwrap();
    c.actor = "agent:helper".into();
    assert_eq!(
        respond(&at("/setup", &format!("token={TOKEN}&profile=house")), &c).status,
        303
    );
    let (log, st) = load(&c).unwrap();
    assert_eq!(log.acts().len(), 1);
    assert!(st.grants().is_empty());
    std::fs::write(store::file(&c.dir), "damaged\n").unwrap();
    let before = acts(&c);
    assert_eq!(respond(&get("/", true), &c).status, 503);
    assert_eq!(
        respond(&at("/setup", &format!("token={TOKEN}&profile=code")), &c).status,
        409
    );
    assert_eq!(acts(&c), before);
    assert_eq!(
        crate::profile::Profile::load(&c.dir).unwrap(),
        crate::profile::Profile::House
    );
}
