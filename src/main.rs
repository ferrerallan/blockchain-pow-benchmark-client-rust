use serde::{Serialize, Deserialize};
use serde_json;
use ring::digest::{Context, SHA256};
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::Client;
use std::error::Error;
use crossterm::{
    ExecutableCommand, terminal, style::{Color, PrintStyledContent, Stylize}, cursor::MoveTo,
};
use std::io::{self, Write};
use tokio::time::sleep;
use std::time::Duration;
use std::fmt::Write as FmtWrite; // Importar a trait Write

const BLOCKCHAIN_SERVER: &str = "http://localhost:3000";

#[derive(Serialize, Deserialize, Debug)]
struct BlockInfo {
    index: u64,
    previousHash: String,
    data: String,
    difficulty: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Block {
    index: u64,
    previousHash: String,
    timestamp: u64,
    data: String,
    difficulty: u64,
    nonce: u64,
    hash: String,
}

impl Block {
    fn from_block_info(block_info: BlockInfo) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        Block {
            index: block_info.index,
            previousHash: block_info.previousHash,
            timestamp,
            data: block_info.data,
            difficulty: block_info.difficulty,
            nonce: 0,
            hash: "".to_string(),
        }
    }

    fn calculate_hash(&self) -> String {
        let block_data = format!("{}{}{}{}{}", self.index, self.previousHash, self.timestamp, self.data, self.nonce);
        let mut context = Context::new(&SHA256);
        context.update(block_data.as_bytes());
        let digest = context.finish();

        let mut hash_string = String::new();
        for byte in digest.as_ref() {
            write!(&mut hash_string, "{:02x}", byte).expect("Unable to write");
        }
        hash_string
    }

    fn mine(&mut self, difficulty: u64, public_key: &str) {
        self.data.push_str(public_key);
        let prefix = "0".repeat(difficulty as usize);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&prefix) {
                break;
            }
            self.nonce += 1;
        }
    }

    fn to_dict(&self) -> BlockInfo {
        BlockInfo {
            index: self.index,
            previousHash: self.previousHash.clone(),
            data: self.data.clone(),
            difficulty: self.difficulty,
        }
    }
}

async fn get_next_block_info(client: &Client) -> Result<BlockInfo, Box<dyn Error>> {
    let response = client.get(format!("{}/blocks/next", BLOCKCHAIN_SERVER)).send().await?;
    let response_text = response.text().await?;
    
    let block_info: BlockInfo = serde_json::from_str(&response_text)?;
    Ok(block_info)
}

async fn mine_block<W: Write>(stdout: &mut W, client: &Client, block_info: BlockInfo, total_mined: &mut u64) -> Result<(), Box<dyn Error>> {
    let mut new_block = Block::from_block_info(block_info);

    stdout.execute(MoveTo(0, 0))?;
    stdout.execute(PrintStyledContent("*** RUST ***".white()))?;

    stdout.execute(MoveTo(0, 1))?;
    stdout.execute(PrintStyledContent(format!("Total mined: {}\n", total_mined).yellow()))?;

    stdout.execute(MoveTo(0, 3))?;
    stdout.execute(PrintStyledContent(format!("Mining block index: {}\n", new_block.index).white()))?;
    
    new_block.mine(new_block.difficulty, "allankey");

    stdout.execute(MoveTo(0, 4))?;
    stdout.execute(PrintStyledContent("Block mined! Sending to blockchain...\n".green()))?;
    stdout.flush()?;

    client.post(format!("{}/blocks/", BLOCKCHAIN_SERVER))
        .json(&new_block.to_dict())
        .send()
        .await?;

    *total_mined += 1;

    
    stdout.flush()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut stdout = io::stdout();
    let mut total_mined = 0;

    terminal::enable_raw_mode()?;

    loop {
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        stdout.execute(MoveTo(0, 0))?;
        stdout.execute(PrintStyledContent("### RUST ###\n".yellow()))?;
        stdout.execute(MoveTo(0, 1))?;
        stdout.execute(PrintStyledContent(format!("Total mined: {}\n", total_mined).yellow()))?;
        stdout.execute(MoveTo(0, 2))?;
        stdout.execute(PrintStyledContent("Fetching next block info...\n".yellow()))?;
        stdout.flush()?;

        match get_next_block_info(&client).await {
            Ok(block_info) => {
                if let Err(e) = mine_block(&mut stdout, &client, block_info, &mut total_mined).await {
                    stdout.execute(MoveTo(0, 3))?;
                    stdout.execute(PrintStyledContent(format!("Error: {}\n", e).red()))?;
                    stdout.flush()?;
                }
            }
            Err(e) => {
                stdout.execute(MoveTo(0, 3))?;
                stdout.execute(PrintStyledContent(format!("Error fetching block info: {}\n", e).red()))?;
                stdout.flush()?;
            }
        }

        sleep(Duration::from_secs(1)).await;
    }

    terminal::disable_raw_mode()?;
    Ok(())
}
