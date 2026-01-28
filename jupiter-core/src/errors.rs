use anyhow::anyhow;
use solana_sdk::{program_pack::Pack, pubkey, pubkey::Pubkey};

pub(crate) fn unsupported_input_mint(mint: &Pubkey) -> anyhow::Error {
    anyhow!("Input mint {mint} is not supported")
}

pub(crate) fn unsupported_output_mint(mint: &Pubkey) -> anyhow::Error {
    anyhow!("Output mint {mint} is not supported")
}

pub(crate) fn math_overflow() -> anyhow::Error {
    anyhow!("Math overflow")
}

pub(crate) fn math_underflow() -> anyhow::Error {
    anyhow!("Math underflow")
}
