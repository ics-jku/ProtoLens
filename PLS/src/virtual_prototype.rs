use futures::lock::Mutex;
use serde::Serialize;
use std::process::{Child, Command};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast::Sender;
use tokio::time::{self};

use crate::transaction::Transaction;

#[derive(PartialEq, Clone)]
pub enum VPCtrlMsg {
    RecvTransaction,
    RecvModule,
    Shutdown,
}

#[derive(Debug, PartialEq)]
pub enum VPMode {
    Stream,
    Step,
}

#[derive(Default, Debug, Serialize)]
pub struct VPLayout {
    pub modules: Vec<String>,
    pub start_addrs: Vec<String>,
    pub end_addrs: Vec<String>,
}

#[derive(Debug)]
pub struct VP {
    pub subproc: Child,
    pub gdbgui: Option<Child>,
    pub channel: Arc<Sender<VPCtrlMsg>>,
    pub is_running: bool,
    pub steps: Arc<Mutex<Vec<Transaction>>>,
    pub arch: Arc<Mutex<VPLayout>>,
    pub mode: VPMode,
    pub tcount: usize,
}

impl Drop for VP {
    fn drop(&mut self) {
        self.stop();
    }
}

impl VP {
    pub async fn start(
        vp_path: String,
        bin_path: String,
        mut args: Vec<String>,
        mode: VPMode,
        channel: Arc<Sender<VPCtrlMsg>>,
        port: Option<u16>,
    ) -> Result<VP, ()> {
        args.push(bin_path);

        println!("[VP] {mode:?} [{vp_path}] {args:?}");

        let mut command = Command::new(vp_path);
        let vp: &mut Command = command.args(args);

        vp.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        match mode {
            VPMode::Step => vp.stdin(std::process::Stdio::null()),
            VPMode::Stream => vp.stdin(std::process::Stdio::inherit()),
        };

        if let Ok(subproc) = vp.spawn() {
            return connect_vp(subproc, mode, channel, port.unwrap_or(5006)).await;
        }

        Err(())
    }

    pub fn stop(&mut self) -> bool {
        self.is_running = false;
        let mut receiver_shutdown = false;
        if let Ok(received) = self.channel.send(VPCtrlMsg::Shutdown) {
            receiver_shutdown = received > 0;
        }
        if let Some(mut gdbgui) = self.gdbgui.take() {
            if gdbgui.kill().is_ok() {
                println!("[VP] gdbgui with PID [{}] was killed", gdbgui.id());
            } else {
                println!("[VP] gdbgui with PID [{}] maybe alive", gdbgui.id());
            }
        }
        if self.subproc.kill().is_ok() && receiver_shutdown {
            println!("[VP] VP with PID [{}] was killed", self.subproc.id());
            true
        } else {
            println!("[VP] VP with PID [{}] maybe alive", self.subproc.id());
            false
        }
    }
}

async fn connect_vp(
    vp_process: Child,
    mode: VPMode,
    channel: Arc<Sender<VPCtrlMsg>>,
    port: u16,
) -> Result<VP, ()> {
    // sleep to let the VP startup
    let dur = time::Duration::from_millis(2000);
    thread::sleep(dur);

    match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(stream) => {
            println!("[VP] listening on bus dump");

            let responses = Arc::new(Mutex::new(Vec::<Transaction>::new()));
            let arch = Arc::new(Mutex::new(VPLayout::default()));
            let cp = responses.clone();
            let ac = arch.clone();
            let ch = channel.clone();

            // spawn task for receiving Transactions
            tokio::spawn(async move {
                recv_loop(stream, cp, ac, ch.clone()).await;
            });

            Ok(VP {
                subproc: vp_process,
                channel,
                is_running: true,
                steps: responses,
                arch,
                mode,
                tcount: 0,
                gdbgui: None,
            })
        }
        Err(_) => {
            println!("[VP] Could not connect to VP trace port (check args)");
            Err(())
        }
    }
}

async fn recv_loop(
    mut stream: TcpStream,
    mut responses: Arc<Mutex<Vec<Transaction>>>,
    mut layout: Arc<Mutex<VPLayout>>,
    channel: Arc<Sender<VPCtrlMsg>>,
) {
    let (sock_rx, _) = stream.split();
    let mut socket_recv = BufReader::new(sock_rx).lines();
    let mut layout_parsed = false;
    let mut interval = time::interval(Duration::from_millis(10));
    let mut status = (false, false);
    let mut cmd_recv = channel.subscribe();

    loop {
        tokio::select! {
            biased;
            // This block handles shutdown commands from the server
            cmd_res = cmd_recv.recv() =>{
                match cmd_res{
                    Ok(cmd) => if cmd == VPCtrlMsg::Shutdown {
                        break;
                    },
                    Err(_) => break
                }
            },
            // This block lets the server push updates to clients
            _ = interval.tick() => if status.0 || status.1{
                // Send archticture first otherwise transaction cannot be displayed
                if status.1{
                    let _ = channel.send(VPCtrlMsg::RecvModule);
                    status.1 = !status.1;
                }else if status.0{
                    let _ = channel.send(VPCtrlMsg::RecvTransaction);
                    status.0 = !status.0;
                }
            },
            // This block handles incomming TCP socket packets
            line_res = socket_recv.next_line() => {
                match line_res {
                    Ok(line) => if line.is_some() {
                            if let Some(res) = handle_response(&line.unwrap(),&mut responses,&mut layout,&mut layout_parsed).await{
                                status.0 = res.0;
                                if !status.1 {
                                    status.1 = res.1;
                                }
                            }
                        },
                    Err(e) => println!("{e}")
                }
            }
        };
    }
    println!("[VP] exiting tcp receiver");
}

async fn handle_response(
    buffer: &str,
    responses: &mut Arc<Mutex<Vec<Transaction>>>,
    layout: &mut Arc<Mutex<VPLayout>>,
    parsing: &mut bool,
) -> Option<(bool, bool)> {
    if let Ok(step) = Transaction::from_str(buffer) {
        responses.lock().await.push(step);
        if *parsing {
            *parsing = !*parsing;
            return Some((true, true));
        }
        return Some((true, false));
    } else {
        *parsing = true;
        let data: Vec<&str> = buffer.split(';').collect();
        if data.len() == 4 {
            let mut l_lock = layout.lock().await;
            l_lock.modules.push(data[1].to_string());
            l_lock.start_addrs.push(data[2].to_string());
            l_lock.end_addrs.push(data[3].to_string());
        }
    }
    None
}
