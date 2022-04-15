use anyhow::Context;
use chrono::{DateTime, Utc};
use containerd_shim as shim;
use containerd_shim_wasmtime_v1::sandbox::error::Error;
use containerd_shim_wasmtime_v1::sandbox::instance::EngineGetter;
use containerd_shim_wasmtime_v1::sandbox::oci;
use containerd_shim_wasmtime_v1::sandbox::Instance;
use containerd_shim_wasmtime_v1::sandbox::{
    instance::maybe_open_stdio, instance::InstanceConfig, ShimCli,
};
use log::info;
use spin_http_engine::HttpTrigger;
use tokio::runtime::Runtime;
use wasmtime::OptLevel;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use wasmtime::{Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use wasmtime_wasi::sync::TcpListener;
use spin_loader;
pub struct Wasi {
    interupt: Arc<RwLock<Option<wasmtime::InterruptHandle>>>,
    exit_code: Arc<(Mutex<Option<(u32, DateTime<Utc>)>>, Condvar)>,
    engine: wasmtime::Engine,

    id: String,
    stdin: String,
    stdout: String,
    stderr: String,
    bundle: String,
}

pub fn prepare_module(
    bundle: String,
) -> Result<(PathBuf, PathBuf), Error> {
    let mut spec = oci::load(Path::new(&bundle).join("config.json").to_str().unwrap())?;

    spec.canonicalize_rootfs(&bundle)
        .map_err(|err| Error::Others(format!("could not canonicalize rootfs: {}", err)))?;
    
    // let rootfs = oci::get_rootfs(&spec)?;
    // let args = oci::get_args(&spec);
    // let env = oci::env_to_wasi(&spec);

    // let mut cmd = args[0].clone();
    // let stripped = args[0].strip_prefix(std::path::MAIN_SEPARATOR);
    // if stripped.is_some() {
    //     cmd = stripped.unwrap().to_string();
    // }
    let working_dir = oci::get_root(&spec)?;
    let mod_path = working_dir.join("spin.toml");
    Ok((working_dir.to_path_buf(), mod_path))
}

impl Instance for Wasi {
    fn new(id: String, cfg: &InstanceConfig) -> Self {
        Wasi {
            interupt: Arc::new(RwLock::new(None)),
            exit_code: Arc::new((Mutex::new(None), Condvar::new())),
            engine: cfg.get_engine(),
            id,
            stdin: cfg.get_stdin().unwrap_or_default(),
            stdout: cfg.get_stdout().unwrap_or_default(),
            stderr: cfg.get_stderr().unwrap_or_default(),
            bundle: cfg.get_bundle().unwrap_or_default(),
        }
    }
    fn start(&self) -> Result<u32, Error> {
        let engine = self.engine.clone();

        let exit_code = self.exit_code.clone();
        let interupt = self.interupt.clone();
        let (tx, rx) = channel::<Result<(), Error>>();
        let bundle = self.bundle.clone();
        let stdin = self.stdin.clone();
        let stdout = self.stdout.clone();
        let stderr = self.stderr.clone();

        thread::Builder::new()
            .name(self.id.clone())
            .spawn(move || {
                
                let (working_dir, mod_path) = match prepare_module(bundle) {
                    Ok(f) => f,
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };

                info!("the path to spin.toml is: {}", mod_path.display());
                info!("the working path is: {}", working_dir.display());

                info!("notifying main thread we are about to start");
                tx.send(Ok(())).unwrap();
                info!("starting spin");
                let rt  = Runtime::new().unwrap();
                let app = rt.block_on(spin_loader::from_file(mod_path, working_dir)).unwrap();
                let http = rt.block_on(HttpTrigger::new("0.0.0.0:80".to_string(), app, None, None, Some(engine.clone()))).unwrap();
                rt.block_on(http.run()).unwrap();
            })?;

        info!("Waiting for start notification");
        match rx.recv().unwrap() {
            Ok(_) => (),
            Err(err) => {
                info!("error starting instance: {}", err);
                let code = self.exit_code.clone();

                let (lock, cvar) = &*code;
                let mut ec = lock.lock().unwrap();
                *ec = Some((139, Utc::now()));
                cvar.notify_all();
                return Err(err);
            }
        }

        Ok(1) // TODO: PID: I wanted to use a thread ID here, but threads use a u64, the API wants a u32
    }

    fn kill(&self, signal: u32) -> Result<(), Error> {
        if signal != 9 {
            return Err(Error::InvalidArgument(
                "only SIGKILL is supported".to_string(),
            ));
        }

        let interupt = self.interupt.read().unwrap();
        let i = interupt.as_ref().ok_or(Error::FailedPrecondition(
            "module is not running".to_string(),
        ))?;

        // TODO: exit the server

        i.interrupt();
        Ok(())
    }

    fn delete(&self) -> Result<(), Error> {
        Ok(())
    }

    fn wait(&self, channel: Sender<(u32, DateTime<Utc>)>) -> Result<(), Error> {
        let code = self.exit_code.clone();
        thread::spawn(move || {
            let (lock, cvar) = &*code;
            let mut exit = lock.lock().unwrap();
            while (*exit).is_none() {
                exit = cvar.wait(exit).unwrap();
            }
            let ec = (*exit).unwrap();
            channel.send(ec).unwrap();
        });

        Ok(())
    }
}

impl EngineGetter for Wasi {
    fn new_engine() -> Result<wasmtime::Engine, Error> {
        let engine = wasmtime::Engine::new(wasmtime::Config::default().interruptable(true).cranelift_opt_level(OptLevel::Speed))?;
        Ok(engine)
    }
}

fn main() {
    shim::run::<ShimCli<Wasi>>("io.containerd.spin.v1", None);
}
