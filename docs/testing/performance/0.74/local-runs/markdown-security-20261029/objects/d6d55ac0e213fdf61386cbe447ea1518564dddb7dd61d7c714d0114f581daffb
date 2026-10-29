//! Fixture-only semantic probe, compiled against exactly one parser per build.
//! Its Cargo manifests/locks live under target, outside the shipping workspace.
use parser::Value;
use serde_json::{json, Value as Json};
use std::error::Error;
use std::io::{self, Read};

fn node(value: &Value) -> Json {
    let rendered = match parser::to_string(value) {
        Ok(text) => json!({"ok": true, "text": text}),
        Err(error) => json!({"ok": false, "message": error.to_string()}),
    };
    let shape = match value {
        Value::Null => json!({"kind": "null"}),
        Value::Bool(value) => json!({"kind": "bool", "value": value}),
        Value::Number(value) => json!({
            "kind": "number",
            "i64": value.as_i64().map(|v| v.to_string()),
            "u64": value.as_u64().map(|v| v.to_string()),
            "f64_bits": value.as_f64().map(|v| format!("{:016x}", v.to_bits())),
            "display": value.to_string(),
        }),
        Value::String(value) => json!({"kind": "string", "value": value}),
        Value::Sequence(values) => {
            json!({"kind": "sequence", "items": values.iter().map(node).collect::<Vec<_>>()})
        }
        Value::Mapping(values) => {
            json!({"kind": "mapping", "entries": values.iter().map(|(key,value)| json!([node(key),node(value)])).collect::<Vec<_>>()})
        }
        Value::Tagged(value) => {
            json!({"kind": "tagged", "tag": value.tag.to_string(), "value": node(&value.value)})
        }
    };
    json!({"shape": shape, "rendered": rendered})
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("semantic envelope exceeds the fixed review limit".into());
    }
    let envelope: Json = serde_json::from_slice(&bytes)?;
    let id = envelope["id"].as_str().ok_or("missing case id")?;
    let text = envelope["text"].as_str().ok_or("missing YAML text")?;
    let result = match parser::from_str::<Value>(text) {
        Ok(value) => {
            let tree = node(&value);
            let roundtrip = match parser::to_string(&value) {
                Ok(rendered) => match parser::from_str::<Value>(&rendered) {
                    Ok(reparsed) => json!({"ok": true, "tree": node(&reparsed)}),
                    Err(error) => json!({"ok": false, "message": error.to_string()}),
                },
                Err(error) => json!({"ok": false, "message": error.to_string()}),
            };
            json!({"id": id, "parsed": true, "tree": tree, "roundtrip": roundtrip})
        }
        Err(error) => {
            let location = error
                .location()
                .map(|v| json!({"line": v.line(), "column": v.column(), "index": v.index()}));
            json!({"id": id, "parsed": false, "message": error.to_string(), "location": location})
        }
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
