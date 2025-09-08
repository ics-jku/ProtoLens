use serde::{Deserialize, Serialize};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc};

use crate::client_handler::State;
use crate::command::StartCommand;
use crate::virtual_prototype::VPMode;

#[derive(Deserialize, Debug)]
pub struct Options {
    pub serv_opt: ServerOptions,
    pub vp_opt: VPOptions,
    pub gdb_opt: GdbOptions,
    pub bin_dir: PathBuf,
    pub vp_dir: PathBuf,
    pub gui_vp_kit_dir: String,
    pub gui_vp_args: String,
}

#[derive(Deserialize, Debug)]
pub struct VPOptions {
    pub vp_debug_port: u16,
    pub vp_trace_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct ServerOptions {
    pub static_dir: PathBuf,
    pub port: u16,
    pub address: String,
}

#[derive(Deserialize, Debug)]
pub struct GdbOptions {
    pub gdbgui_port: u16,
    pub gdbproxy_port: u16,
    pub gdb_bin: PathBuf,
    pub gdbgui: PathBuf,
}

#[derive(Serialize, Debug)]
pub struct Project {
    pub directory: PathBuf,
    pub binary: String,
    pub source: String,
}

#[derive(Serialize, Debug)]
pub struct ProjectTranfer {
    pub dirs: Vec<String>,
    pub vps: Vec<String>,
}

pub struct StartOptions {
    pub vp: String,
    pub binary: String,
    pub args: Vec<String>,
    pub arch: Option<String>,
    pub mode: VPMode,
}

impl From<Arc<(Vec<Project>, Vec<PathBuf>)>> for ProjectTranfer {
    fn from(value: Arc<(Vec<Project>, Vec<PathBuf>)>) -> Self {
        let mut dirs = Vec::new();
        let mut vps = Vec::new();

        for dir in value.0.iter() {
            let file = &dir.directory.file_name().unwrap().to_str();
            if file.is_some() {
                dirs.push(file.unwrap().to_owned());
            }
        }

        for vp in value.1.iter() {
            let file = &vp.file_name().unwrap().to_str();
            if file.is_some() {
                vps.push(file.unwrap().to_owned());
            }
        }

        ProjectTranfer { dirs, vps }
    }
}

fn get_vp(name: &str, vps: &Vec<PathBuf>) -> Result<String, bool> {
    for p in vps {
        if p.ends_with(name) {
            return Ok(p.to_str().unwrap().to_owned());
        }
    }
    Err(false)
}

fn get_binary(proj_name: &str, projects: &Vec<Project>) -> Result<String, bool> {
    for p in projects {
        if p.directory.ends_with(proj_name) {
            let mut path = p.directory.clone();
            path.push(p.binary.clone());
            return Ok(path.to_str().unwrap().to_owned());
        }
    }
    Err(false)
}

pub fn load_projects(wd: PathBuf) -> Vec<Project> {
    let mut projects: Vec<Project> = Vec::new();
    let dir_entries = fs::read_dir(wd).expect("[MAIN] could not read working dir");

    for entry in dir_entries {
        let entry = entry.expect("[MAIN] could not open directory entry").path();
        if entry.is_dir() {
            if let Ok(proj) = get_project(entry) {
                projects.push(proj);
            }
        }
    }

    projects
}

pub fn load_vps(wd: PathBuf) -> Vec<PathBuf> {
    let mut vps: Vec<PathBuf> = Vec::new();
    let dir_entries = fs::read_dir(wd).expect("[MAIN] could not read vps dir");

    for entry in dir_entries {
        let entry = entry.expect("[MAIN] could not open vps directory").path();
        if entry.is_file() && entry.metadata().unwrap().permissions().mode() & 0o111 != 0 {
            vps.push(entry);
        }
    }

    vps
}

fn get_project(dir: PathBuf) -> Result<Project, bool> {
    let mut bin = String::from("");
    let mut src = String::from("");
    let dir_entries = fs::read_dir(dir.clone()).expect("[MAIN] could not read project dir");

    for entry in dir_entries {
        let entry = entry
            .expect("[MAIN] could not open project directory")
            .path();
        if entry.is_file() {
            let is_executable = entry
                .metadata()
                .expect("[MAIN] could not get metadata")
                .permissions()
                .mode()
                & 0o111
                != 0;
            let ext = entry.extension();

            let file_name = entry.file_name().unwrap().to_str().unwrap().to_string();
            if is_executable || ext.is_some() && ext.unwrap() == "elf" {
                bin = file_name;
            } else if file_name.ends_with(".c") || file_name.ends_with(".S") {
                src = file_name;
            }
        }
    }

    if bin.is_empty() {
        return Err(false);
    }

    Ok(Project {
        directory: dir,
        binary: bin,
        source: src,
    })
}

pub fn get_guivp_args(vp_name: &str, guivp_path: &str, guivp_args: &str) -> (String, String) {
    let mut rv = "64";
    let mut cores = "sc";
    let mut mem_size: u32 = 2 * 1024 * 1024 * 1024;

    if vp_name.contains("32") {
        rv = "32";
        mem_size = 1024 * 1024 * 1024;
    }
    if vp_name.contains("mc") {
        cores = "mc";
    }

    // add necessary arguments for GUI-VP-Kit
    let args = format!(" {guivp_args} --dtb-file={guivp_path}/dt/linux-vp_rv{rv}_{cores}.dtb --kernel-file {guivp_path}/buildroot_rv{rv}/output/images/Image --mram-root-image {guivp_path}/runtime_mram/mram_rv{rv}_root.img --mram-data-image {guivp_path}/runtime_mram/mram_rv{rv}_data.img --memory-size {mem_size}");
    let binary = format!("{guivp_path}/buildroot_rv{rv}/output/images/fw_jump.elf");
    (args, binary)
}

pub fn get_vp_args(mut start_cmd: StartCommand, state: Arc<State>) -> Option<StartOptions> {
    let Ok(vp) = get_vp(&start_cmd.vp, &state.pr.1) else {
        println!("[CH] Could not find VP {}", start_cmd.vp);
        return None;
    };

    // Check for linux VP and add additional arguments from GUI-VP Kit
    let mut bin_res = get_binary(&start_cmd.proj, &state.pr.0);
    if start_cmd.vp.contains("linux") {
        println!("[CH] Detected Linux VP. Trying to autofill args");
        let (gui_vp_args, gui_vp_bin) = get_guivp_args(
            &start_cmd.vp,
            &state.options.gui_vp_kit_dir,
            &state.options.gui_vp_args,
        );
        start_cmd.args.push_str(&gui_vp_args);
        bin_res = Ok(gui_vp_bin)
    }

    if bin_res.is_err() {
        println!("[CH] Could not find binary for {}", start_cmd.proj);
        return None;
    }

    // Check if debug architecture was given (only needed in debug mode)
    let mut gdb_arch: Option<String> = None;
    let mut mode = VPMode::Stream;
    if start_cmd.args.contains("--debug-mode") {
        if start_cmd.gdb_arch != "rv32" && start_cmd.gdb_arch != "rv64" {
            println!("[CH] Cannot start VP with debug mode without debugger architecture");
            return None;
        }
        gdb_arch = Some(start_cmd.gdb_arch);
        mode = VPMode::Step;
        let debug_port = format!(" --debug-port {}", state.options.vp_opt.vp_debug_port);
        start_cmd.args.push_str(&debug_port);
    }

    if start_cmd.args.contains("--debug-bus-mode") {
        let debug_bus_port = format!(" --debug-bus-port {}", state.options.vp_opt.vp_trace_port);
        start_cmd.args.push_str(&debug_bus_port);
    } else {
        println!("[CH] Cannot start VP without --debug-bus-mode");
        return None;
    }

    let arg_list = start_cmd
        .args
        .split_ascii_whitespace()
        .map(String::from)
        .collect();

    Some(StartOptions {
        vp,
        binary: bin_res.unwrap(),
        args: arg_list,
        arch: gdb_arch,
        mode,
    })
}
