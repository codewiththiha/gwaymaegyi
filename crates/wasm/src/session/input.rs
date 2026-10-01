use gwaymaegyi_search::SearchLimits;
use std::str::FromStr;
use wasm_bindgen::JsError;

pub(super) fn integer<T: FromStr>(value: f64) -> Result<T, JsError> {
    if !value.is_finite() {
        return Err(JsError::new("value must be a finite nonnegative integer"));
    }
    (value + 0.0)
        .to_string()
        .parse()
        .map_err(|_| JsError::new("value must be a nonnegative integer in range"))
}
pub(super) fn limits(depth: f64, nodes: &str) -> Result<SearchLimits, JsError> {
    Ok(SearchLimits {
        depth: integer(depth)?,
        nodes: nodes
            .parse()
            .map_err(|_| JsError::new("nodes must be an unsigned 64-bit decimal integer"))?,
    })
}
