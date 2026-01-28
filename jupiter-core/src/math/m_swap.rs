/// Ported from https://github.com/m0-foundation/solana-m-extensions
use crate::errors::{math_overflow, math_underflow};
use anyhow::Result;
use spl_token_2022::extension::scaled_ui_amount::ScaledUiAmountConfig;

const INDEX_SCALE_F64: f64 = 1e12f64;
const INDEX_SCALE_U64: u64 = 1_000_000_000_000u64;

pub fn amount_to_principal_down(amount: u64, index: u64) -> Result<u64> {
    // If the index is 1, return the amount directly
    if index == INDEX_SCALE_U64 {
        return Ok(amount);
    }

    // Calculate the principal from the amount and index, rounding down
    let principal: u64 = (amount as u128)
        .checked_mul(INDEX_SCALE_U64 as u128)
        .ok_or(math_overflow())?
        .checked_div(index as u128)
        .ok_or(math_underflow())?
        .try_into()?;

    Ok(principal)
}

pub fn amount_to_principal_up(amount: u64, index: u64) -> Result<u64> {
    // If the index is 1, return the amount directly
    if index == INDEX_SCALE_U64 {
        return Ok(amount);
    }

    // Calculate the principal from the amount and index, rounding up
    let index_128 = index as u128;
    let principal: u64 = (amount as u128)
        .checked_mul(INDEX_SCALE_U64 as u128)
        .ok_or(math_overflow())?
        .checked_add(index_128.checked_sub(1u128).ok_or(math_underflow())?)
        .ok_or(math_overflow())?
        .checked_div(index_128)
        .ok_or(math_underflow())?
        .try_into()?;

    Ok(principal)
}

pub fn principal_to_amount_down(principal: u64, index: u64) -> Result<u64> {
    // If the index is 1, return the principal directly
    if index == INDEX_SCALE_U64 {
        return Ok(principal);
    }

    // Calculate the amount from the principal and index, rounding down
    let amount: u64 = (index as u128)
        .checked_mul(principal as u128)
        .ok_or(math_overflow())?
        .checked_div(INDEX_SCALE_U64 as u128)
        .ok_or(math_underflow())?
        .try_into()?;

    Ok(amount)
}

pub fn principal_to_amount_up(principal: u64, index: u64) -> Result<u64> {
    // If the index is 1, return the principal directly
    if index == INDEX_SCALE_U64 {
        return Ok(principal);
    }

    // Calculate the amount from the principal and index, rounding up
    let amount: u64 = (index as u128)
        .checked_mul(principal as u128)
        .ok_or(math_overflow())?
        .checked_add(
            (INDEX_SCALE_U64 as u128)
                .checked_sub(1u128)
                .ok_or(math_underflow())?,
        )
        .ok_or(math_overflow())?
        .checked_div(INDEX_SCALE_U64 as u128)
        .ok_or(math_underflow())?
        .try_into()?;

    Ok(amount)
}

pub fn multiplier_to_index(multiplier: f64) -> Result<u64> {
    let index: f64 = (INDEX_SCALE_F64 * multiplier).trunc();

    if index < 0.0 {
        Err(math_underflow())
    } else if index > u64::MAX as f64 {
        Err(math_overflow())
    } else {
        // Convert the f64 index to u64
        Ok(index as u64)
    }
}

pub fn index_to_multiplier(index: u64) -> Result<f64> {
    Ok(index as f64 / INDEX_SCALE_F64)
}

/// Emulate MExt -> M0 token unwrapping.
///
/// - assumes that MExt token has no scaled UI token extension;
pub fn unwrap_amount(ext_principal: u64, m_scaled_ui_config: &ScaledUiAmountConfig) -> Result<u64> {
    let m_index: u64 = multiplier_to_index(m_scaled_ui_config.new_multiplier.into())?;
    let m_principal: u64 = amount_to_principal_down(
        principal_to_amount_down(ext_principal, INDEX_SCALE_U64)?,
        m_index,
    )?;

    Ok(m_principal)
}

/// Emulate M0 -> MExt token wrapping.
///
/// - assumes that MExt token has no scaled UI token extension;
pub fn wrap_amount(m_principal: u64, m_scaled_ui_config: &ScaledUiAmountConfig) -> Result<u64> {
    let m_index = multiplier_to_index(m_scaled_ui_config.new_multiplier.into())?;
    let ext_principal = amount_to_principal_down(
        principal_to_amount_down(m_principal, m_index)?,
        INDEX_SCALE_U64,
    )?;

    Ok(ext_principal)
}

/// Swap 1 ext token into another ext token.
///
/// - in the program swap is done via wrapping/unwrapping to m0, so this function emulates this process;
/// - assumes that both ext tokens have no scaled ui config;
pub fn swap_ext_amounts(in_amount: u64, m_scaled_ui_config: &ScaledUiAmountConfig) -> Result<u64> {
    let m_amount = unwrap_amount(in_amount, m_scaled_ui_config)?;
    let out_amount = wrap_amount(m_amount, m_scaled_ui_config)?;
    Ok(out_amount)
}
