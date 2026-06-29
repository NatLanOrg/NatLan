// Benchmark: tests whether the configured model is good enough to compile Jazyk.
// Mirrors docs/benchmark/. It exercises the LLM-invoking stages (A2, A3, A4, L4, L5)
// with representative cases, runs a structural schema gate + field-targeted assertions,
// and reports a per-stage score, an overall score, and a usable/not-usable verdict.
use crate::llm::Llm;
use serde_json::{json, Value};

// Enum domains from docs/compiler/model.schema.yaml. Matched case-insensitively.
const ROLES: &[&str] = &["definition", "reference"];
const RELATIONSHIP_TYPES: &[&str] = &[
    "generalization",
    "realization",
    "composition",
    "aggregation",
    "association",
    "dependency",
    "reference",
];
const SEVERITIES: &[&str] = &["error", "warning", "info", "none"];

// A structural schema validator: returns Ok on conformance, Err(path/value reason) otherwise.
type SchemaFn = fn(&Value) -> Result<(), String>;

// A field-targeted assertion against the parsed output.
#[allow(dead_code)] // `Each` is a documented primitive (checks.md), kept available for future cases.
enum Assert {
    /// Top-level field equals `value` (case-insensitive, trimmed). Works for strings and bools.
    Eq { field: &'static str, value: &'static str },
    /// Top-level string field contains `needle` (case-insensitive).
    Contains { field: &'static str, needle: &'static str },
    /// Some element of `array` has string field `field` containing `needle`.
    Any { array: &'static str, field: &'static str, needle: &'static str },
    /// Every element of `array` has string field `field` containing `needle`.
    Each { array: &'static str, field: &'static str, needle: &'static str },
}

struct Case {
    name: &'static str,
    stage: &'static str,
    system: &'static str,
    user: &'static str,
    // JSON schema sent as a structured-output constraint (response_format), exactly as the
    // compiler does. A conforming endpoint is steered to the right shape and enums at generation.
    json_schema: Value,
    // Post-parse structural gate: re-checks shape/enums (and EARS shape), catching endpoints that
    // ignore response_format and fall back to prompt-only JSON.
    validate: SchemaFn,
    asserts: &'static [Assert],
}

// JSON schemas per stage, mirroring the shapes the compiler requests in compile.rs / link.rs.
fn entities_schema() -> Value {
    json!({
        "type": "object",
        "required": ["entities"],
        "properties": {
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["name", "role", "definition"],
                    "properties": {
                        "name": {"type": "string"},
                        "role": {"type": "string", "enum": ROLES},
                        "definition": {"type": "string"}
                    }
                }
            }
        }
    })
}

fn requirements_schema() -> Value {
    json!({
        "type": "object",
        "required": ["requirements"],
        "properties": {
            "requirements": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["ears", "entities"],
                    "properties": {
                        "ears": {"type": "string"},
                        "entities": {"type": "array", "items": {"type": "string"}}
                    }
                }
            }
        }
    })
}

fn relationship_schema() -> Value {
    json!({
        "type": "object",
        "required": ["type", "reasoning"],
        "properties": {
            "type": {"type": "string", "enum": RELATIONSHIP_TYPES},
            "reasoning": {"type": "string"}
        }
    })
}

fn definition_schema() -> Value {
    json!({
        "type": "object",
        "required": ["coherent", "definition"],
        "properties": {
            "coherent": {"type": "boolean"},
            "definition": {"type": "string"}
        }
    })
}

fn diagnostics_schema() -> Value {
    json!({
        "type": "object",
        "required": ["diagnostics"],
        "properties": {
            "diagnostics": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["rule", "severity", "message"],
                    "properties": {
                        "rule": {"type": "string"},
                        "severity": {"type": "string", "enum": SEVERITIES},
                        "message": {"type": "string"}
                    }
                }
            }
        }
    })
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "a2-entities-basic",
            stage: "A2",
            system: "Extract domain entities. Return ONLY JSON: {\"entities\":[{\"name\":string,\"role\":\"definition\"|\"reference\",\"definition\":string}]}.",
            user: "# Cart\n\nA Cart holds the Products a Customer intends to buy. Each Cart belongs to one Customer. A Product has a price and a name.",
            json_schema: entities_schema(),
            validate: schema_entities,
            asserts: &[
                Assert::Any { array: "entities", field: "name", needle: "Cart" },
                Assert::Any { array: "entities", field: "name", needle: "Product" },
                Assert::Any { array: "entities", field: "name", needle: "Customer" },
            ],
        },
        Case {
            name: "a3-requirements-ears",
            stage: "A3",
            system: "Extract EARS requirements. Each 'ears' must be a full EARS sentence containing the word 'shall'. Return ONLY JSON: {\"requirements\":[{\"ears\":string,\"entities\":[string]}]}.",
            user: "Entities: User, Email\n\nEach User must have a unique Email. When a User registers, the system shall send a confirmation Email.",
            json_schema: requirements_schema(),
            validate: schema_requirements,
            // The EARS shape (every 'ears' contains 'shall') is gated by `validate`, since the
            // structured-output schema cannot enforce content. The assertions check recall of both
            // input requirements: the uniqueness constraint and the registration/confirmation event.
            asserts: &[
                Assert::Any { array: "requirements", field: "ears", needle: "unique" },
                Assert::Any { array: "requirements", field: "ears", needle: "confirmation" },
            ],
        },
        Case {
            name: "a4-relationship-type",
            stage: "A4",
            system: "Pick the single strongest UML relationship type for the pair given the requirements. Types: generalization, realization, composition, aggregation, association, dependency, reference. Return ONLY JSON: {\"type\":string,\"reasoning\":string}.",
            user: "Pair: Cart, Product\nRequirements:\n- A Cart holds the Products a Customer intends to buy.\n- Removing a Cart removes its Products.",
            json_schema: relationship_schema(),
            validate: schema_relationship,
            // The chosen type must be composition, not merely mentioned in the reasoning.
            asserts: &[Assert::Eq { field: "type", value: "composition" }],
        },
        Case {
            name: "l4-synthesize-definition",
            stage: "L4",
            system: "Decide if the definitions describe one coherent thing and synthesize one. Return ONLY JSON: {\"coherent\":true|false,\"definition\":string}.",
            user: "Entity: Customer\nDefinitions:\n- (sales.md): a person or organization that holds an account\n- (orders.md): the buyer who places an order",
            json_schema: definition_schema(),
            validate: schema_definition,
            // A coherent merge, retaining a key concept from the inputs ("account").
            asserts: &[
                Assert::Eq { field: "coherent", value: "true" },
                Assert::Contains { field: "definition", needle: "account" },
            ],
        },
        Case {
            name: "l5-contradiction",
            stage: "L5",
            system: "Review an entity's requirements gathered across documents and find real problems. Severity must be one of: error, warning, info, none. Return ONLY JSON: {\"diagnostics\":[{\"rule\":string,\"severity\":string,\"message\":string}]}.",
            user: "Entity: ABC\nRequirements:\n- ABC is a tricycle with exactly 3 wheels.\n- ABC is a four-wheeled car.",
            json_schema: diagnostics_schema(),
            validate: schema_diagnostics,
            // The contradiction must be flagged and name the conflicting concept.
            asserts: &[Assert::Any { array: "diagnostics", field: "message", needle: "wheel" }],
        },
    ]
}

// ---- structural schema validators ----------------------------------------------------------

fn arr<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    match v.get(key) {
        Some(Value::Array(a)) if !a.is_empty() => Ok(a),
        Some(Value::Array(_)) => Err(format!("'{}' is an empty array", key)),
        Some(_) => Err(format!("'{}' is not an array", key)),
        None => Err(format!("missing key '{}'", key)),
    }
}

// A field that must be a non-empty (after trim) string.
fn nonempty_str(item: &Value, path: &str, field: &str) -> Result<String, String> {
    match item.get(field) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        Some(Value::String(_)) => Err(format!("{}.{} is empty", path, field)),
        Some(_) => Err(format!("{}.{} is not a string", path, field)),
        None => Err(format!("{}.{} missing", path, field)),
    }
}

// A field that must be a string from `domain` (case-insensitive).
fn enum_str(item: &Value, path: &str, field: &str, domain: &[&str]) -> Result<(), String> {
    let s = nonempty_str(item, path, field)?;
    if domain.iter().any(|d| d.eq_ignore_ascii_case(s.trim())) {
        Ok(())
    } else {
        Err(format!("{}.{} '{}' not in {:?}", path, field, s.trim(), domain))
    }
}

fn schema_entities(v: &Value) -> Result<(), String> {
    let items = arr(v, "entities")?;
    for (i, e) in items.iter().enumerate() {
        let p = format!("entities[{}]", i);
        nonempty_str(e, &p, "name")?;
        enum_str(e, &p, "role", ROLES)?;
        // A `definition`-role entity must carry a non-empty definition. A `reference`-role
        // entity is only mentioned here, so its `definition` may be empty or absent; if present
        // it must still be a string.
        let role = e.get("role").and_then(|r| r.as_str()).unwrap_or("").trim().to_lowercase();
        if role == "definition" {
            nonempty_str(e, &p, "definition")?;
        } else if let Some(d) = e.get("definition") {
            if !d.is_string() {
                return Err(format!("{}.definition is not a string", p));
            }
        }
    }
    Ok(())
}

fn schema_requirements(v: &Value) -> Result<(), String> {
    let items = arr(v, "requirements")?;
    for (i, r) in items.iter().enumerate() {
        let p = format!("requirements[{}]", i);
        let ears = nonempty_str(r, &p, "ears")?;
        // EARS shape: every requirement carries the mandatory 'shall'.
        if !ears.to_lowercase().contains("shall") {
            return Err(format!("{}.ears missing 'shall' (not EARS-shaped)", p));
        }
        match r.get("entities") {
            Some(Value::Array(a)) if !a.is_empty() && a.iter().all(|x| x.is_string()) => {}
            Some(Value::Array(_)) => return Err(format!("{}.entities empty or non-string", p)),
            Some(_) => return Err(format!("{}.entities is not an array", p)),
            None => return Err(format!("{}.entities missing", p)),
        }
    }
    Ok(())
}

fn schema_relationship(v: &Value) -> Result<(), String> {
    enum_str(v, "(root)", "type", RELATIONSHIP_TYPES)?;
    nonempty_str(v, "(root)", "reasoning")?;
    Ok(())
}

fn schema_definition(v: &Value) -> Result<(), String> {
    match v.get("coherent") {
        Some(Value::Bool(_)) => {}
        Some(_) => return Err("(root).coherent is not a boolean".into()),
        None => return Err("missing key 'coherent'".into()),
    }
    nonempty_str(v, "(root)", "definition")?;
    Ok(())
}

fn schema_diagnostics(v: &Value) -> Result<(), String> {
    let items = arr(v, "diagnostics")?;
    for (i, d) in items.iter().enumerate() {
        let p = format!("diagnostics[{}]", i);
        nonempty_str(d, &p, "rule")?;
        nonempty_str(d, &p, "message")?;
        enum_str(d, &p, "severity", SEVERITIES)?;
    }
    Ok(())
}

// ---- assertion evaluation ------------------------------------------------------------------

// Render a JSON value's text for substring/equality checks: strings as-is, bools/numbers stringified.
fn as_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn field_contains(item: &Value, field: &str, needle: &str) -> bool {
    item.get(field)
        .and_then(as_text)
        .map(|s| s.to_lowercase().contains(&needle.to_lowercase()))
        .unwrap_or(false)
}

impl Assert {
    fn describe(&self) -> String {
        match self {
            Assert::Eq { field, value } => format!("{} == '{}'", field, value),
            Assert::Contains { field, needle } => format!("{} contains '{}'", field, needle),
            Assert::Any { array, field, needle } => {
                format!("some {}.{} contains '{}'", array, field, needle)
            }
            Assert::Each { array, field, needle } => {
                format!("every {}.{} contains '{}'", array, field, needle)
            }
        }
    }

    fn eval(&self, v: &Value) -> bool {
        match self {
            Assert::Eq { field, value } => v
                .get(field)
                .and_then(as_text)
                .map(|s| s.trim().eq_ignore_ascii_case(value.trim()))
                .unwrap_or(false),
            Assert::Contains { field, needle } => field_contains(v, field, needle),
            Assert::Any { array, field, needle } => match v.get(array) {
                Some(Value::Array(a)) => a.iter().any(|x| field_contains(x, field, needle)),
                _ => false,
            },
            Assert::Each { array, field, needle } => match v.get(array) {
                Some(Value::Array(a)) if !a.is_empty() => {
                    a.iter().all(|x| field_contains(x, field, needle))
                }
                _ => false,
            },
        }
    }
}

// ---- run -----------------------------------------------------------------------------------

// Speed thresholds (see docs/benchmark/scoring.md#throughput). Output speed (decode rate) gates the
// verdict: the target scores 1, the floor is the minimum below which the model is too slow to compile
// with. Time to first token is reported and scored against its target but does not gate, since a slow
// start with a fast decode is still workable.
const SPEED_TARGET_TPS: f64 = 20.0;
const SPEED_FLOOR_TPS: f64 = 5.0;
const TTFT_TARGET_SECS: f64 = 2.0;

pub fn run(llm: &Llm) -> i32 {
    eprintln!("jazyk benchmark: model {} at {}", llm.model, llm.base_url);
    eprintln!("grading the LLM stages used to compile Jazyk (A2, A3, A4, L4, L5).\n");
    let cases = cases();
    let mut stage_scores: std::collections::BTreeMap<&str, Vec<f64>> = std::collections::BTreeMap::new();
    let mut failed: Vec<String> = Vec::new();

    // Measure generation throughput across all cases: reset the meter, run, then read the snapshot.
    crate::llm::throughput_reset();
    for c in &cases {
        let score = score_case(llm, c, &mut failed);
        stage_scores.entry(c.stage).or_default().push(score);
    }
    let tp = crate::llm::throughput_snapshot();
    eprintln!();

    let stage_floor = 0.5;
    let overall_threshold = 0.6;
    let mut all_stages_pass = true;
    let mut overall_parts = Vec::new();
    eprintln!("\nstage scores:");
    for (stage, scores) in &stage_scores {
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        overall_parts.push(mean);
        let pass = mean >= stage_floor;
        if !pass {
            all_stages_pass = false;
        }
        eprintln!("  {:<4} {:.2} {}", stage, mean, if pass { "ok" } else { "BELOW FLOOR" });
    }
    let overall = if overall_parts.is_empty() {
        0.0
    } else {
        overall_parts.iter().sum::<f64>() / overall_parts.len() as f64
    };

    // Speed: split time to first token from the decode (output) speed, measured by streaming. Output
    // speed gates the verdict against its floor; TTFT is reported and scored but does not gate.
    let output_speed = if tp.decode_secs > 0.0 { tp.tokens as f64 / tp.decode_secs } else { 0.0 };
    let ttft_avg = if tp.stream_calls > 0 { tp.ttft_secs / tp.stream_calls as f64 } else { 0.0 };
    let effective = if tp.total_secs > 0.0 { tp.tokens as f64 / tp.total_secs } else { 0.0 };
    let speed_score = (output_speed / SPEED_TARGET_TPS).min(1.0);
    let speed_pass = output_speed >= SPEED_FLOOR_TPS;
    let ttft_score = if ttft_avg > 0.0 { (TTFT_TARGET_SECS / ttft_avg).min(1.0) } else { 1.0 };
    eprintln!("\nspeed (streamed over {} calls):", tp.stream_calls);
    eprintln!(
        "  time to first token : {:.2}s avg — ttft score {:.2} (target {:.1}s)",
        ttft_avg, ttft_score, TTFT_TARGET_SECS
    );
    eprintln!(
        "  output speed        : {:.1} tok/s decode ({} tokens / {:.1}s) — speed score {:.2} (target {:.0} tok/s) {}",
        output_speed,
        tp.tokens,
        tp.decode_secs,
        speed_score,
        SPEED_TARGET_TPS,
        if speed_pass { "ok" } else { "BELOW FLOOR" }
    );
    eprintln!("  effective           : {:.1} tok/s end-to-end (TTFT + decode)", effective);

    let usable = all_stages_pass && speed_pass && overall >= overall_threshold;
    eprintln!("\noverall score: {:.2} (threshold {:.2})", overall, overall_threshold);
    if !failed.is_empty() {
        eprintln!("failed checks:");
        for f in &failed {
            eprintln!("  - {}", f);
        }
    }
    if !speed_pass {
        eprintln!("  - output speed {:.1} tok/s below floor {:.0} tok/s", output_speed, SPEED_FLOOR_TPS);
    }
    eprintln!("verdict: {}", if usable { "USABLE" } else { "NOT USABLE" });
    if usable {
        0
    } else {
        1
    }
}

fn score_case(llm: &Llm, c: &Case, failed: &mut Vec<String>) -> f64 {
    eprintln!("[{}] {}", c.stage, c.name);
    eprintln!("  prompt : {}", oneline(c.system, 140));
    eprintln!("  input  : {}", oneline(c.user, 200));

    // Schema-constrained call, exactly as the compiler issues it (response_format: json_schema).
    // chat_json handles JSON extraction, parsing, and retries; an endpoint that rejects the
    // constraint falls back to prompt-only JSON and is still held to the structural gate below.
    let v = match llm.chat_json_stream(c.system, c.user, c.stage, &c.json_schema, &format!("benchmark · {}", c.stage)) {
        Ok(v) => {
            eprintln!("  output : {}", oneline(&serde_json::to_string(&v).unwrap_or_default(), 400));
            v
        }
        Err(e) => {
            eprintln!("  schema : FAIL — no conforming JSON: {} (gate)\n", oneline(&e, 200));
            failed.push(format!("{}: no conforming JSON: {}", c.name, oneline(&e, 160)));
            return 0.0;
        }
    };

    // Structural schema gate: shape, required fields, enums, EARS shape.
    if let Err(reason) = (c.validate)(&v) {
        eprintln!("  schema : FAIL — {} (gate)\n", reason);
        failed.push(format!("{}: schema: {}", c.name, reason));
        return 0.0;
    }
    eprintln!("  schema : ok — structure conforms");

    // Field-targeted assertions.
    let mut hits = 0usize;
    for a in c.asserts {
        if a.eval(&v) {
            eprintln!("  assert : ok   {}", a.describe());
            hits += 1;
        } else {
            eprintln!("  assert : FAIL {}", a.describe());
            failed.push(format!("{}: {}", c.name, a.describe()));
        }
    }
    let score = if c.asserts.is_empty() {
        1.0
    } else {
        hits as f64 / c.asserts.len() as f64
    };
    eprintln!("  score  : {:.2}\n", score);
    score
}

// Collapse whitespace/newlines and truncate, for readable single-line logging.
fn oneline(s: &str, n: usize) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > n {
        format!("{}…", flat.chars().take(n).collect::<String>())
    } else {
        flat
    }
}
