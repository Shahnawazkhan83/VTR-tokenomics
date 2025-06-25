use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("2jYy4kkMB6hTj9uZCDhCPqUyWaMBRRmZDTjW8rET9kD6");

#[program]
pub mod vtr_token {
    use super::*;

    pub fn initialize_token(
        ctx: Context<InitializeToken>,
        total_supply: u64,
    ) -> Result<()> {
        let token_data = &mut ctx.accounts.token_data;
        token_data.authority = ctx.accounts.authority.key();
        token_data.mint = ctx.accounts.mint.key();
        token_data.total_supply = total_supply;
        token_data.circulating_supply = 0;
        token_data.burned_supply = 0;
        token_data.bump = ctx.bumps.token_data;
        
        Ok(())
    }

    pub fn mint_tokens(
        ctx: Context<MintTokens>,
        amount: u64,
        allocation_type: AllocationType,
    ) -> Result<()> {
        // Validate minting doesn't exceed total supply
        require!(
            ctx.accounts.token_data.circulating_supply + amount <= ctx.accounts.token_data.total_supply,
            ErrorCode::ExceedsTotalSupply
        );

        // Calculate TGE unlock amount
        let tge_unlock_percentage = get_tge_unlock(&allocation_type);
        let tge_amount = (amount * tge_unlock_percentage as u64) / 10000;
        
        // Create allocation record
        let allocation = &mut ctx.accounts.allocation;
        allocation.recipient = ctx.accounts.recipient.key();
        allocation.amount = amount;
        allocation.allocation_type = allocation_type.clone();
        allocation.vesting_start = Clock::get()?.unix_timestamp;
        allocation.cliff_duration = get_cliff_duration(&allocation_type);
        allocation.vesting_duration = get_vesting_duration(&allocation_type);
        allocation.tge_unlock_percentage = tge_unlock_percentage;
        allocation.claimed_amount = 0;
        allocation.bump = ctx.bumps.allocation;

        if tge_amount > 0 {
            // Store values needed for seeds before borrowing
            let mint_key = ctx.accounts.mint.key();
            let token_data_bump = ctx.accounts.token_data.bump;
            
            let seeds = &[
                b"token_data".as_ref(),
                mint_key.as_ref(),
                &[token_data_bump],
            ];
            let signer = &[&seeds[..]];
            
            let cpi_accounts = MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.token_data.to_account_info(),
            };
            
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            
            token::mint_to(cpi_ctx, tge_amount)?;
            
            allocation.claimed_amount = tge_amount;
        }

        // Update token data (mutable borrow at the end)
        let token_data = &mut ctx.accounts.token_data;
        token_data.circulating_supply += tge_amount;
        
        Ok(())
    }

    pub fn claim_vested_tokens(ctx: Context<ClaimVestedTokens>) -> Result<()> {
        let allocation = &mut ctx.accounts.allocation;
        let current_time = Clock::get()?.unix_timestamp;
        
        let claimable_amount = calculate_claimable_amount(allocation, current_time)?;
        
        require!(claimable_amount > 0, ErrorCode::NoTokensToClaim);

        // Store values needed for seeds before any CPI calls
        let mint_key = ctx.accounts.mint.key();
        let token_data_bump = ctx.accounts.token_data.bump;
        
        let seeds = &[
            b"token_data".as_ref(),
            mint_key.as_ref(),
            &[token_data_bump],
        ];
        let signer = &[&seeds[..]];
        
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.token_data.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        
        token::mint_to(cpi_ctx, claimable_amount)?;
        
        allocation.claimed_amount += claimable_amount;
        
        // Update token data (mutable borrow at the end)
        let token_data = &mut ctx.accounts.token_data;
        token_data.circulating_supply += claimable_amount;
        
        Ok(())
    }

    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        // Transfer tokens to burn account
        let cpi_accounts = Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.burn_vault.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, amount)?;
        
        let token_data = &mut ctx.accounts.token_data;
        token_data.burned_supply += amount;
        token_data.circulating_supply -= amount;
        
        Ok(())
    }

    pub fn initialize_staking(
        ctx: Context<InitializeStaking>,
        apy_percentage: u16,
        min_stake_duration: i64,
    ) -> Result<()> {
        let staking_pool = &mut ctx.accounts.staking_pool;
        staking_pool.authority = ctx.accounts.authority.key();
        staking_pool.apy_percentage = apy_percentage;
        staking_pool.min_stake_duration = min_stake_duration;
        staking_pool.total_staked = 0;
        staking_pool.bump = ctx.bumps.staking_pool;
        
        Ok(())
    }

    pub fn stake_tokens(
        ctx: Context<StakeTokens>,
        amount: u64,
        duration: i64,
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(
            duration >= ctx.accounts.staking_pool.min_stake_duration,
            ErrorCode::InsufficientStakeDuration
        );

        // Transfer tokens to staking vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.staking_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, amount)?;

        // Create stake account
        let stake_account = &mut ctx.accounts.stake_account;
        stake_account.user = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.stake_time = Clock::get()?.unix_timestamp;
        stake_account.unlock_time = stake_account.stake_time + duration;
        stake_account.claimed_rewards = 0;
        stake_account.bump = ctx.bumps.stake_account;

        // Update staking pool
        let staking_pool = &mut ctx.accounts.staking_pool;
        staking_pool.total_staked += amount;
        
        Ok(())
    }

    pub fn unstake_tokens(ctx: Context<UnstakeTokens>) -> Result<()> {
        let stake_account = &ctx.accounts.stake_account;
        let current_time = Clock::get()?.unix_timestamp;
        
        require!(
            current_time >= stake_account.unlock_time,
            ErrorCode::StakingPeriodNotEnded
        );

        // Calculate pending rewards
        let pending_rewards = calculate_staking_rewards(
            stake_account.amount,
            ctx.accounts.staking_pool.apy_percentage,
            stake_account.stake_time,
            current_time,
        );

        // Store values needed for seeds before any CPI calls
        let authority_key = ctx.accounts.staking_pool.authority;
        let staking_pool_bump = ctx.accounts.staking_pool.bump;
        let stake_amount = stake_account.amount;

        // Transfer staked tokens back
        let seeds = &[
            b"staking_pool".as_ref(),
            authority_key.as_ref(),
            &[staking_pool_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.staking_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.staking_pool.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        
        token::transfer(cpi_ctx, stake_amount)?;

        // Mint rewards if any
        if pending_rewards > 0 {
            let mint_key = ctx.accounts.mint.key();
            let token_data_bump = ctx.accounts.token_data.bump;
            
            let seeds = &[
                b"token_data".as_ref(),
                mint_key.as_ref(),
                &[token_data_bump],
            ];
            let signer = &[&seeds[..]];
            
            let cpi_accounts = MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.token_data.to_account_info(),
            };
            
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            
            token::mint_to(cpi_ctx, pending_rewards)?;
        }

        // Update staking pool (mutable borrow at the end)
        let staking_pool = &mut ctx.accounts.staking_pool;
        staking_pool.total_staked -= stake_amount;
        
        Ok(())
    }
}

// Helper functions
fn get_cliff_duration(allocation_type: &AllocationType) -> i64 {
    match allocation_type {
        AllocationType::TokenSale => 0,
        AllocationType::TeamAdvisors => 12 * 30 * 24 * 3600, // 12 months
        AllocationType::EcosystemGrowth => 0,
        AllocationType::Liquidity => 0,
        AllocationType::PlatformReserve => 6 * 30 * 24 * 3600, // 6 months
        AllocationType::BuybackBurn => 0,
        AllocationType::Marketing => 0,
    }
}

fn get_vesting_duration(allocation_type: &AllocationType) -> i64 {
    match allocation_type {
        AllocationType::TokenSale => 12 * 30 * 24 * 3600, // 12 months
        AllocationType::TeamAdvisors => 36 * 30 * 24 * 3600, // 36 months
        AllocationType::EcosystemGrowth => 48 * 30 * 24 * 3600, // 48 months
        AllocationType::Liquidity => 6 * 30 * 24 * 3600, // 6 months
        AllocationType::PlatformReserve => 60 * 30 * 24 * 3600, // 60 months
        AllocationType::BuybackBurn => 24 * 30 * 24 * 3600, // 24 months
        AllocationType::Marketing => 18 * 30 * 24 * 3600, // 18 months
    }
}

fn get_tge_unlock(allocation_type: &AllocationType) -> u16 {
    match allocation_type {
        AllocationType::TokenSale => 1000, // 10%
        AllocationType::TeamAdvisors => 0, // 0%
        AllocationType::EcosystemGrowth => 1500, // 15%
        AllocationType::Liquidity => 5000, // 50%
        AllocationType::PlatformReserve => 500, // 5%
        AllocationType::BuybackBurn => 0, // 0%
        AllocationType::Marketing => 2000, // 20%
    }
}

fn calculate_claimable_amount(allocation: &TokenAllocation, current_time: i64) -> Result<u64> {
    let vesting_start = allocation.vesting_start + allocation.cliff_duration;
    
    if current_time < vesting_start {
        return Ok(0);
    }
    
    let elapsed_time = current_time - vesting_start;
    let vesting_progress = std::cmp::min(
        elapsed_time,
        allocation.vesting_duration,
    ) as u64;
    
    let total_vested = if allocation.vesting_duration == 0 {
        allocation.amount
    } else {
        allocation.amount * vesting_progress / (allocation.vesting_duration as u64)
    };
    
    let claimable = total_vested.saturating_sub(allocation.claimed_amount);
    Ok(claimable)
}

fn calculate_staking_rewards(
    amount: u64,
    apy_percentage: u16,
    stake_time: i64,
    current_time: i64,
) -> u64 {
    let duration_seconds = (current_time - stake_time) as u64;
    let duration_years = duration_seconds as f64 / (365.25 * 24.0 * 3600.0);
    let apy = apy_percentage as f64 / 10000.0;
    
    let rewards = (amount as f64 * apy * duration_years) as u64;
    rewards
}

// Account validation structs
#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = token_data,
    )]
    pub mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + TokenData::LEN,
        seeds = [b"token_data", mint.key().as_ref()],
        bump
    )]
    pub token_data: Account<'info, TokenData>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [b"token_data", mint.key().as_ref()],
        bump = token_data.bump,
        has_one = authority,
    )]
    pub token_data: Account<'info, TokenData>,
    
    /// CHECK: Recipient can be any account
    pub recipient: AccountInfo<'info>,
    
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + TokenAllocation::LEN,
        seeds = [b"allocation", recipient.key().as_ref()],
        bump
    )]
    pub allocation: Account<'info, TokenAllocation>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimVestedTokens<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,
    
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [b"token_data", mint.key().as_ref()],
        bump = token_data.bump,
    )]
    pub token_data: Account<'info, TokenData>,
    
    #[account(
        mut,
        seeds = [b"allocation", recipient.key().as_ref()],
        bump = allocation.bump,
        has_one = recipient,
    )]
    pub allocation: Account<'info, TokenAllocation>,
    
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = recipient,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [b"token_data", mint.key().as_ref()],
        bump = token_data.bump,
    )]
    pub token_data: Account<'info, TokenData>,
    
    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,
    
    #[account(
        init_if_needed,
        payer = authority,
        token::mint = mint,
        token::authority = token_data,
        seeds = [b"burn_vault", mint.key().as_ref()],
        bump
    )]
    pub burn_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeStaking<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + StakingPool::LEN,
        seeds = [b"staking_pool", authority.key().as_ref()],
        bump
    )]
    pub staking_pool: Account<'info, StakingPool>,
    
    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = staking_pool,
        seeds = [b"staking_vault", authority.key().as_ref()],
        bump
    )]
    pub staking_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub mint: Account<'info, Mint>,
    
    #[account(
        mut,
        seeds = [b"staking_pool", staking_pool.authority.as_ref()],
        bump = staking_pool.bump,
    )]
    pub staking_pool: Account<'info, StakingPool>,
    
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"staking_vault", staking_pool.authority.as_ref()],
        bump
    )]
    pub staking_vault: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer = user,
        space = 8 + StakeAccount::LEN,
        seeds = [b"stake_account", user.key().as_ref()],
        bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UnstakeTokens<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    
    #[account(
        seeds = [b"token_data", mint.key().as_ref()],
        bump = token_data.bump,
    )]
    pub token_data: Account<'info, TokenData>,
    
    #[account(
        mut,
        seeds = [b"staking_pool", staking_pool.authority.as_ref()],
        bump = staking_pool.bump,
    )]
    pub staking_pool: Account<'info, StakingPool>,
    
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"staking_vault", staking_pool.authority.as_ref()],
        bump
    )]
    pub staking_vault: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"stake_account", user.key().as_ref()],
        bump = stake_account.bump,
        has_one = user,
        close = user
    )]
    pub stake_account: Account<'info, StakeAccount>,
    
    pub token_program: Program<'info, Token>,
}

// Data structures
#[account]
pub struct TokenData {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_supply: u64,
    pub circulating_supply: u64,
    pub burned_supply: u64,
    pub bump: u8,
}

impl TokenData {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 1;
}

#[account]
pub struct TokenAllocation {
    pub recipient: Pubkey,
    pub amount: u64,
    pub allocation_type: AllocationType,
    pub vesting_start: i64,
    pub cliff_duration: i64,
    pub vesting_duration: i64,
    pub tge_unlock_percentage: u16,
    pub claimed_amount: u64,
    pub bump: u8,
}

impl TokenAllocation {
    pub const LEN: usize = 32 + 8 + 1 + 8 + 8 + 8 + 2 + 8 + 1;
}

#[account]
pub struct StakingPool {
    pub authority: Pubkey,
    pub apy_percentage: u16,
    pub min_stake_duration: i64,
    pub total_staked: u64,
    pub bump: u8,
}

impl StakingPool {
    pub const LEN: usize = 32 + 2 + 8 + 8 + 1;
}

#[account]
pub struct StakeAccount {
    pub user: Pubkey,
    pub amount: u64,
    pub stake_time: i64,
    pub unlock_time: i64,
    pub claimed_rewards: u64,
    pub bump: u8,
}

impl StakeAccount {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 8 + 1;
}

// Enums
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum AllocationType {
    TokenSale,
    TeamAdvisors,
    EcosystemGrowth,
    Liquidity,
    PlatformReserve,
    BuybackBurn,
    Marketing,
}

// Error codes
#[error_code]
pub enum ErrorCode {
    #[msg("Amount exceeds total supply")]
    ExceedsTotalSupply,
    #[msg("No tokens to claim")]
    NoTokensToClaim,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Insufficient stake duration")]
    InsufficientStakeDuration,
    #[msg("Staking period has not ended")]
    StakingPeriodNotEnded,
}