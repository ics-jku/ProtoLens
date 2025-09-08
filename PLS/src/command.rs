use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Command {
    Start,
    Status,
    Step,
    Options,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenericCommand {
    pub command: Command,
    pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct StartCommand {
    pub vp: String,
    pub proj: String,
    pub args: String,
    pub gdb_arch: String,
}

#[derive(Deserialize, Debug)]
pub struct StepCommand {
    /// number of steps to be performed on the virtual prototype
    pub steps: u32,
    /// indicates if the transactions should be sent as one message
    pub batch_trans: bool,
}

#[derive(Serialize, Debug)]
pub struct StepResponse {
    pub trans: Vec<String>,
    pub steps_done: u32,
}

#[derive(Deserialize, Debug)]
pub struct StepUntilCommand {
    pub action: String,
    pub module: String,
    pub start_addr: String,
    pub end_addr: String,
    pub data: String,
    pub time: String,
    pub max_steps: u32,
    pub batch_trans: bool,
}

impl StepUntilCommand {
    pub fn parse_addrs(&mut self) -> Result<(Option<u64>, Option<u64>), String> {
        let s_addr = u64::from_str_radix(&self.start_addr, 16).ok();
        let e_addr = u64::from_str_radix(&self.end_addr, 16).ok();

        if s_addr > e_addr {
            return Err(String::from("Start address is bigger than end address"));
        }

        Ok((s_addr, e_addr))
    }

    pub fn parse_time(&mut self) -> Option<f64> {
        str::parse::<f64>(&self.time).ok()
    }
}
