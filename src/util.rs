use serde_json::Value;

// a helper function for parsing u64 fields
#[allow(dead_code)]
pub fn parse_u64_field(value: &serde_json::value::Value, key: &str) -> u64 {
    let maybe_num = match &value[key] {
        Value::Number(maybe_num) => {
            maybe_num.as_u64()
        },
        _ => panic!(format!("Applied parse_u64_field on non-number field: {}", key)),
    };

    if let Some(num) = maybe_num {
        num
    } else {
        panic!(format!("Casting serde_json::Value::Number as u64 failed: {}", key));
    }
}

// a helper function for parsing i64 fields
#[allow(dead_code)]
pub fn parse_i64_field(value: &serde_json::value::Value, key: &str) -> i64 {
    let maybe_num = match &value[key] {
        Value::Number(maybe_num) => {
            maybe_num.as_i64()
        },
        _ => panic!(format!("Applied parse_i64_field on non-number field: {}", key)),
    };

    if let Some(num) = maybe_num {
        num
    } else {
        panic!(format!("Casting serde_json::Value::Number as i64 failed: {}", key));
    }
}

// a helper function for parsing float fields
#[allow(dead_code)]
pub fn parse_f64_field(value: &serde_json::value::Value, key: &str) -> f64 {
    let maybe_num = match &value[key] {
        Value::Number(maybe_num) => maybe_num.as_f64(),
        _ => panic!(format!("Applied parse_f64_field on non-number field: {}", key)),
    };

    if let Some(num) = maybe_num {
        num
    } else {
        panic!(format!("Applied parse_f64_field on non-number field: {}", key))
    }
}

// a helper function for parsing String fields
#[allow(dead_code)]
pub fn parse_string_field(value: &serde_json::value::Value, key: &str) -> String {
    match &value[key] {
        Value::String(string) => string.to_string(),
        _ => panic!(format!("Applied parse_string_field on non-String field: {}", key)),
    }
}