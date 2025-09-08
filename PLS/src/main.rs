use futures::lock::Mutex;
use std::net::Ipv4Addr;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::signal::unix::SignalKind;
use tokio::sync::broadcast::{self, Sender};
use warp::{ws::WebSocket, Filter, Rejection, Reply};

use client_handler::{Gdb, State};
use gdb_proxy::GdbStatus;
use options::{Options, Project, ProjectTranfer};
use virtual_prototype::{VPCtrlMsg, VP};

pub mod client_handler;
pub mod command;
pub mod gdb_proxy;
pub mod options;
pub mod transaction;
pub mod virtual_prototype;

#[tokio::main]
async fn main() {
    let options = Arc::new(load_options(PathBuf::from("./appsettings.json")));
    let address: Ipv4Addr = options
        .serv_opt
        .address
        .parse()
        .expect("[MAIN] could not parse address");

    let pr = (
        options::load_projects(options.bin_dir.clone()),
        options::load_vps(options.vp_dir.clone()),
    );
    println!(
        "[MAIN] loaded {} virtual prototype(s) and {} source project(s)",
        pr.1.len(),
        pr.0.len()
    );
    let vp_opt: Option<VP> = None;
    let vp = Mutex::new(vp_opt);
    let (vp_channel, _) = broadcast::channel::<VPCtrlMsg>(32);

    let gdb_channels = start_gdbproxy(
        options.serv_opt.address.clone(),
        options.vp_opt.vp_debug_port,
        options.gdb_opt.gdbproxy_port,
    );

    let state = Arc::new(State {
        gdb: Gdb {
            connection_status: Arc::new(Mutex::new(GdbStatus::NotConnected)),
            proxy_receiver: gdb_channels.1,
            proxy_sender: gdb_channels.0,
        },
        options: options.clone(),
        vp: Arc::new(vp),
        pr: Arc::new(pr),
        vp_channel: Arc::new(vp_channel),
    });

    // setup websocket and static file routes
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and_then(move |ws| ws_upgrade(ws, state.clone()))
        .with(warp::cors().allow_any_origin());
    let file_route = warp::fs::dir(options.serv_opt.static_dir.clone());
    let routes = ws_route.or(file_route);

    let (_, local_server) = warp::serve(routes).bind_with_graceful_shutdown(
        (address, options.serv_opt.port),
        async move {
            // register all signals which initiate a graceful shutdown
            let mut term = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
            let mut hup = tokio::signal::unix::signal(SignalKind::hangup()).unwrap();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = term.recv() => {}
                _ = hup.recv() => {}
            }
        },
    );

    println!(
        "[MAIN] Server listening on \nhttp://{}:{}/",
        address, options.serv_opt.port
    );

    match tokio::join!(tokio::task::spawn(local_server)).0 {
        Ok(()) => println!("[MAIN] Server stopped"),
        Err(e) => println!("[MAIN] Thread join error {e}"),
    };
}

fn start_gdbproxy(
    address: String,
    vp_port: u16,
    gdbproxy_port: u16,
) -> (Sender<u32>, Sender<GdbStatus>) {
    let (gdb_cmd, _) = broadcast::channel::<u32>(32);
    let (gdb_status, _) = broadcast::channel::<GdbStatus>(32);
    let gdb_cmd_cl = gdb_cmd.clone();
    let gdb_status_cl = gdb_status.clone();

    tokio::task::spawn(async move {
        match gdb_proxy::run(address, vp_port, gdbproxy_port, gdb_cmd_cl, gdb_status_cl).await {
            Ok(()) => println!("[PROXY] exited normally"),
            Err(e) => println!("[PROXY] exited with {e}"),
        }
    });
    (gdb_cmd, gdb_status)
}

async fn ws_upgrade(ws: warp::ws::Ws, state: Arc<State>) -> Result<impl Reply, Rejection> {
    Ok(ws.on_upgrade(move |socket| handle_ws_client(socket, state)))
}

async fn handle_ws_client(ws: WebSocket, state: Arc<State>) {
    println!("[MAIN] client connect");

    client_handler::handle(ws, state).await;

    println!("[MAIN] client disconnect");
}

fn load_options(path: PathBuf) -> Options {
    let fcont = fs::read_to_string(&path).expect("[MAIN] could not read file");
    println!(
        "[MAIN] loaded {}",
        path.file_name().unwrap().to_str().unwrap()
    );
    serde_json::from_str(&fcont).expect("[MAIN] could not parse settings")
}
