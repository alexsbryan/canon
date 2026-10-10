// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon ui` — the record as a page, on this machine only.
//!
//! The same queue as `canon review`, for somebody who would rather read
//! than type: each proposal as the words it would put in force, a change
//! marked up against the wording it replaces, the reason and the source
//! beside it, and a button. Underneath, what is in force, and for any rule,
//! how it got there.
//!
//! **A page that can write acts is a door, so it is a narrow one.**
//!
//! - It listens on `127.0.0.1` only, on a port the OS picks, for as long as
//!   the command runs.
//! - The address it prints carries a random token. The first visit trades
//!   it for a `SameSite=Strict`, `HttpOnly` cookie; every request after must
//!   carry it, so another site open in the same browser cannot drive it.
//! - The `Host` header must name this address, which stops a DNS-rebinding
//!   page reaching it under a name it controls.
//! - A decision is a `POST` whose `Origin` (when sent) is this page and
//!   whose form carries the token again, and only about something waiting
//!   on the person who started it.
//! - The page loads nothing and runs no script: `Content-Security-Policy`
//!   allows inline style and nothing else.
//!
//! It writes as whoever started it, through the same `cmds::write` every
//! verb uses, so an identified person's decisions are signed — and a key
//! that needs a touch asks for it in the terminal that started the page.
//! Nothing here is reachable by anything that could not already run
//! `canon approve` as you.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use canon_core::{ActId, Canon, Log};

use crate::cmds::{dir, fail, positionals};
use crate::review::{self, Item, Kind};
use crate::store;

/// What a running page knows.
pub struct Ctx {
    pub dir: PathBuf,
    pub actor: String,
    pub token: String,
    pub port: u16,
}

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    fn param(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
    fn cookie(&self, name: &str) -> Option<&str> {
        self.header("cookie")?.split(';').find_map(|c| {
            let (k, v) = c.trim().split_once('=')?;
            (k == name).then_some(v)
        })
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    fn html(status: u16, body: String) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            body,
        }
    }
    fn see(location: &str) -> Self {
        Self {
            status: 303,
            headers: vec![("Location".into(), location.into())],
            body: String::new(),
        }
    }
}

const COOKIE: &str = "canon_ui";

// ── serving ─────────────────────────────────────────────────

pub fn run(args: &[String]) -> i32 {
    let _ = positionals(args);
    let d = match dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let listener = match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(l) => l,
        Err(e) => return fail(format!("could not listen on this machine: {e}")),
    };
    let port = match listener.local_addr() {
        Ok(a) => a.port(),
        Err(e) => return fail(e),
    };
    let mut raw = [0u8; 16];
    if let Err(e) = getrandom::fill(&mut raw) {
        return fail(format!("no randomness for the page's token: {e}"));
    }
    let ctx = Ctx {
        dir: d,
        actor: store::actor(),
        token: raw.iter().map(|b| format!("{b:02x}")).collect(),
        port,
    };
    let url = format!("http://127.0.0.1:{port}/?t={}", ctx.token);
    println!("canon is open for {} at\n\n  {url}\n", ctx.actor);
    println!("Only this machine can reach it, and only through that address. Ctrl-C closes it.");
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        open_browser(&url);
    }
    // A thread per connection: a browser opens connections it may never
    // use, and one idle socket must not hold the page. Writes are safe in
    // parallel, because every act is one atomic append.
    let ctx = std::sync::Arc::new(ctx);
    for stream in listener.incoming().flatten() {
        let ctx = std::sync::Arc::clone(&ctx);
        std::thread::spawn(move || {
            let _ = serve(stream, &ctx);
        });
    }
    0
}

fn open_browser(url: &str) {
    let mut cmd = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        std::process::Command::new("xdg-open")
    };
    let _ = cmd
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn serve(mut stream: TcpStream, ctx: &Ctx) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let response = match read_request(&mut stream) {
        Ok(req) => respond(&req, ctx),
        Err(why) => Response::html(
            400,
            page("Not understood", &format!("<p>{}</p>", esc(&why))),
        ),
    };
    let reason = match response.status {
        200 => "OK",
        303 => "See Other",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        421 => "Misdirected Request",
        _ => "Unprocessable Content",
    };
    let mut head = format!("HTTP/1.1 {} {reason}\r\n", response.status);
    for (k, v) in &response.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    for (k, v) in SECURITY_HEADERS {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        response.body.len()
    ));
    stream.write_all(head.as_bytes())?;
    stream.write_all(response.body.as_bytes())?;
    stream.flush()
}

/// On every response: nothing loads, nothing frames it, nothing is cached.
const SECURITY_HEADERS: [(&str, &str); 5] = [
    (
        "Content-Security-Policy",
        "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; \
         frame-ancestors 'none'; base-uri 'none'",
    ),
    ("X-Content-Type-Options", "nosniff"),
    ("X-Frame-Options", "DENY"),
    ("Referrer-Policy", "no-referrer"),
    ("Cache-Control", "no-store"),
];

/// Read one request, refusing anything larger than a person's form.
fn read_request(stream: &mut impl Read) -> Result<Request, String> {
    const HEAD: usize = 16 * 1024;
    const BODY: usize = 64 * 1024;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > HEAD {
            return Err("headers too large".into());
        }
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed early".into());
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8(buf[..end].to_vec()).map_err(|_| "headers are not text")?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (method, target) = (first.next().unwrap_or(""), first.next().unwrap_or("/"));
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    if length > BODY {
        return Err("body too large".into());
    }
    let mut body = buf[end + 4..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(length);
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok(Request {
        method: method.to_string(),
        path: decode(path),
        query: form(query.as_bytes()),
        headers,
        body,
    })
}

// ── routing ─────────────────────────────────────────────────

/// Everything a request can do, with no socket in sight — which is what
/// makes the door's checks testable.
pub fn respond(req: &Request, ctx: &Ctx) -> Response {
    let here = [
        format!("127.0.0.1:{}", ctx.port),
        format!("localhost:{}", ctx.port),
    ];
    if !req
        .header("host")
        .is_some_and(|h| here.iter().any(|x| x == h))
    {
        return Response::html(
            421,
            page(
                "Wrong address",
                "<p>This page answers only at the address <code>canon ui</code> printed.</p>",
            ),
        );
    }
    // The printed address: trade the token for a cookie, and drop it from
    // the address bar and the history.
    if req.method == "GET" && req.path == "/" {
        if let Some(t) = req.param("t") {
            if same(t, &ctx.token) {
                let mut r = Response::see("/");
                r.headers.push((
                    "Set-Cookie".into(),
                    format!("{COOKIE}={}; HttpOnly; SameSite=Strict; Path=/", ctx.token),
                ));
                return r;
            }
        }
    }
    if !req.cookie(COOKIE).is_some_and(|c| same(c, &ctx.token)) {
        return Response::html(
            403,
            page(
                "Open it from your terminal",
                "<p>This page opens only through the address <code>canon ui</code> printed when it started. Run it again for a fresh one.</p>",
            ),
        );
    }
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => home(ctx, req.param("done"), None),
        ("GET", p) if p.starts_with("/why/") => why(ctx, &p["/why/".len()..]),
        ("POST", "/decide") => decide(req, ctx),
        ("GET", _) => Response::html(
            404,
            page(
                "Not here",
                "<p><a href=\"/\">Back to what is waiting</a></p>",
            ),
        ),
        _ => Response::html(
            405,
            page(
                "Not here",
                "<p><a href=\"/\">Back to what is waiting</a></p>",
            ),
        ),
    }
}

fn decide(req: &Request, ctx: &Ctx) -> Response {
    let origins = [
        format!("http://127.0.0.1:{}", ctx.port),
        format!("http://localhost:{}", ctx.port),
    ];
    let cross = req
        .header("origin")
        .is_some_and(|o| !origins.iter().any(|x| x == o))
        || req
            .header("sec-fetch-site")
            .is_some_and(|s| s != "same-origin" && s != "none");
    let fields = form(&req.body);
    let field = |k: &str| {
        fields
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    if cross || !same(field("token"), &ctx.token) {
        return Response::html(
            403,
            page(
                "Not from this page",
                "<p>A decision has to come from this page. Nothing was written.</p>",
            ),
        );
    }
    let id = ActId::from_raw(field("id"));
    let approve = match field("decision") {
        "approve" => true,
        "object" => false,
        _ => return Response::html(422, page("Not understood", "<p>Nothing was written.</p>")),
    };
    // Only what is waiting on this person: the form is not a way to write
    // a position about anything at all.
    let waiting = load(ctx)
        .map(|(log, st)| review::queue(&log, &st, &ctx.actor, store::now()))
        .unwrap_or_default();
    if !waiting.iter().any(|i| i.yours && i.id == id) {
        return home(
            ctx,
            None,
            Some("That is not waiting on you any more. Nothing was written.".into()),
        );
    }
    match review::decide(&ctx.dir, &ctx.actor, &id, approve, field("because")) {
        Ok(_) => Response::see(&format!("/?done={}", id.as_str())),
        Err(e) => home(
            ctx,
            None,
            Some(format!("{} Nothing was written.", sentence(&e))),
        ),
    }
}

fn load(ctx: &Ctx) -> Result<(Log, Canon), String> {
    let log = store::read(&ctx.dir)?;
    let st = log.derive_at(store::now());
    Ok((log, st))
}

// ── pages ───────────────────────────────────────────────────

fn home(ctx: &Ctx, done: Option<&str>, error: Option<String>) -> Response {
    let (log, st) = match load(ctx) {
        Ok(v) => v,
        Err(e) => {
            return Response::html(
                200,
                page("canon", &format!("<p class=\"note bad\">{}</p>", esc(&e))),
            )
        }
    };
    let now = store::now();
    let items = review::queue(&log, &st, &ctx.actor, now);
    let (mine, others): (Vec<&Item>, Vec<&Item>) = items.iter().partition(|i| i.yours);
    let name = crate::lineage::name_of(&ctx.dir);
    let mut b = String::new();

    let identified = canon_core::sign::verify(log.acts())
        .bindings
        .get(&ctx.actor)
        .is_some_and(|b| !b.contested);
    let who = if identified {
        format!(
            "You are {}. What you decide here is signed with your key.",
            esc(crate::explain::person(&ctx.actor))
        )
    } else {
        format!(
            "You are {}. Anyone who can write this file could decide as you; \
             <code>canon identify</code> makes it so only your key can.",
            esc(crate::explain::person(&ctx.actor))
        )
    };
    b.push_str(&format!(
        "<header><p class=\"canon\">{}</p><p class=\"who\">{who}</p></header>",
        esc(&name)
    ));
    if let Some(e) = error {
        b.push_str(&format!(
            "<p class=\"note bad\" role=\"alert\">{}</p>",
            esc(&e)
        ));
    }
    if let Some(id) = done {
        let id = ActId::from_raw(id);
        let what = st
            .get(&id)
            .map(|c| c.text.clone())
            .or_else(|| {
                st.ratifications
                    .iter()
                    .find(|r| r.act == id)
                    .map(|r| r.text.clone())
            })
            .unwrap_or_else(|| id.as_str().to_string());
        b.push_str(&format!(
            "<p class=\"note\" role=\"status\">Recorded. <q>{}</q> is {}.</p>",
            esc(&crate::explain::one_line(&what)),
            esc(&review::outcome(&st, &id))
        ));
    }

    b.push_str("<main>");
    b.push_str(&format!("<h1>{}</h1>", esc(&waiting_line(mine.len()))));
    if mine.is_empty() {
        b.push_str("<p class=\"quiet\">When someone proposes a rule for a room you hold, it appears here for you to approve or object to.</p>");
    }
    let touch = canon_core::sign::verify(log.acts())
        .bindings
        .get(&ctx.actor)
        .is_some_and(|bd| bd.keys.iter().any(canon_core::Key::touch_required));
    for item in &mine {
        b.push_str(&motion(ctx, item, now, touch));
    }

    if !others.is_empty() {
        b.push_str(&format!(
            "<details class=\"others\"><summary>{} waiting on someone else</summary><ul>",
            others.len()
        ));
        for item in &others {
            b.push_str(&format!(
                "<li><span class=\"rule\">{}</span><span class=\"meta\">Needs {}</span></li>",
                esc(&item.text),
                esc(&item.needs)
            ));
        }
        b.push_str("</ul></details>");
    }

    b.push_str(&in_force(&st));
    b.push_str("</main>");
    Response::html(200, page(&format!("{} — canon", name), &b))
}

fn waiting_line(n: usize) -> String {
    match n {
        0 => "Nothing is waiting on you.".into(),
        1 => "One proposal is waiting on you.".into(),
        n => format!("{} proposals are waiting on you.", words(n)),
    }
}

fn words(n: usize) -> String {
    const SMALL: [&str; 10] = [
        "Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine",
    ];
    SMALL
        .get(n)
        .map_or_else(|| n.to_string(), |w| w.to_string())
}

/// One proposal: the words it would put in force, and the decision.
fn motion(ctx: &Ctx, item: &Item, now: i64, touch: bool) -> String {
    let mut m = format!("<article class=\"motion {}\">", kind_class(item.kind));
    let room = item
        .scope
        .as_ref()
        .map_or_else(|| "for the whole canon".to_string(), |s| format!("for {s}"));
    m.push_str(&format!(
        "<p class=\"meta\">{} {room}, proposed by {} {}.</p>",
        esc(&capital(item.kind.noun())),
        esc(crate::explain::person(&item.by)),
        esc(&review::ago(item.at, now))
    ));
    match item.kind {
        Kind::Change if item.replaces.len() == 1 => {
            m.push_str(&format!(
                "<p class=\"rule amend\">{}</p>",
                amendment(&item.replaces[0].1, &item.text)
            ));
        }
        Kind::Change => {
            for (_, old) in &item.replaces {
                m.push_str(&format!(
                    "<p class=\"rule gone\"><del>{}</del></p>",
                    esc(old)
                ));
            }
            m.push_str(&format!(
                "<p class=\"rule\"><ins>{}</ins></p>",
                esc(&item.text)
            ));
        }
        Kind::Retraction => {
            m.push_str(&format!(
                "<p class=\"rule gone\"><del>{}</del></p>",
                esc(&item.text)
            ));
        }
        Kind::Rule | Kind::DecisionRule => {
            m.push_str(&format!("<p class=\"rule\">{}</p>", esc(&item.text)));
        }
    }
    if !item.because.is_empty() {
        m.push_str(&format!(
            "<figure class=\"because\"><blockquote>{}</blockquote><figcaption>{}, on why</figcaption></figure>",
            esc(&item.because),
            esc(crate::explain::person(&item.by))
        ));
    }
    if let Some(src) = &item.source {
        match review::passage(&ctx.dir, src) {
            Some(p) => m.push_str(&format!(
                "<figure class=\"source\"><blockquote>{}</blockquote><figcaption>{}</figcaption></figure>",
                esc(&p),
                esc(src)
            )),
            None => m.push_str(&format!("<p class=\"meta\">Drafted from {}</p>", esc(src))),
        }
    }
    m.push_str(&format!(
        "<p class=\"needs\">Needs {}</p>",
        esc(&item.needs)
    ));
    let hidden = format!(
        "<input type=\"hidden\" name=\"token\" value=\"{}\"><input type=\"hidden\" name=\"id\" value=\"{}\">",
        esc(&ctx.token),
        esc(item.id.as_str())
    );
    m.push_str(&format!(
        "<div class=\"decide\"><form method=\"post\" action=\"/decide\">{hidden}\
         <button name=\"decision\" value=\"approve\" class=\"carry\">{}</button></form>\
         <details><summary>Object</summary><form method=\"post\" action=\"/decide\">{hidden}\
         <label>Your reason, which everyone will see<textarea name=\"because\" required rows=\"3\"></textarea></label>\
         <button name=\"decision\" value=\"object\" class=\"object\">Object</button></form></details></div>",
        if item.kind == Kind::Retraction { "Approve taking it away" } else { "Approve" }
    ));
    if touch {
        m.push_str("<p class=\"meta\">Your key will ask for a touch in the terminal running this page.</p>");
    }
    m.push_str("</article>");
    m
}

fn kind_class(k: Kind) -> &'static str {
    match k {
        Kind::Rule => "new",
        Kind::Change => "change",
        Kind::Retraction => "retract",
        Kind::DecisionRule => "procedure",
    }
}

/// An error as a sentence: capital first, full stop last.
fn sentence(s: &str) -> String {
    let s = capital(s.trim());
    if s.ends_with(['.', '!', '?']) {
        s
    } else {
        format!("{s}.")
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// The charter: what is in force now, by room.
fn in_force(st: &Canon) -> String {
    let mut rooms: std::collections::BTreeMap<String, Vec<&canon_core::Commitment>> =
        std::collections::BTreeMap::new();
    for c in st.active() {
        let room = st
            .scope_of(&c.id)
            .map_or_else(|| "The whole canon".to_string(), |s| s.as_str().to_string());
        rooms.entry(room).or_default().push(c);
    }
    let n: usize = rooms.values().map(Vec::len).sum();
    let mut b = format!(
        "<section class=\"charter\"><h2>In force</h2><p class=\"quiet\">{}</p>",
        match n {
            0 => "No rules yet.".to_string(),
            1 => "One rule.".to_string(),
            n => format!("{} rules.", words(n)),
        }
    );
    for (room, rules) in rooms {
        b.push_str(&format!("<h3>{}</h3><ul>", esc(&room)));
        for c in rules {
            b.push_str(&format!(
                "<li><a href=\"/why/{}\">{}</a></li>",
                esc(c.id.as_str()),
                esc(&c.text)
            ));
        }
        b.push_str("</ul>");
    }
    b.push_str("</section>");
    b
}

/// How a rule got here: `canon why`, as a page, with every id a link.
fn why(ctx: &Ctx, needle: &str) -> Response {
    let (log, st) = match load(ctx) {
        Ok(v) => v,
        Err(e) => {
            return Response::html(
                200,
                page("canon", &format!("<p class=\"note bad\">{}</p>", esc(&e))),
            )
        }
    };
    let Ok(id) = crate::explain::resolve_any(&st, needle) else {
        return Response::html(
            404,
            page("Not in this canon", "<p><a href=\"/\">Back</a></p>"),
        );
    };
    let Ok(e) = crate::explain::explain(&log, &st, &id) else {
        return Response::html(
            404,
            page("Not in this canon", "<p><a href=\"/\">Back</a></p>"),
        );
    };
    let text = e
        .headline
        .split_once("  ")
        .map_or(e.headline.as_str(), |(_, t)| t);
    let mut b = format!(
        "<header><p class=\"canon\"><a href=\"/\">{}</a></p></header><main><p class=\"rule lone\">{}</p><ul class=\"history\">",
        esc(&crate::lineage::name_of(&ctx.dir)),
        esc(text)
    );
    for line in &e.lines {
        let depth = if line.starts_with(' ') {
            " class=\"sub\""
        } else {
            ""
        };
        b.push_str(&format!("<li{depth}>{}</li>", linked(line.trim())));
    }
    b.push_str("</ul></main>");
    Response::html(200, page(&crate::explain::one_line(text), &b))
}

/// Escape, then turn every act id into a link to its history.
fn linked(text: &str) -> String {
    let escaped = esc(text);
    let mut out = String::new();
    let mut rest = escaped.as_str();
    while let Some(at) = rest.find("can-") {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let id: String = from.chars().take(16).collect();
        if id.len() == 16 && id[4..].chars().all(|c| c.is_ascii_hexdigit()) {
            out.push_str(&format!("<a href=\"/why/{id}\">{id}</a>"));
            rest = &from[16..];
        } else {
            out.push_str("can-");
            rest = &from[4..];
        }
    }
    out.push_str(rest);
    out
}

/// The old wording struck and the new written in, word by word: an
/// amendment as a minute book would mark it.
fn amendment(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.split_whitespace().collect();
    let b: Vec<&str> = new.split_whitespace().collect();
    if a.len() * b.len() > 40_000 {
        return format!("<del>{}</del> <ins>{}</ins>", esc(old), esc(new));
    }
    // Longest common subsequence, then walk it.
    let mut lcs = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut parts: Vec<(char, Vec<&str>)> = Vec::new();
    fn push<'a>(kind: char, w: &'a str, parts: &mut Vec<(char, Vec<&'a str>)>) {
        match parts.last_mut() {
            Some((k, ws)) if *k == kind => ws.push(w),
            _ => parts.push((kind, vec![w])),
        }
    }
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            push('=', a[i], &mut parts);
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            // What goes, before what replaces it: "two three", read struck.
            push('-', a[i], &mut parts);
            i += 1;
        } else {
            push('+', b[j], &mut parts);
            j += 1;
        }
    }
    parts
        .iter()
        .map(|(k, ws)| {
            let t = esc(&ws.join(" "));
            match k {
                '+' => format!("<ins>{t}</ins>"),
                '-' => format!("<del>{t}</del>"),
                _ => t,
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── the frame ───────────────────────────────────────────────

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>{}</title><style>{STYLE}</style></head><body>{body}</body></html>",
        esc(title)
    )
}

const STYLE: &str = r#"
:root{
  --paper:#f2f4f6; --sheet:#fbfcfd; --ink:#1d2633; --muted:#6b7686; --rule:#d5dbe2;
  --carry:#2e6a4f; --object:#8e2f3c; --mark:#e3eee8;
  --serif:"Iowan Old Style","Palatino Linotype",Palatino,"URW Palladio L",P052,"Book Antiqua",serif;
  --sans:Seravek,"Gill Sans Nova",Ubuntu,Calibri,"Source Sans 3","DejaVu Sans",sans-serif;
  color-scheme:light dark;
}
@media (prefers-color-scheme:dark){:root{
  --paper:#161c24; --sheet:#1c232d; --ink:#e4e9ef; --muted:#9aa5b3; --rule:#2f3946;
  --carry:#7cc4a0; --object:#e58a97; --mark:#21382e;
}}
*{box-sizing:border-box}
html{background:var(--paper)}
body{margin:0 auto;max-width:42rem;padding:2.5rem 1.25rem 5rem;color:var(--ink);
  font:1.0625rem/1.55 var(--sans)}
a{color:inherit;text-decoration-color:var(--rule);text-underline-offset:.2em}
a:hover{text-decoration-color:currentColor}
:focus-visible{outline:2px solid var(--carry);outline-offset:3px;border-radius:2px}
header{display:flex;flex-wrap:wrap;gap:.25rem 1.5rem;justify-content:space-between;
  align-items:baseline;padding-bottom:1rem;border-bottom:1px solid var(--rule);margin-bottom:2.5rem}
.canon{margin:0;font:600 1.0625rem/1.3 var(--serif);letter-spacing:.01em}
.who{margin:0;color:var(--muted);font-size:.875rem;max-width:30rem}
h1{font:400 1.75rem/1.25 var(--serif);margin:0 0 1.75rem}
h2{font:400 1.375rem/1.3 var(--serif);margin:0 0 .25rem}
h3{font:600 .9375rem/1.4 var(--sans);color:var(--muted);margin:1.75rem 0 .5rem}
.quiet{color:var(--muted);margin:0 0 1.5rem}
.note{padding:.75rem 1rem;background:var(--mark);border-radius:4px;margin:0 0 2rem}
.note.bad{background:transparent;border:1px solid var(--object);color:var(--object)}
.motion{background:var(--sheet);border:1px solid var(--rule);border-radius:6px;
  padding:1.5rem 1.5rem 1.25rem;margin:0 0 1.5rem}
.meta{color:var(--muted);font-size:.875rem;margin:0 0 .75rem}
.rule{font:1.5rem/1.45 var(--serif);margin:0 0 1rem;max-width:34em;hyphens:auto}
.rule.lone{font-size:1.75rem;margin-bottom:1.5rem}
.rule.gone{color:var(--muted)}
del{text-decoration:line-through;text-decoration-color:var(--object);text-decoration-thickness:.08em;
  color:var(--muted)}
ins{text-decoration:none;background:var(--mark);box-shadow:inset 0 -.12em 0 var(--carry);
  padding:0 .08em;border-radius:2px}
.because{margin:0 0 1rem}
.because blockquote{margin:0;font:italic 1.0625rem/1.5 var(--serif)}
.because blockquote::before{content:"\201C"}.because blockquote::after{content:"\201D"}
.because figcaption{color:var(--muted);font-size:.8125rem;margin-top:.125rem}
.source{margin:0 0 1rem}
.source blockquote{margin:0;padding:.125rem 0 .125rem 1rem;border-left:2px solid var(--rule);
  font-family:var(--serif);white-space:pre-wrap}
.source figcaption{color:var(--muted);font-size:.8125rem;margin-top:.25rem}
.needs{font-size:.9375rem;margin:0 0 1.25rem}
.decide{display:flex;flex-wrap:wrap;gap:.75rem;align-items:flex-start}
.decide form{margin:0}
.decide details[open]{flex-basis:100%}
.decide summary{display:inline-block;cursor:pointer;list-style:none;font:inherit;font-size:.9375rem;
  padding:.5rem 1rem;border:1px solid var(--rule);border-radius:4px;color:var(--object)}
.decide summary::-webkit-details-marker{display:none}
.decide details[open] summary{border-color:var(--object)}
.decide label{display:block;font-size:.875rem;color:var(--muted);margin:.75rem 0 .25rem}
textarea{display:block;width:100%;margin-top:.25rem;font:1rem/1.45 var(--serif);color:var(--ink);
  background:var(--paper);border:1px solid var(--rule);border-radius:4px;padding:.5rem .625rem}
button{font:inherit;font-size:.9375rem;padding:.5rem 1.125rem;border-radius:4px;cursor:pointer;
  border:1px solid transparent}
button.carry{background:var(--carry);color:var(--sheet)}
button.object{background:transparent;color:var(--object);border-color:var(--object);margin-top:.5rem}
.others{margin:2rem 0 0;color:var(--muted)}
.others summary{cursor:pointer}
.others ul{list-style:none;padding:0;margin:.75rem 0 0}
.others li{padding:.5rem 0;border-top:1px solid var(--rule)}
.others .rule{display:block;font-size:1.0625rem;margin:0;color:var(--ink)}
.others .meta{display:block;margin:0}
.charter{margin-top:4rem;padding-top:2rem;border-top:1px solid var(--rule)}
.charter ul{list-style:none;margin:0;padding:0;font:1.125rem/1.5 var(--serif)}
.charter li{margin:.5rem 0}
.history{list-style:none;padding:0;margin:0}
.history li{padding:.5rem 0;border-top:1px solid var(--rule)}
.history li.sub{border-top:0;padding-top:0;padding-left:1.25rem;color:var(--muted)}
code{font-size:.9em}
@media (max-width:30rem){.rule{font-size:1.3125rem}.motion{padding:1.25rem 1rem 1rem}h1{font-size:1.5rem}}
@media (prefers-reduced-motion:reduce){*{transition:none!important}}
"#;

// ── small things ────────────────────────────────────────────

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Compare two secrets without saying how much of them matched.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""),
                    16,
                ) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `application/x-www-form-urlencoded`, which is also a query string.
fn form(body: &[u8]) -> Vec<(String, String)> {
    String::from_utf8_lossy(body)
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect()
}

#[cfg(test)]
mod tests;
