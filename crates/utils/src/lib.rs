pub const PARSE_QUERY_TRUE: &str = "";

/**
 * Convert query to string
 * # Examples: vec![("a".to_string(), "1".to_string()), ("b".to_string(), "".to_string())] => "?a=1&b"
 */
pub fn stringify_query(query: &Vec<(String, String)>) -> String {
    if query.is_empty() {
        return String::new();
    }

    let mut qs = vec![];

    for (k, v) in query {
        if v == PARSE_QUERY_TRUE || v.is_empty() {
            qs.push(k.to_string());
        } else {
            qs.push(format!("{}={}", k, v));
        }
    }

    format!("?{}", qs.join("&"))
}

pub mod hash;
