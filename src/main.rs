use containerd_shim_wasmtime_v1::sandbox::Instance;
use containerd_shim_wasmtime_v1::sandbox::error::Error;
use containerd_shim_wasmtime_v1::sandbox::oci;
use anyhow::Context;
use chrono::{DateTime, Utc};
use log::{info};
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;
use wasmtime::{Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use cloudevents::{EventBuilder, AttributesReader, EventBuilderV10};
use uuid::Uuid;
use rouille::Response;
use containerd_shim as shim;
use containerd_shim_wasmtime_v1::sandbox::{ShimCli, instance::maybe_open_stdio, instance::InstanceConfig};

wit_bindgen_wasmtime::import!("wasi-ce.wit");

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
    engine: wasmtime::Engine,
    bundle: String,
    stdin_path: String,
    stdout_path: String,
    stderr_path: String,
) -> Result<(WasiCtx, Module), Error> {
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
    let rootfs = oci::wasi_dir(root.to_str().unwrap(), OpenOptions::new().read(true))
        .map_err(|err| Error::Others(format!("could not open rootfs: {}", err)))?;
    let args = oci::get_args(&spec);
    let env = oci::env_to_wasi(&spec);

    info!("setting up wasi");
    let mut wasi_builder = WasiCtxBuilder::new()
        .args(args)?
        .envs(env.as_slice())?
        .preopened_dir(rootfs, "/")?;

    info!("opening stdin");
    let stdin = maybe_open_stdio(&stdin_path).context("could not open stdin")?;
    if stdin.is_some() {
        wasi_builder = wasi_builder.stdin(Box::new(stdin.unwrap()));
    }

    info!("opening stdout");
    let stdout = maybe_open_stdio(&stdout_path).context("could not open stdout")?;
    if stdout.is_some() {
        wasi_builder = wasi_builder.stdout(Box::new(stdout.unwrap()));
    }

    info!("opening stderr");
    let stderr = maybe_open_stdio(&stderr_path).context("could not open stderr")?;
    if stderr.is_some() {
        wasi_builder = wasi_builder.stderr(Box::new(stderr.unwrap()));
    }

    info!("building wasi context");
    let wctx = wasi_builder.build();
    info!("wasi context ready");

    let mut cmd = args[0].clone();
    let stripped = args[0].strip_prefix(std::path::MAIN_SEPARATOR);
    if stripped.is_some() {
        cmd = stripped.unwrap().to_string();
    }

    let mod_path = root.join(cmd);

    info!("loading module from file");
    let module = Module::from_file(&engine, mod_path)
        .map_err(|err| Error::Others(format!("could not load module from file: {}", err)))?;

    Ok((wctx, module))
}

pub struct WasiContext {
    pub wasi: WasiCtx,
    pub wasi_data: Option<wasi_ce::WasiCeData>
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
                info!("starting instance");
                let wasi_data = Some(wasi_ce::WasiCeData::default());

                let mut linker = Linker::new(&engine);

                match wasmtime_wasi::add_to_linker(&mut linker, |s: &mut WasiContext| &mut s.wasi)
                    .map_err(|err| Error::Others(format!("error adding to linker: {}", err)))
                {
                    Ok(_) => (),
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };

                wasi_ce::WasiCe::add_to_linker(&mut linker, |cx| cx.wasi_data.as_mut().unwrap()).unwrap();

                info!("preparing module");
                let m = match prepare_module(engine.clone(), bundle, stdin, stdout, stderr) {
                    Ok(f) => f,
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };


                let ctx = WasiContext {
                    wasi: m.0,
                    wasi_data,
                };

                let mut store = Store::new(&engine, ctx);

                info!("instantiating instnace");
                let i = match linker.instantiate(&mut store, &m.1).map_err(|err| {
                    Error::Others(format!("error instantiating module: {}", err))
                }) {
                    Ok(i) => i,
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };

                info!("getting interupt handle");
                match store.interrupt_handle().map_err(|err| {
                    Error::Others(format!("could not get interupt handle: {}", err))
                }) {
                    Ok(h) => {
                        let mut lock = interupt.write().unwrap();
                        *lock = Some(h);
                        drop(lock);
                    }
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };

                info!("notifying main thread we are about to start");
                tx.send(Ok(())).unwrap();

                info!("starting wasi instance");

                let (lock, cvar) = &*exit_code;
                let t = wasi_ce::WasiCe::new(&mut store, &i, |host| {
                    host.wasi_data.as_mut().unwrap()
                }).unwrap();
                
                let mut store = Mutex::new(store);

                let mut ec = lock.lock().unwrap();
                *ec = Some((0, Utc::now()));
                cvar.notify_all();
                rouille::start_server("0.0.0.0:80", move |request| {
                    handle_ce(request, &store, &t)
                });
            }
        )?;

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

fn handle_ce(request: &rouille::Request, store: &Mutex<Store<WasiContext>>, t: &wasi_ce::WasiCe<WasiContext>) -> Response {
    let mut data = request.data().expect("Oops, body already retrieved, problem \
                                          in the server");
    let mut buf = Vec::new();
    match data.read_to_end(&mut buf) {
        Ok(_) => (),
        Err(_) => return Response::text("Failed to read body")
    };
    let event = EventBuilderV10::new()
        .id(Uuid::new_v4().to_string())
        .source(request.url().as_str())
        .ty("com.microsoft.steelthread.wasm")
        .time(Utc::now())
        .data("string", buf)
        .build().unwrap();
    let event = serde_json::to_string(&event).unwrap();
    let mut store = store.lock().unwrap();
    let event = t.ce_handler(&mut *store, &event).unwrap();
    Response::text("hello world")
}

fn main() {
    shim::run::<ShimCli<Wasi>>("io.containerd.cehostshim.v1", None);
}