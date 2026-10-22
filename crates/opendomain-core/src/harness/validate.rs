//! Vérificateur statique embarqué : sous-ensemble de JSON Schema.
//!
//! Utilisé par les gates « `y_i ∈ C_i` » sans dépendance lourde :
//! `type`, `required`, `properties`, `items`, `additionalProperties`,
//! `minLength`, `maxLength`. Suffisant pour les contrats d'agents.

use serde_json::Value;

/// Valide `value` contre `schema` ; renvoie la liste des écarts en cas d'échec.
pub fn validate_output(value: &Value, schema: &Value) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    check(value, schema, "$", &mut errs);
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn check(value: &Value, schema: &Value, path: &str, errs: &mut Vec<String>) {
    let Some(s) = schema.as_object() else {
        return;
    };

    if let Some(t) = s.get("type").and_then(Value::as_str) {
        let ok = match t {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "null" => value.is_null(),
            _ => true,
        };
        if !ok {
            errs.push(format!(
                "{path} : attendu `{t}`, obtenu `{}`",
                json_type(value)
            ));
            return;
        }
    }

    if let Some(req) = s.get("required").and_then(Value::as_array) {
        if let Some(obj) = value.as_object() {
            for r in req {
                if let Some(name) = r.as_str() {
                    if !obj.contains_key(name) {
                        errs.push(format!("{path} : champ requis `{name}` manquant"));
                    }
                }
            }
        }
    }

    let props = s.get("properties").and_then(Value::as_object);
    if let Some(obj) = value.as_object() {
        if let Some(props) = props {
            for (k, sub) in props {
                if let Some(v) = obj.get(k) {
                    check(v, sub, &format!("{path}.{k}"), errs);
                }
            }
        }
        if s.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            for k in obj.keys() {
                let declared = props.map(|p| p.contains_key(k)).unwrap_or(false);
                if !declared {
                    errs.push(format!("{path} : champ supplémentaire `{k}` interdit"));
                }
            }
        }
    }

    if let Some(items) = s.get("items") {
        if let Some(arr) = value.as_array() {
            for (i, v) in arr.iter().enumerate() {
                check(v, items, &format!("{path}[{i}]"), errs);
            }
        }
    }

    if let Some(min) = s.get("minLength").and_then(Value::as_u64) {
        if let Some(txt) = value.as_str() {
            if (txt.chars().count() as u64) < min {
                errs.push(format!("{path} : longueur < minLength ({min})"));
            }
        }
    }

    if let Some(min) = s.get("minItems").and_then(Value::as_u64) {
        if let Some(arr) = value.as_array() {
            if (arr.len() as u64) < min {
                errs.push(format!(
                    "{path} : {n} élément(s) < minItems ({min})",
                    n = arr.len()
                ));
            }
        }
    }

    if let Some(max) = s.get("maxLength").and_then(Value::as_u64) {
        if let Some(txt) = value.as_str() {
            if (txt.chars().count() as u64) > max {
                errs.push(format!("{path} : longueur > maxLength ({max})"));
            }
        }
    }
}

fn json_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
