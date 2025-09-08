use futures::FutureExt;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::process::{Child, Command};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::options::GdbOptions;

#[derive(Clone, Debug, PartialEq)]
pub enum GdbStatus {
    Connected,
    NotConnected,
}

impl fmt::Display for GdbStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn start_gdbgui(options: &GdbOptions, arch: String, bin_path: &str) -> Result<Child, String> {
    // create gdbcmd file
    let cmds = format!(
        "set architecture riscv:{}\ntarget remote 127.0.0.1:{}",
        arch, options.gdbproxy_port
    );
    if let Ok(mut file) = File::create("gdbcmd") {
        let _ = file.write(cmds.as_bytes());
    }

    let gdbgui_path = options.gdbgui.to_str();
    let gdb_path = options.gdb_bin.to_str();
    if gdbgui_path.is_none() {
        return Err("[GDB] gdbgui path is empty".to_string());
    } else if gdb_path.is_none() {
        return Err("[GDB] gdb path is empty".to_string());
    }

    // create gdb args
    let g_arg = format!("{} --command gdbcmd {}", gdb_path.unwrap(), bin_path);
    let args: Vec<String> = vec![
        "-p".to_string(),
        options.gdbgui_port.to_string(),
        "-g".to_string(),
        g_arg,
    ];

    let mut c = Command::new(gdbgui_path.unwrap());
    let c_args = c.args(&args);
    c_args
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit());

    match c_args.spawn() {
        Ok(gdbgui) => {
            println!("[GDB] gdbgui with PID [{}] started", gdbgui.id());
            println!("[GDB] Args {args:?}");
            Ok(gdbgui)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub async fn run(
    address: String,
    upstream_port: u16,
    downstream_port: u16,
    cmd_channel: Sender<u32>,
    status_channel: Sender<GdbStatus>,
) -> Result<(), Box<dyn Error>> {
    let mut upstream_addr = address.clone();
    let mut downstream_addr = address.clone();

    upstream_addr.push(':');
    upstream_addr.push_str(&upstream_port.to_string());
    downstream_addr.push(':');
    downstream_addr.push_str(&downstream_port.to_string());

    let listener_addr: SocketAddr = downstream_addr.parse().expect("socket addr");
    let listener = TcpListener::bind(&listener_addr).await?;

    let mut recv = cmd_channel.subscribe();
    let _ = &mut status_channel.send(GdbStatus::NotConnected);

    loop {
        println!("[PROXY] Waiting for downstream connection on {listener_addr}");
        let (mut client_stream, _) = listener.accept().await?;

        let mut remote = TcpStream::connect(&upstream_addr).await?;

        // notify the server
        let _ = &mut status_channel.send(GdbStatus::Connected);

        let (mut client_read, mut client_write) = client_stream.split();
        let (mut remote_read, mut remote_write) = remote.split();

        let (rx, _) = broadcast::channel::<()>(32);

        let (down, up) = tokio::join! {
            stream_copy(&mut remote_read, &mut client_write,rx.subscribe(),None)
                .then(|r| { let _ = rx.send(()); async { r } }),
            stream_copy(&mut client_read, &mut remote_write,rx.subscribe(),Some(&mut recv))
                .then(|r| { let _ = rx.send(()); async { r } }),
        };
        if down.is_ok() && up.is_ok() {
            println!(
                "[PROXY] Up/downstream copied {}/{} bytes",
                down.unwrap(),
                up.unwrap()
            );
        }
        let _ = &mut status_channel.send(GdbStatus::NotConnected);
    }
}

async fn stream_copy<R, W>(
    read: &mut R,
    write: &mut W,
    mut kill_signal: Receiver<()>,
    recv: Option<&mut Receiver<u32>>,
) -> tokio::io::Result<usize>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut copied = 0;
    let mut buf = [0u8; 2048];

    // loop behaviour differs for up and downstream
    if recv.is_some() {
        println!("[PROXY] gdbgui connection established");
        let step_buf = "$vCont;c#a8".to_string();
        let rec = recv.unwrap();
        loop {
            let bytes_read;
            let mut step_read = false;
            tokio::select! {
                biased;

                result = read.read(&mut buf) => {
                    use std::io::ErrorKind::{ConnectionReset, ConnectionAborted};
                    bytes_read = result.or_else(|e| match e.kind() {
                        ConnectionReset | ConnectionAborted => Ok(copied),
                        _ => Err(e)
                    })?;
                },

                // check for external inject command
                result = rec.recv() => {
                    bytes_read = 0;
                    step_read = true;
                    if result.is_ok(){
                        let steps = result.unwrap();
                        println!("[PROXY] injecting {steps} step packets");
                        for _ in 0..steps{
                            write.write_all(step_buf.as_bytes()).await?;
                        }
                    }
                },
                _ = kill_signal.recv() =>{
                    break;
                }
            }

            if step_read {
                continue;
            }

            if bytes_read == 0 {
                break;
            }

            write.write_all(&buf[0..bytes_read]).await?;
            copied += bytes_read;
        }
    } else {
        println!("[PROXY] gdb stub connection established");
        loop {
            let bytes_read;
            tokio::select! {
                biased;

                result = read.read(&mut buf) => {
                    use std::io::ErrorKind::{ConnectionReset, ConnectionAborted};
                    bytes_read = result.or_else(|e| match e.kind() {
                        ConnectionReset | ConnectionAborted => Ok(copied),
                        _ => Err(e)
                    })?;
                },
                _ = kill_signal.recv() =>{
                    break;
                }
            }

            if bytes_read == 0 {
                break;
            }

            write.write_all(&buf[0..bytes_read]).await?;
            copied += bytes_read;
        }
    }

    Ok(copied)
}
