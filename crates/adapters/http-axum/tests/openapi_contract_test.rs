fn spec() -> serde_json::Value {
    serde_json::to_value(http_axum::openapi::spec()).unwrap()
}

fn operations(spec: &serde_json::Value) -> Vec<(String, String, serde_json::Value)> {
    spec["paths"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(path, methods)| {
            methods
                .as_object()
                .unwrap()
                .iter()
                .map(move |(verb, op)| (verb.to_uppercase(), path.clone(), op.clone()))
        })
        .collect()
}

#[test]
fn the_spec_is_not_vacuous() {
    let spec = spec();
    assert!(!operations(&spec).is_empty());
    assert!(spec["components"]["schemas"]["ErrorResponse"].is_object());
}

#[test]
fn every_2xx_json_response_declares_a_schema() {
    let spec = spec();
    let mut untyped = Vec::new();

    for (verb, path, op) in operations(&spec) {
        for (code, r) in op["responses"].as_object().unwrap() {
            if !code.starts_with('2') {
                continue;
            }
            if let Some(json) = r["content"].get("application/json")
                && !json["schema"].is_object()
            {
                untyped.push(format!("{verb} {path} -> {code}"));
            }
        }
    }

    assert!(untyped.is_empty(), "2xx JSON without a schema: {untyped:?}");
}

#[test]
fn every_operation_documents_a_refusal() {
    let spec = spec();
    let silent: Vec<_> = operations(&spec)
        .into_iter()
        .filter(|(_, _, op)| {
            !op["responses"]
                .as_object()
                .unwrap()
                .keys()
                .any(|c| c.starts_with('4'))
        })
        .map(|(v, p, _)| format!("{v} {p}"))
        .collect();
    assert!(silent.is_empty(), "no 4xx documented: {silent:?}");
}

#[test]
fn every_authenticated_operation_documents_401() {
    let spec = spec();
    let silent: Vec<_> = operations(&spec)
        .into_iter()
        .filter(|(_, _, op)| op.get("security").is_some() && op["responses"].get("401").is_none())
        .map(|(v, p, _)| format!("{v} {p}"))
        .collect();
    assert!(silent.is_empty(), "no 401 documented: {silent:?}");
}

#[test]
fn every_refusal_is_the_one_error_shape() {
    let spec = spec();
    let mut odd = Vec::new();
    for (verb, path, op) in operations(&spec) {
        for (code, response) in op["responses"].as_object().unwrap() {
            if !code.starts_with('4') && !code.starts_with('5') {
                continue;
            }
            let named = response["content"]["application/json"]["schema"]["$ref"]
                .as_str()
                .unwrap_or_default();
            if named != "#/components/schemas/ErrorResponse" {
                odd.push(format!("{verb} {path} -> {code}"));
            }
        }
    }
    assert!(odd.is_empty(), "refusals not ErrorResponse: {odd:?}");
}
