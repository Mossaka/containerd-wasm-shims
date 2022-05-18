use anyhow::Context;
use chrono::{DateTime, Utc};
use containerd_shim as shim;
use containerd_shim_wasmtime_v1::sandbox::error::Error;
use containerd_shim_wasmtime_v1::sandbox::instance::EngineGetter;
use containerd_shim_wasmtime_v1::sandbox::oci;
use containerd_shim_wasmtime_v1::sandbox::Instance;
use containerd_shim_wasmtime_v1::sandbox::{instance::InstanceConfig, ShimCli,
};
use log::info;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use wasmtime::OptLevel;

pub struct Wasi {
    exit_code: Arc<(Mutex<Option<(u32, DateTime<Utc>)>>, Condvar)>,
    engine: wasmtime::Engine,

    id: String,
    stdin: String,
    stdout: String,
    stderr: String,
    bundle: String,
    shutdown_signal: Arc<(Mutex<bool>, Condvar)>,
}

pub fn maybe_open_stdio(path: &str) -> Result<Option<File>, Error> {
    if path.is_empty() {
        return Ok(None);
    }

    OpenOptions::new().read(true).write(true).create(true).open(path)
        .map(|file| Some(file))
        .map_err(|err| Error::Others(format!("could not open stdio: {}", err)))
}

pub fn prepare_module(bundle: String,) -> Result<PathBuf, Error> {
    let mut spec = oci::load(Path::new(&bundle).join("config.json").to_str().unwrap())?;

    spec.canonicalize_rootfs(&bundle)
        .map_err(|err| Error::Others(format!("could not canonicalize rootfs: {}", err)))?;
    let root = match spec.root() {
        Some(r) => r.path(),
        None => {
            return Err(Error::Others(
                "rootfs is not specified in the config.json".to_string(),
            ));
        }
    };

    info!("opening rootfs");
    let args = oci::get_args(&spec);
    
    let mut cmd = args[0].clone();
    let stripped = args[0].strip_prefix(std::path::MAIN_SEPARATOR);
    if stripped.is_some() {
        cmd = stripped.unwrap().to_string();
    }

    let mod_path = root.join(cmd);
    Ok(mod_path)
}

impl Instance for Wasi {
    type E = wasmtime::Engine;
    fn new(id: String, cfg: Option<&InstanceConfig<Self::E>>) -> Self {
        let cfg = cfg.unwrap();
        Wasi {
            exit_code: Arc::new((Mutex::new(None), Condvar::new())),
            engine: cfg.get_engine(),
            id,
            stdin: cfg.get_stdin().unwrap_or_default(),
            stdout: cfg.get_stdout().unwrap_or_default(),
            stderr: cfg.get_stderr().unwrap_or_default(),
            bundle: cfg.get_bundle().unwrap_or_default(),
            shutdown_signal: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
    fn start(&self) -> Result<u32, Error> {
        // let engine = self.engine.clone();

        let exit_code = self.exit_code.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        let (tx, rx) = channel::<Result<(), Error>>();
        let bundle = self.bundle.clone();
        let stdin = self.stdin.clone();
        let stdout = maybe_open_stdio(&self.stdout.clone()).context("could not open stdout")?;
        let stderr = maybe_open_stdio(&self.stderr.clone()).context("could not open stderr")?;

        thread::Builder::new()
            .name(self.id.clone())
            .spawn(move || {
                info!("get module");
                let m = match prepare_module(bundle) {
                    Ok(f) => f,
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };

                info!("module path is: {:?}", m);

                info!("notifying main thread we are about to start");
                tx.send(Ok(())).unwrap();

                let sat = Command::new("sat")
                        .env("SAT_HTTP_PORT", "80")
                        .env("HOME", "/")
                        .arg(m)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn();
                
                let mut sat = match sat {
                    Ok(mut sat) => {
                        info!("sat started");
                        sat
                    },
                    Err(err) => {
                        let (lock, cvar) = &*exit_code;
                        let mut exit_code = lock.lock().unwrap();
                        *exit_code = Some((137, Utc::now()));
                        info!("sat failed to start: {:?}", err);
                        tx.send(Err(Error::Others(format!("could not start sat: {}", err))))
                            .unwrap();
                        cvar.notify_all();
                        return;
                    }
                };
                // let child_stdout = sat.stdout.take().unwrap();
                // let mut reader = std::io::BufReader::new(child_stdout);

                let (lock, cvar) = &*shutdown_signal;
                let mut shutdown = lock.lock().unwrap();
                while !*shutdown {
                    let mut buf = [0u8; 1024];
                    shutdown = cvar.wait(shutdown).unwrap();
                    // for line in reader.by_ref().lines() {
                    //     info!(">>> {}", line.unwrap());
                    // }
                }

                info!(" >>> User requested shutdown: exiting");
                let _ = sat.kill();
                let (lock, cvar) = &*exit_code;
                let mut ec = lock.lock().unwrap();
                *ec = Some((0, Utc::now()));
                cvar.notify_all();
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
        let (lock, cvar) = &*self.shutdown_signal;
        let mut shutdown = lock.lock().unwrap();
        *shutdown = true;
        cvar.notify_all();

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
    type E = wasmtime::Engine;
    fn new_engine() -> Result<Self::E, Error> {
        let engine = Self::E::new(
            wasmtime::Config::default()
                .interruptable(true)
                .cranelift_opt_level(OptLevel::Speed),
        )?;
        Ok(engine)
    }
}

fn main() {
    shim::run::<ShimCli<Wasi, _>>("io.containerd.aspdotnet.v1", None);
}
