//! A real-browser check that untrusted values cannot run code (-010).
//!
//! The server renders pages with real `#[derive(Hemplate)]` templates, putting
//! the classic injection payloads into every context a value may still reach.
//! A driver script in the browser loads each page in an iframe, hovers, focuses
//! and clicks every element, and reports what happened. Two author-written
//! control pages must execute and fetch, which proves the driver can see both;
//! the payload pages must do neither.
//!
//! ```console
//! cargo run -p hemplate-browser-vectors
//! # open the printed URL in a browser; the process exits 0 only if every check holds
//! ```

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hemplate::{Descriptor, Hemplate, Srcset, StyleValue, Url};

/// Strings that executed or fetched when HTML escaping was the only defense.
/// `VECTOR` stands for the second port, the host that must never be contacted.
const PAYLOADS: [&str; 9] = [
    "javascript:top.__pwned='scheme'",
    "');top.__pwned='handler';('",
    "\" onmouseover=\"top.__pwned='breakout'\" x=\"",
    "'><img src=x onerror=top.__pwned='tag'>",
    "&quot; onfocus=&quot;top.__pwned='entity'",
    "red; background: url(http://VECTOR/track.png)",
    "data:text/html,<script>top.__pwned='data'</script>",
    "java\tscript:top.__pwned='tab'",
    "</script><script>top.__pwned='script'</script>",
];

/// A color the application builds itself: six hex digits, nothing else.
struct Rgb(String);

impl Rgb {
    fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| Self(format!("color: #{hex}")))
    }
}

impl std::fmt::Display for Rgb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl StyleValue for Rgb {}

#[derive(Hemplate)]
struct VectorText {
    payload: String,
}

#[derive(Hemplate)]
struct VectorLink {
    url: Url,
}

#[derive(Hemplate)]
struct VectorTyped {
    link: String,
    color: Rgb,
    srcset: Srcset,
}

fn typed_page(payload: &str, vector: &str) -> String {
    let payload = payload.replace("VECTOR", vector);
    // The application meets the payload at the boundary and decides: a bad URL
    // gives no link, a bad color the default, a bad candidate no image.
    let link = match Url::parse(&payload) {
        Ok(url) => VectorLink { url }.render().unwrap(),
        Err(_) => "<span id=\"link\">no link: URL not accepted</span>".to_string(),
    };
    let color = Rgb::parse(&payload).unwrap_or_else(|| Rgb("color: #000000".into()));
    let srcset = Url::parse(&payload)
        .ok()
        .and_then(|url| {
            Srcset::new()
                .candidate(&url, Some(Descriptor::Width(1)))
                .ok()
        })
        .unwrap_or_default();
    VectorTyped {
        link,
        color,
        srcset,
    }
    .render()
    .unwrap()
}

/// Author-written controls: they must execute and fetch, or the driver is blind.
fn control_page(vector: &str) -> String {
    // What context-blind escaping used to render: each of these must run or fetch.
    format!(
        "<script>function g() {{}}</script><main id=\"page\">\
         <a id=\"control-link\" href=\"javascript:top.__pwned = (top.__pwned || '') + 'link '\">js</a>\
         <button id=\"control-handler\" onclick=\"g('&#x27;);top.__pwned = (top.__pwned || &#x27;&#x27;) + &#x27;handler &#x27;;(&#x27;')\">h</button>\
         <p id=\"control-style\" style=\"background-image: url(http://{vector}/style.png)\">s</p>\
         <img id=\"control-image\" src=\"http://{vector}/control.png\"></main>"
    )
}

fn document(body: &str) -> String {
    format!("<!doctype html><meta charset=\"utf-8\"><title>case</title>{body}")
}

struct State {
    vector_hits: AtomicUsize,
    vector: String,
    report: Mutex<Option<BTreeMap<String, String>>>,
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

/// Read one request: method, path and body.
fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..read]);
        if let Some(at) = data.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let length = head
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse().unwrap_or(0))
        })
        .unwrap_or(0usize);
    while data.len() < head_end + length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let mut first = head.lines().next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let body = String::from_utf8_lossy(&data[head_end..]).to_string();
    Some((method, path, body))
}

fn serve_app(listener: TcpListener, state: Arc<State>) {
    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            let mut stream = stream;
            let Some((method, path, body)) = read_request(&mut stream) else {
                return;
            };
            match (method.as_str(), path.as_str()) {
                ("GET", "/") => respond(&mut stream, "200 OK", "text/html", &index()),
                ("GET", "/driver.js") => respond(&mut stream, "200 OK", "text/javascript", DRIVER),
                ("GET", "/cases") => {
                    let mut cases = vec!["control".to_string()];
                    for index in 0..PAYLOADS.len() {
                        cases.push(format!("text/{index}"));
                        cases.push(format!("typed/{index}"));
                    }
                    respond(&mut stream, "200 OK", "text/plain", &cases.join("\n"));
                }
                ("GET", "/reset-vector") => {
                    state.vector_hits.store(0, Ordering::SeqCst);
                    respond(&mut stream, "200 OK", "text/plain", "0");
                }
                ("GET", "/vector-hits") => {
                    let hits = state.vector_hits.load(Ordering::SeqCst);
                    respond(&mut stream, "200 OK", "text/plain", &hits.to_string());
                }
                ("POST", "/report") => {
                    let fields = body
                        .lines()
                        .filter_map(|line| line.split_once('='))
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    *state.report.lock().unwrap() = Some(fields);
                    respond(&mut stream, "200 OK", "text/plain", "ok");
                }
                ("GET", page) if page.starts_with("/case/") => {
                    let name = &page["/case/".len()..];
                    let body = match name.split_once('/') {
                        None if name == "control" => control_page(&state.vector),
                        Some((kind, index)) => {
                            let payload = index.parse::<usize>().ok().and_then(|i| PAYLOADS.get(i));
                            match (kind, payload) {
                                ("text", Some(p)) => VectorText {
                                    payload: p.replace("VECTOR", &state.vector),
                                }
                                .render()
                                .unwrap(),
                                ("typed", Some(p)) => typed_page(p, &state.vector),
                                _ => {
                                    return respond(
                                        &mut stream,
                                        "404 Not Found",
                                        "text/plain",
                                        "no case",
                                    )
                                }
                            }
                        }
                        _ => return respond(&mut stream, "404 Not Found", "text/plain", "no case"),
                    };
                    respond(&mut stream, "200 OK", "text/html", &document(&body));
                }
                // Targets of the relative links the driver clicks.
                _ => respond(
                    &mut stream,
                    "200 OK",
                    "text/html",
                    &document("<p>navigated</p>"),
                ),
            }
        });
    }
}

fn serve_vector(listener: TcpListener, state: Arc<State>) {
    for stream in listener.incoming().flatten() {
        state.vector_hits.fetch_add(1, Ordering::SeqCst);
        let mut stream = stream;
        let _ = read_request(&mut stream);
        respond(&mut stream, "204 No Content", "text/plain", "");
    }
}

fn index() -> String {
    "<!doctype html><meta charset=\"utf-8\"><title>hemplate vectors</title>\
     <pre id=\"log\">running…</pre><script src=\"/driver.js\"></script>"
        .to_string()
}

fn main() {
    let app = TcpListener::bind("127.0.0.1:0").expect("bind app");
    let vector = TcpListener::bind("127.0.0.1:0").expect("bind vector host");
    let state = Arc::new(State {
        vector_hits: AtomicUsize::new(0),
        vector: vector.local_addr().unwrap().to_string(),
        report: Mutex::new(None),
    });
    println!(
        "{{\"phase\":\"ready\",\"url\":\"http://{}/\",\"vector_host\":\"{}\"}}",
        app.local_addr().unwrap(),
        state.vector
    );
    {
        let state = Arc::clone(&state);
        std::thread::spawn(move || serve_app(app, state));
    }
    {
        let state = Arc::clone(&state);
        std::thread::spawn(move || serve_vector(vector, state));
    }
    let waited = (0..600).find_map(|_| {
        std::thread::sleep(Duration::from_millis(100));
        state.report.lock().unwrap().take()
    });
    let Some(report) = waited else {
        eprintln!("the browser did not report within 60 s");
        std::process::exit(2);
    };
    let failures = check(&report, state.vector_hits.load(Ordering::SeqCst));
    for failure in &failures {
        eprintln!("FAIL {failure}");
    }
    println!(
        "{{\"phase\":\"result\",\"cases\":{},\"failures\":{}}}",
        report.get("cases").map_or("0", String::as_str),
        failures.len()
    );
    std::process::exit(i32::from(!failures.is_empty()));
}

/// Judge the browser's report. Every assertion here is a rule about the page.
fn check(report: &BTreeMap<String, String>, vector_hits_after: usize) -> Vec<String> {
    let mut failures = Vec::new();
    let get = |key: &str| {
        report
            .get(key)
            .map_or("<missing>".to_string(), |v| v.clone())
    };
    let mut fail_if = |bad: bool, what: String| {
        if bad {
            failures.push(what);
        }
    };
    // The controls prove the driver sees execution and fetching at all.
    let pwned = get("control.pwned");
    fail_if(
        !pwned.contains("link"),
        "control javascript: link did not execute".into(),
    );
    fail_if(
        !pwned.contains("handler"),
        "control decoded handler did not execute".into(),
    );
    let fetched: usize = get("control.fetched").parse().unwrap_or(0);
    fail_if(
        fetched < 2,
        "control image and style were not both fetched".into(),
    );
    fail_if(
        get("cases") != (PAYLOADS.len() * 2 + 1).to_string(),
        "not every page ran".into(),
    );
    fail_if(get("clicked") == "0", "nothing was clicked".into());
    for index in 0..PAYLOADS.len() {
        for kind in ["text", "typed"] {
            let case = format!("{kind}.{index}");
            let field = |name: &str| get(&format!("{case}.{name}"));
            fail_if(
                field("rendered") != "yes",
                format!("{case}: page did not render"),
            );
            fail_if(field("pwned") != "none", format!("{case}: code ran"));
            fail_if(
                field("foreign") != "0",
                format!("{case}: payload added markup"),
            );
            fail_if(
                field("fetched") != "0",
                format!("{case}: vector host contacted"),
            );
        }
    }
    fail_if(
        vector_hits_after != 0,
        format!("vector host contacted {vector_hits_after} times"),
    );
    failures
}

/// Runs in the browser.
const DRIVER: &str = r#"
const log = (text) => { document.getElementById("log").textContent += "\n" + text; };
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const report = {};
let clicked = 0;

async function open(path) {
  const frame = document.createElement("iframe");
  document.body.append(frame);
  await new Promise((resolve) => { frame.onload = resolve; frame.src = path; });
  return frame;
}

// Hover, focus and click everything: handlers and javascript: URLs run on events.
async function exercise(frame) {
  const doc = frame.contentDocument;
  const elements = [...doc.querySelectorAll("*")];
  for (const el of elements) {
    el.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
    el.dispatchEvent(new MouseEvent("mouseenter"));
    el.dispatchEvent(new Event("focus"));
    if (el.matches("a, button, input")) { el.click(); clicked += 1; }
  }
  await pause(120);
}

// Anything an attacker could add: other elements, handlers, new attributes.
function foreign(page, allowed) {
  let count = 0;
  for (const el of page.querySelectorAll("*")) {
    if (!allowed.includes(el.localName)) count += 1;
    for (const name of el.getAttributeNames()) {
      if (name.startsWith("on") && !(el.localName === "button" && name === "onclick")) count += 1;
    }
  }
  return count;
}

async function run() {
  const cases = (await (await fetch("/cases")).text()).split("\n");
  for (const name of cases) {
    const isControl = name === "control";
    top.__pwned = undefined;
    const frame = await open("/case/" + name);
    const key = name.replace("/", ".");
    // Measure the page as rendered, before a click navigates the frame away.
    if (!isControl) {
      const page = frame.contentDocument.getElementById("page");
      report[key + ".rendered"] = page && page.children.length > 0 ? "yes" : "no";
      report[key + ".foreign"] = page ? String(foreign(page, ["p", "input", "a", "button", "span", "img"])) : "no page";
    }
    await exercise(frame);
    report[key + ".pwned"] = top.__pwned === undefined ? "none" : String(top.__pwned);
    const hits = Number(await (await fetch("/vector-hits")).text());
    report[key + ".fetched"] = String(hits);
    // Only the control may reach the vector host; count from zero after it.
    if (isControl) await fetch("/reset-vector");
    frame.remove();
  }
  report.cases = String(cases.length);
  report.clicked = String(clicked);
  // Hits are judged by the server after the report: a late request still counts.
  await pause(300);
  await fetch("/report", { method: "POST", body: Object.entries(report).map(([k, v]) => k + "=" + v).join("\n") });
  log("done: " + cases.length + " pages");
}
run().catch((error) => { log("driver error: " + error); fetch("/report", { method: "POST", body: "error=" + error }); });
"#;
