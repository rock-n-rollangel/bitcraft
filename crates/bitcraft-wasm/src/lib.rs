use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmSchema {
    inner: bitcraft::schema::Schema,
}

#[wasm_bindgen]
impl WasmSchema {
    #[wasm_bindgen(constructor)]
    pub fn new(schema_json: &str) -> Result<WasmSchema, JsValue> {
        Err(JsValue::from("Not implemented"))
    }

    pub fn parse(&self, data: &[u8]) -> Result<JsValue, JsValue> {
        Err(JsValue::from("Not implemented"))
    }
}
