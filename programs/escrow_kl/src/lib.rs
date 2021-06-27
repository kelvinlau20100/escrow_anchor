use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer, SetAuthority};
use spl_token::instruction::AuthorityType;


#[program]
pub mod escrow_kl {
    use super::*;

    pub fn init_escrow(
        ctx: Context<InitEscrow>,
        amount: u64,
    ) -> ProgramResult {
        // Initialise the escrow transaction
        let escrow_acc = &mut ctx.accounts.escrow_acc;

        escrow_acc.is_initialized = true;
        escrow_acc.initializer_pubkey = *ctx.accounts.initializer_acc.key;
        escrow_acc.temp_token_acc_pubkey = *ctx.accounts.temp_token_acc.to_account_info().key;
        escrow_acc.initializer_token_to_receive_acc_pubkey = *ctx.accounts.token_to_rx_acc.to_account_info().key;
        escrow_acc.expected_amount = amount;
        let (pda, _nonce) = Pubkey::find_program_address(&[b"escrow"], ctx.program_id);
        token::set_authority(ctx.accounts.into(), AuthorityType::AccountOwner, Some(pda))?;

        Ok(())
    }

    pub fn exchange(
        ctx: Context<Exchange>,
        amount_expected_by_taker: u64,
    ) -> ProgramResult {
        // Transfer to initializer

        // Constraints checking
        if ctx.accounts.escrow_acc.expected_amount != ctx.accounts.taker_token_acc_y.amount {
            return Err(ErrorCode::ExpectedAmountMismatch.into());
        }

        if amount_expected_by_taker != ctx.accounts.initializer_token_acc_x.amount {
            return Err(ErrorCode::ExpectedAmountMismatch.into());
        }

        if ctx.accounts.escrow_acc.temp_token_acc_pubkey != *ctx.accounts.initializer_token_acc_x.to_account_info().key {
            return Err(ErrorCode::InvalidAccount.into());
        }

        if ctx.accounts.escrow_acc.initializer_pubkey != *ctx.accounts.initializer_main_acc.key {
            return Err(ErrorCode::InvalidAccount.into());
        }

        if ctx.accounts.escrow_acc.initializer_token_to_receive_acc_pubkey != *ctx.accounts.initializer_token_acc_y.to_account_info().key {
            return Err(ErrorCode::InvalidAccount.into());
        }

        let (_pda, bump_seed) = Pubkey::find_program_address(&[b"escrow"], ctx.program_id);
        let signer_seeds = &[&b"escrow"[..], &[bump_seed]];

        token::transfer(
            ctx.accounts.into_transfer_to_initializer(), 
            ctx.accounts.escrow_acc.expected_amount,
        )?;

        token::transfer(
            ctx.accounts.into_transfer_to_taker().with_signer(&[&signer_seeds[..]]),
            amount_expected_by_taker,
        )?;
        
        token::set_authority(ctx.accounts.into_set_auth_to_initializer().with_signer(&[&signer_seeds[..]]), 
            AuthorityType::AccountOwner, 
            Some(*ctx.accounts.initializer_main_acc.key),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitEscrow<'info> {
    // The account of the person initializing the escrow
    #[account(signer)]
    initializer_acc: AccountInfo<'info>,
    // Temporary token account that should be created prior to this instruction and owned by the initializer
    #[account(mut)]
    temp_token_acc: CpiAccount<'info, TokenAccount>,
    // The initializer's token account for the token they will receive should the trade go through
    token_to_rx_acc: CpiAccount<'info, TokenAccount>,
    // The escrow account, it will hold all necessary info about the trade.
    #[account(init)]
    escrow_acc: ProgramAccount<'info, EscrowAcc>,
    // The rent sysvar
    pub rent: Sysvar<'info, Rent>,
    // The token program
    token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    // The account of the person taking the trade
    #[account(signer)]
    taker_acc: AccountInfo<'info>,
    // Taker token account for token they will send
    #[account(mut)]
    taker_token_acc_y: CpiAccount<'info, TokenAccount>,
    // Taker token account for token they will receive
    #[account(mut)]
    taker_token_acc_x: CpiAccount<'info, TokenAccount>,
    // The PDA's temp token account from init_escrow to get Token X from
    #[account(mut)]
    initializer_token_acc_x: CpiAccount<'info, TokenAccount>,
    // Initializers main account
    #[account(mut)]
    initializer_main_acc: AccountInfo<'info>,
    // Initializer token account for token they will receive
    #[account(mut)]
    initializer_token_acc_y: CpiAccount<'info, TokenAccount>,
    //Escrow Account
    #[account(mut, close=initializer_main_acc)]
    escrow_acc: ProgramAccount<'info, EscrowAcc>,
    // Token Program
    token_program: AccountInfo<'info>,
    // PDA Account
    #[account(mut)]
    pda_acc: AccountInfo<'info>,  
}


#[account]
pub struct EscrowAcc {
    pub is_initialized: bool,
    pub initializer_pubkey: Pubkey,
    pub temp_token_acc_pubkey: Pubkey,
    pub initializer_token_to_receive_acc_pubkey: Pubkey,
    pub expected_amount: u64,
}

impl<'a, 'b, 'c, 'info> From<&mut InitEscrow<'info>> 
    for CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>>
{
    fn from(
        accounts: &mut InitEscrow<'info>,
    ) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: accounts.temp_token_acc.to_account_info().clone(),
            current_authority: accounts.initializer_acc.clone(),
        };
        let cpi_program = accounts.token_program.clone();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'a, 'b, 'c, 'info> Exchange<'info> {
    fn into_transfer_to_initializer (&self) -> CpiContext<'a, 'b, 'c, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_token_acc_y.to_account_info().clone(),
            to: self.initializer_token_acc_y.to_account_info().clone(),
            authority: self.taker_acc.clone(),
        };
        let cpi_program = self.token_program.clone();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'a, 'b, 'c, 'info> Exchange<'info> {
    fn into_transfer_to_taker (&self) -> CpiContext<'a, 'b, 'c, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.initializer_token_acc_x.to_account_info().clone(),
            to: self.taker_token_acc_x.to_account_info().clone(),
            authority: self.pda_acc.clone(),
        };
        let cpi_program = self.token_program.clone();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'a, 'b, 'c, 'info> Exchange<'info> {
    fn into_set_auth_to_initializer (&self) -> CpiContext<'a, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.initializer_token_acc_x.to_account_info().clone(),
            current_authority: self.pda_acc.clone(),
        };
        let cpi_program = self.token_program.clone();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[error]
pub enum ErrorCode {
    #[msg("Invalid Account Data")]
    InvalidAccount,
    #[msg("Expected Amount Mismatch")]
    ExpectedAmountMismatch,
}