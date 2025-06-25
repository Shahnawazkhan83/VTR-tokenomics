use anchor_lang::prelude::*;
use crate::*;

#[derive(Accounts)]
pub struct CreateProposal<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    #[account(
        init,
        payer = proposer,
        space = 8 + Proposal::LEN,
        seeds = [b"proposal", proposer.key().as_ref(), &[governance.proposal_count as u8]],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    
    #[account(
        mut,
        seeds = [b"governance"],
        bump = governance.bump,
    )]
    pub governance: Account<'info, Governance>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"proposal", proposal.proposer.as_ref(), &[proposal.id as u8]],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
    
    #[account(
        init_if_needed,
        payer = voter,
        space = 8 + Vote::LEN,
        seeds = [b"vote", proposal.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote: Account<'info, Vote>,
    
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Governance {
    pub authority: Pubkey,
    pub proposal_count: u64,
    pub min_threshold: u64,
    pub voting_period: i64,
    pub execution_delay: i64,
    pub bump: u8,
}

impl Governance {
    pub const LEN: usize = 32 + 8 + 8 + 8 + 8 + 1;
}

#[account]
pub struct Proposal {
    pub id: u64,
    pub proposer: Pubkey,
    pub title: String,
    pub description: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub executed: bool,
    pub bump: u8,
}

impl Proposal {
    pub const LEN: usize = 8 + 32 + 100 + 500 + 8 + 8 + 8 + 8 + 1 + 1;
}

#[account]
pub struct Vote {
    pub voter: Pubkey,
    pub proposal: Pubkey,
    pub vote_type: VoteType,
    pub weight: u64,
    pub bump: u8,
}

impl Vote {
    pub const LEN: usize = 32 + 32 + 1 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum VoteType {
    For,
    Against,
}