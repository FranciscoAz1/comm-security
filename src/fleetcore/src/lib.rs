use risc0_zkvm::{Digest, Receipt};
use serde::{Deserialize, Serialize};

// COnstant variable
pub const DEAD: Digest = Digest::from([0u8; 32]);

// Struct sent by the rust code for input on the methods join, wave and win
// The struct is read by the zkvm code and the data is used to generate the output Journal
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BaseInputs {
    pub gameid: String,
    pub fleet: String,
    pub board: Vec<u8>,
    pub random: String,
}

// Struct sent by the rust code for input on the methods fire and report
// The struct is read by the zkvm code and the data is used to generate the output Journal
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FireInputs {
    pub gameid: String,
    pub fleet: String,
    pub board: Vec<u8>,
    pub random: String,
    pub target: String,
    pub pos: u8,
}

// Enum used to define the command that will be sent to the server by the host in the communication packet
#[derive(Deserialize, Serialize)]
pub enum Command {
    Join,
    Fire,
    Report,
    Wave,
    Win,
}

// Struct used to specify the packet sent from the client to the blockchain server
#[derive(Deserialize, Serialize)]
pub struct CommunicationData {
    pub cmd: Command,
    pub receipt: Receipt,
}

// Struct to specify the  output journal for join, wave and win methods
#[derive(Deserialize, PartialEq, Eq, Serialize, Default)]
pub struct BaseJournal {
    pub gameid: String,
    pub fleet: String,
    pub board: Digest,
}

// Struct to specify the  output journal for fire method
#[derive(Deserialize, PartialEq, Eq, Serialize, Default)]
pub struct FireJournal {
    pub gameid: String,
    pub fleet: String,
    pub board: Digest,
    pub target: String,
    pub pos: u8,
}

// Struct to specify the  output journal for report method
#[derive(Deserialize, PartialEq, Eq, Serialize, Default)]
pub struct ReportJournal {
    pub gameid: String,
    pub fleet: String,
    pub report: String,
    pub pos: u8,
    pub board: Digest,
    pub next_board: Digest,
}
