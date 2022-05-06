trait Engine {
    fn new(mut config: wasmtime::Config) -> Result<Self, Error>;
    fn get_inner() -> wasmtime::Engine;
}

// pub trait EngineGetter {
//     type Engine: Engine; 
//     fn new_engine() -> Result<Engine, Error> {
//         let engine = Engine::new(EngineConfig::default().interruptable(true))?;
//         Ok(engine)
//     }
// }