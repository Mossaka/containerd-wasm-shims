wit_bindgen_rust::export!("wasi-ce.wit");

use wasi_ce::*;
use cloudevents::{Event, AttributesReader};


struct WasiCe {}

impl wasi_ce::WasiCe for WasiCe {
    fn ce_handler(event: String) -> Result<String,Error> {
        println!("hello from wasm!");
        println!("");
        println!("event is {}", event);
        let event_: Event = serde_json::from_str(&event).unwrap();
        println!("event source: {}", event_.source());
        Ok(event)
    }
}

// TODO
// Error handling is currently not implemented.
impl From<anyhow::Error> for Error {
    fn from(_: anyhow::Error) -> Self {
        Self::Error
    }
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::Error
    }
}
