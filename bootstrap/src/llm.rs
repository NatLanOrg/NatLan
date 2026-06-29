// Minimal OpenAI-compatible chat client over raw TCP (works against Ollama at /v1).
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

// Set once an endpoint rejects `response_format`, so the rest of the run skips structured output
// instead of paying a rejected request plus a fallback on every call.
static STRUCTURED_UNSUPPORTED: AtomicBool = AtomicBool::new(false);

// Generation-speed meter. Each successful request records its output tokens and its timing. Streamed
// calls additionally split time to first token (TTFT) from the decode window, so the benchmark can
// report the two separately (see docs/benchmark/scoring.md#throughput). The benchmark resets the meter
// before its cases and reads the snapshot after. The counters are global but only the benchmark reads
// them; the build records into them harmlessly.
static GEN_TOKENS: AtomicU64 = AtomicU64::new(0);
static GEN_TOTAL_MICROS: AtomicU64 = AtomicU64::new(0); // request → last token, all calls
static GEN_TTFT_MICROS: AtomicU64 = AtomicU64::new(0); // request → first token, streamed calls only
static GEN_DECODE_MICROS: AtomicU64 = AtomicU64::new(0); // first → last token, streamed calls only
static GEN_CALLS: AtomicU64 = AtomicU64::new(0);
static GEN_STREAM_CALLS: AtomicU64 = AtomicU64::new(0);

// A snapshot of the meter. Seconds are totals across calls; the benchmark derives means and rates.
pub struct Throughput {
    pub tokens: u64,
    pub total_secs: f64,
    pub ttft_secs: f64,
    pub decode_secs: f64,
    pub stream_calls: u64,
}

// Reset the speed meter to zero.
pub fn throughput_reset() {
    for c in [&GEN_TOKENS, &GEN_TOTAL_MICROS, &GEN_TTFT_MICROS, &GEN_DECODE_MICROS, &GEN_CALLS, &GEN_STREAM_CALLS] {
        c.store(0, Ordering::Relaxed);
    }
}

pub fn throughput_snapshot() -> Throughput {
    Throughput {
        tokens: GEN_TOKENS.load(Ordering::Relaxed),
        total_secs: GEN_TOTAL_MICROS.load(Ordering::Relaxed) as f64 / 1_000_000.0,
        ttft_secs: GEN_TTFT_MICROS.load(Ordering::Relaxed) as f64 / 1_000_000.0,
        decode_secs: GEN_DECODE_MICROS.load(Ordering::Relaxed) as f64 / 1_000_000.0,
        stream_calls: GEN_STREAM_CALLS.load(Ordering::Relaxed),
    }
}

// A non-streamed call: only the blended request → response time is known.
fn record_generation(tokens: u64, elapsed: Duration) {
    GEN_TOKENS.fetch_add(tokens, Ordering::Relaxed);
    GEN_TOTAL_MICROS.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
    GEN_CALLS.fetch_add(1, Ordering::Relaxed);
}

// A streamed call: TTFT and the decode window are measured separately.
fn record_generation_stream(tokens: u64, ttft: Duration, decode: Duration) {
    GEN_TOKENS.fetch_add(tokens, Ordering::Relaxed);
    GEN_TTFT_MICROS.fetch_add(ttft.as_micros() as u64, Ordering::Relaxed);
    GEN_DECODE_MICROS.fetch_add(decode.as_micros() as u64, Ordering::Relaxed);
    GEN_TOTAL_MICROS.fetch_add((ttft + decode).as_micros() as u64, Ordering::Relaxed);
    GEN_CALLS.fetch_add(1, Ordering::Relaxed);
    GEN_STREAM_CALLS.fetch_add(1, Ordering::Relaxed);
}

// Verbose request logging: when on, each LLM call logs its label, outcome, and duration so the user
// can see what is being compiled and linked through a slow model. Enabled by the CLI (`jazyk build`
// / `watch`) via `set_verbose`, or by the `JAZYK_VERBOSE` env var for any frontend (lazy default).
static VERBOSE: AtomicBool = AtomicBool::new(false);
static VERBOSE_INIT: AtomicBool = AtomicBool::new(false);
pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
    VERBOSE_INIT.store(true, Ordering::Relaxed);
}
fn verbose() -> bool {
    if !VERBOSE_INIT.load(Ordering::Relaxed) {
        let on = std::env::var("JAZYK_VERBOSE").map(|v| !v.is_empty() && v != "0").unwrap_or(false);
        set_verbose(on);
    }
    VERBOSE.load(Ordering::Relaxed)
}

// Global cap on concurrent in-flight LLM requests, so parallel compilation/linking does not
// overwhelm the backend (a local Ollama serializes work and 502s under heavy fan-out). Tunable
// with JAZYK_MAX_CONCURRENCY; default 6.
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}
static SEM: OnceLock<Semaphore> = OnceLock::new();
fn semaphore() -> &'static Semaphore {
    SEM.get_or_init(|| {
        let n = std::env::var("JAZYK_MAX_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(6)
            .max(1);
        Semaphore { permits: Mutex::new(n), cv: Condvar::new() }
    })
}
struct Permit;
fn acquire() -> Permit {
    let s = semaphore();
    let mut p = s.permits.lock().unwrap();
    while *p == 0 {
        p = s.cv.wait(p).unwrap();
    }
    *p -= 1;
    Permit
}
impl Drop for Permit {
    fn drop(&mut self) {
        let s = semaphore();
        let mut p = s.permits.lock().unwrap();
        *p += 1;
        s.cv.notify_one();
    }
}

// Number of retries (in addition to the first attempt) for failed LLM calls. Tunable with
// JAZYK_MAX_RETRIES; default 2.
fn max_retries() -> usize {
    std::env::var("JAZYK_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(2)
}

// Whether an error looks transient (worth retrying) versus a hard client error.
fn is_transient(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("502")
        || e.contains("503")
        || e.contains("504")
        || e.contains("bad gateway")
        || e.contains("service unavailable")
        || e.contains("gateway timeout")
        || e.contains("request failed")
        || e.contains("timed out")
        || e.contains("connect ")
        || e.contains("read:")
        || e.contains("write:")
        || e.contains("no http body")
}

#[derive(Clone)]
pub struct Llm {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    // Sampling temperature. Defaults to 0 for deterministic builds, but some models (e.g. newer
    // OpenAI ones) only allow their default; `None` omits the field entirely.
    pub temperature: Option<f64>,
}

impl Llm {
    // Chat expecting structured JSON, constrained by a JSON Schema sent as `response_format`
    // (OpenAI-compatible structured output). `schema_name` labels the call; `schema` is the bare
    // JSON Schema for the reply. Retries when the response is not valid JSON of a usable shape
    // (truncated / wrapped in prose), in addition to the transport retries in `chat`. If the
    // endpoint rejects structured output, `chat_inner` falls back to prompt-only JSON. `label`
    // identifies the call (file and stage, or linked entity) in retry and verbose log lines.
    pub fn chat_json(&self, system: &str, user: &str, schema_name: &str, schema: &Value, label: &str) -> Result<Value, String> {
        self.chat_json_impl(system, user, schema_name, schema, label, false)
    }

    // Like `chat_json`, but streams the response so the throughput meter can split time to first token
    // from the decode rate. Used by the benchmark; the build uses the non-streamed `chat_json`.
    pub fn chat_json_stream(&self, system: &str, user: &str, schema_name: &str, schema: &Value, label: &str) -> Result<Value, String> {
        self.chat_json_impl(system, user, schema_name, schema, label, true)
    }

    fn chat_json_impl(&self, system: &str, user: &str, schema_name: &str, schema: &Value, label: &str, stream: bool) -> Result<Value, String> {
        let rf = json!({
            "type": "json_schema",
            "json_schema": { "name": schema_name, "strict": true, "schema": schema }
        });
        let max = max_retries();
        let mut last = String::new();
        for attempt in 0..=max {
            let content = self.chat_fmt(system, user, Some(&rf), label, stream)?;
            match extract_json_object(&content) {
                Some(obj) => match serde_json::from_str::<Value>(&obj) {
                    Ok(v) => return Ok(v),
                    Err(e) => last = format!("bad JSON: {} :: {}", e, truncate(&obj, 300)),
                },
                None => last = format!("no JSON object in response: {}", truncate(&content, 300)),
            }
            if attempt < max {
                // Retry immediately (no backoff): the backend serializes work behind the concurrency
                // cap, so delaying only adds latency.
                eprintln!("[jazyk] {} — malformed JSON, retrying ({}/{}): {}", label, attempt + 1, max, truncate(&last, 160));
            }
        }
        Err(last)
    }

    // Chat returning raw text, with no structured-output constraint.
    pub fn chat(&self, system: &str, user: &str, label: &str) -> Result<String, String> {
        self.chat_fmt(system, user, None, label, false)
    }

    // Chat with an optional `response_format`. Retries transient transport failures (gateway/5xx,
    // dropped connections) immediately, with no backoff.
    fn chat_fmt(&self, system: &str, user: &str, rf: Option<&Value>, label: &str, stream: bool) -> Result<String, String> {
        let max = max_retries();
        let mut last = String::new();
        let started = std::time::Instant::now();
        if verbose() {
            eprintln!("[jazyk] → {}", label);
        }
        for attempt in 0..=max {
            match self.chat_once(system, user, rf, stream) {
                Ok(s) => {
                    if verbose() {
                        eprintln!("[jazyk] ✓ {} ({} ms)", label, started.elapsed().as_millis());
                    }
                    return Ok(s);
                }
                Err(e) => {
                    last = e;
                    if attempt < max && is_transient(&last) {
                        eprintln!("[jazyk] {} — transient error, retrying ({}/{}): {}", label, attempt + 1, max, truncate(&last, 120));
                    } else {
                        break;
                    }
                }
            }
        }
        if verbose() {
            eprintln!("[jazyk] ✗ {} ({} ms): {}", label, started.elapsed().as_millis(), truncate(&last, 120));
        }
        Err(last)
    }

    // One logical attempt. Drops a parameter the model rejects and retries once: `response_format`
    // first (fall back to prompt-only JSON), then a non-default temperature. The recursion covers
    // any combination of the two.
    fn chat_once(&self, system: &str, user: &str, rf: Option<&Value>, stream: bool) -> Result<String, String> {
        self.attempt(system, user, self.temperature, rf, stream)
    }

    fn attempt(&self, system: &str, user: &str, temperature: Option<f64>, rf: Option<&Value>, stream: bool) -> Result<String, String> {
        match self.chat_inner(system, user, temperature, rf, stream) {
            Err(e) if rf.is_some() && {
                let le = e.to_lowercase();
                le.contains("response_format") || le.contains("response format") || le.contains("json_schema")
            } =>
            {
                if !STRUCTURED_UNSUPPORTED.swap(true, Ordering::Relaxed) {
                    eprintln!("[jazyk] endpoint rejected response_format; falling back to prompt-only JSON for this run");
                }
                self.attempt(system, user, temperature, None, stream)
            }
            Err(e) if temperature.is_some() && e.to_lowercase().contains("temperature") => {
                eprintln!("[jazyk] model rejected temperature; retrying without it");
                self.attempt(system, user, None, rf, stream)
            }
            other => other,
        }
    }

    fn chat_inner(&self, system: &str, user: &str, temperature: Option<f64>, rf: Option<&Value>, stream: bool) -> Result<String, String> {
        let mut payload = json!({
            "model": self.model,
            "stream": false,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        });
        if stream {
            // Stream Server-Sent Events and ask for a final usage chunk, so the meter gets exact
            // completion tokens alongside the per-token timing.
            payload["stream"] = json!(true);
            payload["stream_options"] = json!({"include_usage": true});
        }
        if let Some(t) = temperature {
            payload["temperature"] = json!(t);
        }
        if let Some(rf) = rf {
            // Skip if a prior call already learned this endpoint rejects structured output.
            if !STRUCTURED_UNSUPPORTED.load(Ordering::Relaxed) {
                payload["response_format"] = rf.clone();
            }
        }
        let body = payload.to_string();

        // Bound concurrent requests across all worker threads.
        let _permit = acquire();

        let (host, port, base_path) = parse_url(&self.base_url)?;
        let path = format!("{}/chat/completions", base_path);
        let addr = format!("{}:{}", host, port);
        let mut conn = TcpStream::connect(&addr).map_err(|e| format!("connect {}: {}", addr, e))?;
        conn.set_read_timeout(Some(Duration::from_secs(900))).ok();
        conn.set_write_timeout(Some(Duration::from_secs(60))).ok();
        let auth = if self.api_key.is_empty() {
            String::new()
        } else {
            format!("Authorization: Bearer {}\r\n", self.api_key)
        };
        let req = format!(
            "POST {} HTTP/1.0\r\nHost: {}\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            path, host, auth, body.as_bytes().len(), body
        );
        // Time the generation from the request being sent. Excludes the concurrency-permit wait above,
        // so the meter reflects the server's response time, not local queueing.
        let started = Instant::now();
        conn.write_all(req.as_bytes()).map_err(|e| format!("write: {}", e))?;

        if stream {
            return read_stream(&mut conn, started);
        }

        let mut buf = Vec::new();
        conn.read_to_end(&mut buf).map_err(|e| format!("read: {}", e))?;
        let text = String::from_utf8_lossy(&buf).to_string();
        let sep = text.find("\r\n\r\n").ok_or("no http body separator")?;
        let head = &text[..sep];
        let resp_body = &text[sep + 4..];
        let ok = head.lines().next().map(|l| l.contains(" 200")).unwrap_or(false);
        if !ok {
            return Err(format!("http error: {} :: {}", head.lines().next().unwrap_or(""), truncate(resp_body, 300)));
        }
        let v: Value = serde_json::from_str(resp_body)
            .map_err(|e| format!("response json: {} :: {}", e, truncate(resp_body, 300)))?;
        let content = v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
        // Record throughput: prefer the endpoint's reported completion tokens, else estimate from the
        // response length (~4 chars per token). Same unit as the benchmark target, so either is usable.
        let tokens = v["usage"]["completion_tokens"]
            .as_u64()
            .unwrap_or_else(|| (content.chars().count() as u64).div_ceil(4));
        record_generation(tokens, started.elapsed());
        Ok(content)
    }
}

// Read a streamed (SSE) chat completion, separating time to first token from the decode window.
// Parses `data:` lines as they arrive: timestamps the first content delta (TTFT) and the last
// (decode end), accumulates the content, and prefers the final `usage` chunk for the token count
// (falling back to the streamed delta count). Tolerant of HTTP chunked framing, since non-`data:`
// lines (chunk-size lines, blanks) are skipped. Falls back to a whole-body JSON parse if the server
// ignored `stream: true` and returned one object. Records into the streaming meter on success.
fn read_stream(conn: &mut TcpStream, req_start: Instant) -> Result<String, String> {
    let mut raw: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    let mut headers_done = false;
    let mut head_line = String::new();
    let mut body_all = String::new(); // full SSE body, for the non-streamed fallback
    let mut pending = String::new(); // unprocessed tail (partial line)
    let mut content = String::new();
    let mut first: Option<Instant> = None;
    let mut last = req_start;
    let mut delta_tokens: u64 = 0;
    let mut usage_tokens: Option<u64> = None;
    let mut done = false;

    loop {
        let n = match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(format!("read: {}", e)),
        };
        if !headers_done {
            raw.extend_from_slice(&buf[..n]);
            let Some(pos) = find_subslice(&raw, b"\r\n\r\n") else { continue };
            let head = String::from_utf8_lossy(&raw[..pos]).to_string();
            head_line = head.lines().next().unwrap_or("").to_string();
            if !head_line.contains(" 200") {
                // Error response (e.g. rejected response_format): drain and surface the body so
                // `attempt` can react to it the same way the non-streamed path does.
                let mut rest = raw[pos + 4..].to_vec();
                while let Ok(m) = conn.read(&mut buf) {
                    if m == 0 {
                        break;
                    }
                    rest.extend_from_slice(&buf[..m]);
                }
                let body = String::from_utf8_lossy(&rest);
                return Err(format!("http error: {} :: {}", head_line, truncate(&body, 300)));
            }
            headers_done = true;
            let body_bytes = raw[pos + 4..].to_vec();
            let chunk = String::from_utf8_lossy(&body_bytes).to_string();
            body_all.push_str(&chunk);
            pending.push_str(&chunk);
        } else {
            let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
            body_all.push_str(&chunk);
            pending.push_str(&chunk);
        }

        // Process complete lines; leave any partial line in `pending`.
        while let Some(nl) = pending.find('\n') {
            let line = pending[..nl].trim_end_matches('\r').to_string();
            pending.drain(..nl + 1);
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                done = true;
                break;
            }
            let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
            if let Some(u) = v["usage"]["completion_tokens"].as_u64() {
                usage_tokens = Some(u);
            }
            if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                if !delta.is_empty() {
                    if first.is_none() {
                        first = Some(Instant::now());
                    }
                    last = Instant::now();
                    content.push_str(delta);
                    delta_tokens += 1;
                }
            }
        }
        if done {
            break;
        }
    }

    // Fallback: the server ignored `stream: true` and returned a single JSON object.
    if content.is_empty() && delta_tokens == 0 {
        if let Some(obj) = extract_json_object(&body_all) {
            if let Ok(v) = serde_json::from_str::<Value>(&obj) {
                content = v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                usage_tokens = v["usage"]["completion_tokens"].as_u64().or(usage_tokens);
                first.get_or_insert(last);
            }
        }
        if content.is_empty() {
            let detail = if head_line.is_empty() { truncate(&body_all, 200) } else { head_line };
            return Err(format!("empty stream response :: {}", detail));
        }
    }

    let first_t = first.unwrap_or(last);
    let ttft = first_t.saturating_duration_since(req_start);
    let decode = last.saturating_duration_since(first_t);
    let tokens = usage_tokens.unwrap_or(delta_tokens);
    record_generation_stream(tokens, ttft, decode);
    Ok(content)
}

// Index of the first occurrence of `needle` in `hay`.
fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

fn parse_url(u: &str) -> Result<(String, u16, String), String> {
    let rest = u.strip_prefix("http://").or_else(|| u.strip_prefix("https://")).unwrap_or(u);
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let (host, port) = match hostport.find(':') {
        Some(i) => (hostport[..i].to_string(), hostport[i + 1..].parse::<u16>().unwrap_or(80)),
        None => (hostport.to_string(), 80),
    };
    Ok((host, port, path.trim_end_matches('/').to_string()))
}

// Extract the first balanced JSON object from possibly noisy model output.
pub fn extract_json_object(s: &str) -> Option<String> {
    let mut s = s.to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if a < b {
            s.replace_range(a..b + "</think>".len(), "");
        } else {
            break;
        }
    }
    let bytes = s.as_bytes();
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else {
            match c {
                '"' => in_str = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(s[start..=i].to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

// Extract the first <svg ...>...</svg> block from possibly noisy model output.
pub fn extract_svg(s: &str) -> Option<String> {
    let start = s.find("<svg")?;
    let end = s[start..].find("</svg>")? + start + "</svg>".len();
    Some(s[start..end].to_string())
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}
