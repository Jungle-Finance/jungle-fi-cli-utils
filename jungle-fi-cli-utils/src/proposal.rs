use serum_multisig::TransactionAccount;
use anchor_client::anchor_lang::{AnchorDeserialize, AnchorSerialize};
use std::str::FromStr;
use solana_program::instruction::Instruction;
use solana_program::pubkey::Pubkey;
use solana_sdk::bs58;

/// Print a vec of instructions as proposals, each with a paired name.
pub fn print_proposals(ixs: &Vec<Instruction>, names: &Vec<&str>) -> anyhow::Result<()> {
    let mut count: usize = 0;
    ixs
        .iter()
        .zip(names)
        .for_each(|(ix, name)| {
            println!("");
            println!("Instruction {}: {}", count, name);
            println!("{}", Proposal::from(ix).to_string());
            count += 1;
        });
    Ok(())
}

/// Multisig transaction proposal
pub struct Proposal {
    pub pid: Pubkey,
    pub accs: Vec<TransactionAccount>,
    pub data: Vec<u8>,
}

impl From<&Proposal> for serum_multisig::instruction::CreateTransaction {
    fn from(proposal: &Proposal) -> Self {
        serum_multisig::instruction::CreateTransaction {
            pid: proposal.pid.clone(),
            accs: proposal.accs.clone(),
            data: proposal.data.clone(),
        }
    }
}

impl From<&serum_multisig::instruction::CreateTransaction> for Proposal {
    fn from(ix: &serum_multisig::instruction::CreateTransaction) -> Self {
        Self {
            pid: ix.pid.clone(),
            accs: ix.accs.clone(),
            data: ix.data.clone(),
        }
    }
}

/// Base58 encoded as the actual multisig program instruction data.
/// This makes it ready to throw into a [create_transaction] instruction
/// without any modification.
impl ToString for Proposal {
    fn to_string(&self) -> String {
        let mut buf = Vec::new();
        serum_multisig::instruction::CreateTransaction::from(self)
            .serialize(&mut buf)
            .unwrap();
        bs58::encode(buf.as_slice()).into_string()
    }
}

impl FromStr for Proposal {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let decoded = bs58::decode(s).into_vec()?;
        let ix = serum_multisig::instruction::CreateTransaction::deserialize(&mut decoded.as_slice())?;
        Ok(Proposal::from(&ix))
    }
}

impl From<&Instruction> for Proposal {
    fn from(ix: &Instruction) -> Self {
        Self {
            pid: ix.program_id,
            accs: ix.accounts
                .iter()
                .map(|acc| TransactionAccount::from(acc))
                .collect(),
            data: ix.data.clone(),
        }
    }
}

impl From<Instruction> for Proposal {
    fn from(ix: Instruction) -> Self {
        Proposal::from(&ix)
    }
}
