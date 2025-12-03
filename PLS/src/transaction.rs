use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, PartialEq, Serialize)]
pub enum TransactionCmd {
    Read,
    Write,
}

impl FromStr for TransactionCmd {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(TransactionCmd::Read),
            "W" => Ok(TransactionCmd::Write),
            _ => Err(()),
        }
    }
}

impl Display for TransactionCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionCmd::Read => write!(f, "Read"),
            TransactionCmd::Write => write!(f, "Write"),
        }
    }
}

impl TransactionCmd {
    fn to_byte(&self) -> u8 {
        if *self == TransactionCmd::Read {
            return 0;
        }
        1
    }
}

pub trait ToBinary {
    const BIN_SIZE: usize = 28;
    fn to_binary(&self) -> [u8; Transaction::BIN_SIZE];
}

#[derive(Serialize, Debug)]
pub struct Transaction {
    pub sim_time: u64,
    pub action: TransactionCmd,
    pub initiator: String,
    pub target: u8,
    pub address: String,
    pub data_length: u8,
    pub data: String,
}

impl ToBinary for Transaction {
    fn to_binary(&self) -> [u8; Transaction::BIN_SIZE] {
        let mut arr = [0; Transaction::BIN_SIZE];
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&self.sim_time.to_le_bytes());
        data.push(self.action.to_byte());
        data.push(self.initiator.chars().last().unwrap() as u8);
        data.extend_from_slice(&self.target.to_le_bytes());
        data.extend_from_slice(&(u64::from_str_radix(&self.address, 16).unwrap()).to_le_bytes());
        data.push(self.data_length);
        data.extend_from_slice(&(u64::from_str_radix(&self.data, 16).unwrap()).to_le_bytes());
        arr.copy_from_slice(data.as_slice()); // Panics if slice sizes do not match
        arr
    }
}

impl FromStr for Transaction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut data: Vec<&str> = s.split(';').collect();

        if data.len() < 6 || data[0].starts_with("I") {
            return Err(());
        }

        if data.len() == 6 {
            data.push("")
        }

        Ok(Transaction {
            action: TransactionCmd::from_str(data[0]).expect("[VP] could not parse action"),
            initiator: data[1].to_owned(),
            target: data[2].parse::<u8>().expect("[VP] could not parse target"),
            address: data[3].to_owned(),
            sim_time: data[4]
                .parse::<u64>()
                .expect("[VP] could not parse simulation time"),
            data_length: data[5]
                .trim_end_matches("\n")
                .parse::<u8>()
                .expect("[VP] could not parse data length"),
            data: data[6].trim_end_matches("\n").to_owned(),
        })
    }
}
