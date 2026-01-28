use crate::{
    amm::*,
    constants::{M_MINT, USD_PLUS_MINT, WM_MINT},
    errors::{unsupported_input_mint, unsupported_output_mint},
    math::m_swap::swap_ext_amounts,
};
use anchor_lang::{ToAccountMetas, prelude::AccountDeserialize};
use anyhow::{anyhow, Context, Result};
use jupiter_amm_interface::{try_get_account_data, AccountMap, AmmContext};
use program_interfaces::{
    m_ext::constants::{EXT_GLOBAL_SEED, MINT_AUTHORITY_SEED, M_VAULT_SEED},
    m_swap::{
        accounts::SwapGlobal, client::accounts::Swap as SwapAccounts, constants::GLOBAL_SEED,
        ID as M_SWAP_ID,
    },
};
use solana_sdk::{pubkey, pubkey::Pubkey};
use solana_sdk_ids::system_program;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::{
    extension::{
        scaled_ui_amount::ScaledUiAmountConfig, BaseStateWithExtensions, StateWithExtensions,
    },
    state::Mint,
};

#[derive(Clone)]
pub struct UsdPlusAmm {
    key: Pubkey,
    label: String,
    program_id: Pubkey,
    swap_global: SwapGlobal,
    m_zero_scaled_ui: Option<ScaledUiAmountConfig>,
}

impl UsdPlusAmm {
    pub const WM_EXT_PROGRAM_ID: Pubkey = pubkey!("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko");
    pub const USD_PLUS_EXT_PROGRAM_ID: Pubkey =
        pubkey!("extUkDFf3HLekkxbcZ3XRUizMjbxMJgKBay3p9xGVmg");

    pub fn get_program_id_for_mint(mint: &Pubkey) -> Result<Pubkey> {
        match mint {
            &WM_MINT => Ok(Self::WM_EXT_PROGRAM_ID),
            &USD_PLUS_MINT => Ok(Self::USD_PLUS_EXT_PROGRAM_ID),
            _ => Err(anyhow!("Unexpected mint {}", mint.to_string())),
        }
    }

    pub fn get_token_program_id_for_mint(mint: &Pubkey) -> Result<Pubkey> {
        match mint {
            &WM_MINT => Ok(spl_token_2022::ID),
            &USD_PLUS_MINT => Ok(spl_token::ID),
            _ => Err(anyhow!("Unexpected mint {}", mint.to_string())),
        }
    }

    pub fn find_swap_global_pubkey() -> Pubkey {
        Pubkey::find_program_address(&[GLOBAL_SEED], &M_SWAP_ID).0
    }

    pub fn find_ext_global_pubkey(program_id: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[EXT_GLOBAL_SEED], program_id).0
    }

    pub fn find_ext_vault_authority(program_id: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[M_VAULT_SEED], program_id).0
    }

    pub fn find_ext_mint_authority(program_id: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], program_id).0
    }

    pub fn find_m_vault_pubkey(authority: &Pubkey) -> Pubkey {
        get_associated_token_address_with_program_id(authority, &M_MINT, &spl_token_2022::id())
    }

    pub fn is_allowed_mint(mint: &Pubkey) -> bool {
        WM_MINT.eq(mint) || USD_PLUS_MINT.eq(mint)
    }
}

impl AmmProgramIdToLabel for UsdPlusAmm {
    const PROGRAM_ID_TO_LABELS: &[(Pubkey, AmmLabel)] = &[(M_SWAP_ID, "M Swap")];
}

impl Amm for UsdPlusAmm {
    fn from_keyed_account(
        keyed_account: &KeyedAccount,
        _amm_context: &AmmContext,
    ) -> Result<Self> {
        let mut data: &[u8] = &keyed_account.account.data;
        let swap_global = SwapGlobal::try_deserialize(&mut data)?;

        Ok(Self {
            key: Self::find_swap_global_pubkey(),
            label: String::from("M Swap"),
            program_id: M_SWAP_ID,
            swap_global,
            m_zero_scaled_ui: None,
        })
    }

    fn label(&self) -> String {
        self.label.clone()
    }

    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        vec![WM_MINT, USD_PLUS_MINT]
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        vec![M_MINT]
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let m_zero_mint = try_get_account_data(account_map, &M_MINT)?;
        let state = StateWithExtensions::<Mint>::unpack(m_zero_mint)?;
        let scaled_ui = state.get_extension::<ScaledUiAmountConfig>().ok().copied();

        self.m_zero_scaled_ui = scaled_ui;

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        if !UsdPlusAmm::is_allowed_mint(&quote_params.input_mint) {
            return Err(unsupported_input_mint(&quote_params.input_mint));
        }
        if !UsdPlusAmm::is_allowed_mint(&quote_params.output_mint) {
            return Err(unsupported_output_mint(&quote_params.output_mint));
        }

        let m_scaled_ui_config = self
            .m_zero_scaled_ui
            .as_ref()
            .context("Expected Scaled UI to be initialized")?;
        let out_amount = swap_ext_amounts(quote_params.amount, m_scaled_ui_config)?;

        Ok(Quote {
            in_amount: quote_params.amount,
            out_amount,
            ..Default::default()
        })
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let from_program_id = Self::get_program_id_for_mint(&swap_params.source_mint)?;
        let to_program_id = Self::get_program_id_for_mint(&swap_params.destination_mint)?;
        let swap_global = Self::find_swap_global_pubkey();
        let from_m_vault_auth = Self::find_ext_vault_authority(&from_program_id);
        let to_m_vault_auth = Self::find_ext_vault_authority(&to_program_id);

        Ok(SwapAndAccountMetas {
            // TODO: add MSwap program to Enum
            swap: Swap::TokenSwap,
            account_metas: SwapAccounts {
                signer: swap_params.token_transfer_authority,
                wrap_authority: None,
                unwrap_authority: None,
                swap_global,
                from_global: Self::find_ext_global_pubkey(&from_program_id),
                to_global: Self::find_ext_global_pubkey(&to_program_id),
                from_mint: swap_params.source_mint,
                to_mint: swap_params.destination_mint,
                m_mint: M_MINT,
                from_token_account: swap_params.source_token_account,
                to_token_account: swap_params.destination_token_account,
                swap_m_account: Self::find_m_vault_pubkey(&swap_global),
                from_m_vault_auth,
                to_m_vault_auth,
                from_mint_authority: Self::find_ext_mint_authority(&from_program_id),
                to_mint_authority: Self::find_ext_mint_authority(&to_program_id),
                from_m_vault: Self::find_m_vault_pubkey(&from_m_vault_auth),
                to_m_vault: Self::find_m_vault_pubkey(&to_m_vault_auth),
                from_token_program: Self::get_token_program_id_for_mint(&swap_params.source_mint)?,
                to_token_program: Self::get_token_program_id_for_mint(
                    &swap_params.destination_mint,
                )?,
                m_token_program: spl_token_2022::ID,
                from_ext_program: from_program_id,
                to_ext_program: to_program_id,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn get_accounts_len(&self) -> usize {
        24
    }
}
