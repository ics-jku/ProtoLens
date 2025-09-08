use futures::stream::SplitSink;
use futures::{lock::Mutex, SinkExt, StreamExt};
use serde_json::Error;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::broadcast::Sender;
use warp::filters::ws::Message;
use warp::ws::WebSocket;

use crate::command::{Command, GenericCommand};
use crate::gdb_proxy::{self, GdbStatus};
use crate::options::{self, Options};
use crate::transaction::{ToBinary, Transaction};
use crate::virtual_prototype::{VPCtrlMsg, VPLayout, VPMode, VP};
use crate::{Project, ProjectTranfer};

pub struct Gdb {
    pub connection_status: Arc<Mutex<GdbStatus>>,
    // channel on which status updates are sent by gdb_proxy
    pub proxy_receiver: Sender<GdbStatus>,
    // channel on which continue commands are sent to gdb_proxy
    pub proxy_sender: Sender<u32>,
}

pub struct State {
    pub vp: Arc<Mutex<Option<VP>>>,
    pub pr: Arc<(Vec<Project>, Vec<PathBuf>)>,
    pub gdb: Gdb,
    pub vp_channel: Arc<Sender<VPCtrlMsg>>,
    pub options: Arc<Options>,
}

pub struct LocalState {
    pub sent_steps: usize,
}

pub async fn handle(ws: WebSocket, state: Arc<State>) {
    let (mut sndr, mut rcvr) = ws.split();
    let sndr_ptr = &mut sndr;

    let sent_steps = send_state(sndr_ptr, state.clone()).await;
    let mut l_state = LocalState { sent_steps };
    send_transactions(sndr_ptr, state.vp.clone(), &mut l_state).await;

    let mut vp_recv = state.vp_channel.clone().subscribe();
    let mut gdb_status_recv = state.gdb.proxy_receiver.subscribe();
    loop {
        tokio::select! {
            biased;
            // This block handles incoming websocket messages
            body = rcvr.next() => {
                match body {
                    Some(b) => match b{
                        Ok(msg) => handle_msg(msg, sndr_ptr, state.clone(),&mut l_state).await,
                        Err(error) => {
                            println!("[CH] msg err [{error}]");
                            break;
                        }
                    },
                    None => break
                }
            }
            // This block handles updates from the VP transaction receiver
            update_signal = vp_recv.recv() =>{
                if update_signal.is_ok(){
                    match update_signal.unwrap(){
                        VPCtrlMsg::RecvModule => send_layout(sndr_ptr, state.vp.clone()).await,
                        VPCtrlMsg::RecvTransaction => send_transactions(sndr_ptr, state.vp.clone(),&mut l_state).await,
                        VPCtrlMsg::Shutdown => {},
                    }
                }
            }
            // This block relays updates from the gdb connection to the PLW
            con_update = gdb_status_recv.recv() => {
                if con_update.is_ok(){
                    let signal = con_update.unwrap();
                    let mut con = state.gdb.connection_status.lock().await;
                    *con = signal;
                    send_command(sndr_ptr,Command::Status,con.to_string()).await;
                }
            }
        };
    }
}

async fn send_transactions(
    sndr: &mut SplitSink<WebSocket, Message>,
    vp_mutex: Arc<Mutex<Option<VP>>>,
    l_state: &mut LocalState,
) {
    let mut vp_lock = vp_mutex.lock().await;
    if vp_lock.is_none() {
        return;
    }
    let vp = vp_lock.as_mut().unwrap();
    let mut trans_lock = vp.steps.lock().await;

    let len = trans_lock.len();
    if len == 0 || len == l_state.sent_steps {
        return;
    }

    // Step mode: send new transactions since last call
    // Stream mode: send all transactions and clear vector
    if let Some(transactions) = trans_lock.get(l_state.sent_steps..len) {
        let mut buffer: Vec<u8> = Vec::new();
        // first 8 byte of each packet is the current transaction count
        let send_size = (Transaction::BIN_SIZE * transactions.len()) + 8;
        buffer.reserve(send_size);

        let cnt = vp.tcount as u64;
        buffer.extend_from_slice(&cnt.to_le_bytes());
        for transaction in transactions {
            buffer.extend_from_slice(&transaction.to_binary());
        }

        let packet = Message::binary(buffer.clone());
        buffer.clear();

        vp.tcount += transactions.len();
        let _ = sndr.send(packet).await;

        if vp.mode == VPMode::Stream {
            trans_lock.clear();
        } else if vp.mode == VPMode::Step {
            l_state.sent_steps = vp.tcount;
        }
    }
}

async fn handle_msg(
    message: Message,
    sndr: &mut SplitSink<WebSocket, Message>,
    state: Arc<State>,
    local_state: &mut LocalState,
) {
    if !message.is_text() {
        return;
    }

    let Ok(msg) = message.to_str() else {
        println!("[CH] could not get message text");
        return;
    };

    let Ok(cmd): Result<GenericCommand, Error> = serde_json::from_str(msg) else {
        println!("[CH] could not parse GenericCommand");
        return;
    };

    match cmd.command {
        Command::Start => {
            let is_running = get_status(state.vp.clone()).await;
            if is_running && cmd.value.is_empty() {
                stop_vp(sndr, state).await;
                local_state.sent_steps = 0;
            } else {
                start_vp(sndr, state.clone(), cmd.value).await;
            }
        }
        Command::Status => send_status(sndr, state.vp.clone()).await,
        Command::Step => {
            match cmd.value.parse::<u32>() {
                Ok(steps) => {
                    if steps < 1 {
                        return;
                    }

                    if let Err(e) = state.gdb.proxy_sender.send(steps) {
                        println!("[CH] could not send steps to gdb proxy{e}");
                    }
                }
                Err(_err) => unimplemented!(),
            };
        }
        Command::Options => unimplemented!(),
    }
}

async fn start_vp(sndr: &mut SplitSink<WebSocket, Message>, state: Arc<State>, command: String) {
    let Ok(cmd) = serde_json::from_str(&command) else {
        println!("[CH] could not parse StartCommand");
        return;
    };

    if get_status(state.vp.clone()).await {
        println!("[CH] VP is running already");
        return;
    }

    let Some(start_opt) = options::get_vp_args(cmd, state.clone()) else {
        return;
    };
    let binary = start_opt.binary.clone();

    let Ok(new_vp) = VP::start(
        start_opt.vp,
        start_opt.binary,
        start_opt.args,
        start_opt.mode,
        state.vp_channel.clone(),
        Some(state.options.vp_opt.vp_trace_port),
    )
    .await
    else {
        return;
    };
    let mut vp_lock = state.vp.lock().await;
    let vp = vp_lock.insert(new_vp);

    // spawn gdbgui if needed
    if start_opt.arch.is_some() {
        if let Ok(gdb_subprocess) =
            gdb_proxy::start_gdbgui(&state.options.gdb_opt, start_opt.arch.unwrap(), &binary)
        {
            vp.gdbgui = Some(gdb_subprocess)
        }
    }

    println!("[CH] VP with PID [{}] started", vp.subproc.id());
    send_command(sndr, Command::Start, vp.is_running.to_string()).await;
}

async fn stop_vp(sndr: &mut SplitSink<WebSocket, Message>, state: Arc<State>) {
    let mut vp_lock = state.vp.lock().await;
    if vp_lock.is_some() && vp_lock.as_mut().unwrap().stop() {
        let msg = serde_json::to_string(&GenericCommand {
            command: Command::Start,
            value: "".to_string(),
        })
        .unwrap();
        let _ = sndr.send(Message::text(msg)).await;
        let _ = vp_lock.take(); // Drop old VP struct
    }
}

async fn send_command(sndr: &mut SplitSink<WebSocket, Message>, command: Command, value: String) {
    let _ = sndr
        .send(Message::text(
            serde_json::to_string(&GenericCommand { command, value }).unwrap(),
        ))
        .await;
}

#[allow(clippy::len_zero)]
async fn send_layout(sndr: &mut SplitSink<WebSocket, Message>, vp: Arc<Mutex<Option<VP>>>) {
    let mut vp_locked = vp.lock().await;
    if vp_locked.is_some() {
        let v = vp_locked.as_mut().unwrap();
        let arch_lock = v.arch.lock().await;
        if arch_lock.modules.len() > 0 {
            let layout = VPLayout {
                modules: arch_lock.modules.clone(),
                end_addrs: arch_lock.end_addrs.clone(),
                start_addrs: arch_lock.start_addrs.clone(),
            };
            let _ = sndr
                .send(Message::text(
                    serde_json::to_string(&layout).expect("[CH] could not serialize layout"),
                ))
                .await;
        }
    }
}

async fn send_state(sndr: &mut SplitSink<WebSocket, Message>, state: Arc<State>) -> usize {
    send_status(sndr, state.vp.clone()).await;

    let trans: ProjectTranfer = ProjectTranfer::from(state.pr.clone());
    let _ = sndr
        .send(Message::text(
            serde_json::to_string(&trans).expect("[CH] could not serialize state"),
        ))
        .await;

    send_layout(sndr, state.vp.clone()).await;

    let mut sent_steps: usize = 0;
    let mut vp_lock = state.vp.lock().await;
    if vp_lock.is_some() {
        let s = vp_lock.as_mut().unwrap().steps.lock().await;
        if !s.is_empty() {
            sent_steps = s.len() - 1;
        } else {
            sent_steps = 0;
        }
    }

    let gdb_status = state.gdb.connection_status.lock().await;
    let _ = send_command(sndr, Command::Status, gdb_status.to_string()).await;
    let _ = send_command(
        sndr,
        Command::Options,
        state.options.gdb_opt.gdbproxy_port.to_string(),
    )
    .await;

    sent_steps
}

async fn get_status(vp: Arc<Mutex<Option<VP>>>) -> bool {
    let mut vp_lock = vp.lock().await;
    if vp_lock.is_some() {
        return vp_lock.as_mut().unwrap().is_running;
    }
    false
}

async fn send_status(sndr: &mut SplitSink<WebSocket, Message>, vp: Arc<Mutex<Option<VP>>>) {
    let cmd = GenericCommand {
        command: Command::Status,
        value: get_status(vp.clone()).await.to_string(),
    };
    let Ok(msg) = serde_json::to_string(&cmd) else {
        return;
    };
    let _ = sndr.send(Message::text(msg)).await;
}
