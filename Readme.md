# VTR Token

A comprehensive Solana SPL token implementation with advanced tokenomics, vesting schedules, staking mechanisms, and deflationary features built on the Anchor framework.

## Overview

VTR Token is a sophisticated token ecosystem designed for sustainable growth and community engagement. The project implements industry-standard tokenomics with programmatic vesting, yield-generating staking, and revenue-driven buyback and burn mechanisms.

## Key Features

### Token Economics
- **Total Supply**: 2,000,000,000 VTR tokens
- **Decimals**: 9
- **Standard**: SPL Token (Solana)
- **Deflationary Mechanism**: Revenue-based buyback and burn

### Allocation Structure
- **Token Sale**: 400M tokens (20%) - 10% TGE, 12-month linear vesting
- **Team & Advisors**: 300M tokens (15%) - 12-month cliff, 36-month linear vesting
- **Ecosystem Growth**: 500M tokens (25%) - 15% TGE, 48-month linear vesting
- **Liquidity**: 200M tokens (10%) - 50% TGE, 6-month linear vesting
- **Platform Reserve**: 300M tokens (15%) - 5% TGE, 6-month cliff, 60-month vesting
- **Buyback & Burn**: 200M tokens (10%) - 24-month distribution schedule
- **Marketing & Partnerships**: 100M tokens (5%) - 20% TGE, 18-month linear vesting

### Core Functionality
- **Vesting Schedules**: Automated token release based on predefined schedules
- **Staking System**: Configurable APY with lock periods (15% default APY)
- **Burn Mechanism**: Tiered revenue-based token burning (10-25% of revenue)
- **Governance Ready**: Token-weighted voting system
- **Security Features**: Multi-signature support, timelocks, whale protection

## Technical Specifications

### Smart Contract Architecture
- **Framework**: Anchor 0.31.1
- **Language**: Rust
- **Network**: Solana
- **Program ID**: `2jYy4kkMB6hTj9uZCDhCPqUyWaMBRRmZDTjW8rET9kD6`

### Dependencies
- Anchor Lang: 0.31.1
- Anchor SPL: 0.31.1
- SPL Token: 4.0
- SPL Associated Token Account: 2.3

### Account Structure
- **TokenData**: Central token state and supply tracking
- **TokenAllocation**: Individual vesting schedules and claims
- **StakingPool**: Staking parameters and total stake tracking
- **StakeAccount**: Individual stake positions and rewards

## Installation

### Prerequisites
- Node.js 16+ and npm/yarn
- Rust 1.70+
- Solana CLI 1.18+
- Anchor CLI 0.31.1

### Setup
```bash
# Clone the repository
git clone <repository-url>
cd vtr-token

# Install dependencies
npm install

# Install Rust dependencies
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install 0.31.1
avm use 0.31.1

# Verify installation
anchor --version
solana --version
```

### Local Development
```bash
# Start local validator
solana-test-validator --reset

# Set to localnet
solana config set --url localhost

# Build the program
anchor build

# Deploy to localnet
anchor deploy

# Run tests
anchor test --skip-local-validator
```

## Usage

### Initialization
```typescript
// Initialize the token with total supply
await program.methods
  .initializeToken(new anchor.BN("2000000000000000000"))
  .accounts({
    authority: authority.publicKey,
    mint: mint.publicKey,
  })
  .signers([mint])
  .rpc();
```

### Token Allocation
```typescript
// Mint tokens with specific allocation type
await program.methods
  .mintTokens(amount, { tokenSale: {} })
  .accounts({
    authority: authority.publicKey,
    mint: mint.publicKey,
    recipient: recipient.publicKey,
    recipientTokenAccount,
  })
  .rpc();
```

### Staking Operations
```typescript
// Initialize staking pool
await program.methods
  .initializeStaking(1500, new anchor.BN(30 * 24 * 3600)) // 15% APY, 30 days minimum
  .accounts({
    authority: authority.publicKey,
    mint: mint.publicKey,
  })
  .rpc();

// Stake tokens
await program.methods
  .stakeTokens(stakeAmount, stakeDuration)
  .accounts({
    user: user.publicKey,
    mint: mint.publicKey,
    userTokenAccount,
  })
  .signers([user])
  .rpc();
```

### Vesting Claims
```typescript
// Claim vested tokens
await program.methods
  .claimVestedTokens()
  .accounts({
    recipient: recipient.publicKey,
    mint: mint.publicKey,
    recipientTokenAccount,
  })
  .signers([recipient])
  .rpc();
```

## Testing

### Run Test Suite
```bash
# Full test suite
anchor test

# Skip validator startup (if already running)
anchor test --skip-local-validator

# Specific test file
npx ts-mocha -p ./tsconfig.json tests/vtr-token.ts
```

### Test Coverage
- Token initialization and mint authority
- Multi-allocation minting with TGE unlocks
- Vesting schedule calculations and claims
- Staking pool creation and token staking
- Unstaking with reward calculations
- Token burning mechanism
- Error handling and edge cases

## Deployment

### Mainnet Deployment
```bash
# Configure for mainnet
solana config set --url mainnet-beta

# Update Anchor.toml
[programs.mainnet]
vtr_token = "YOUR_MAINNET_PROGRAM_ID"

# Deploy
anchor deploy --provider.cluster mainnet
```

### Devnet Deployment
```bash
# Configure for devnet
solana config set --url devnet

# Deploy
anchor deploy --provider.cluster devnet
```

## API Reference

### Program Instructions

#### `initialize_token(total_supply: u64)`
Initializes the token mint and central data account.

#### `mint_tokens(amount: u64, allocation_type: AllocationType)`
Mints tokens to a recipient with specified allocation parameters.

#### `claim_vested_tokens()`
Claims available vested tokens based on time elapsed and vesting schedule.

#### `initialize_staking(apy_percentage: u16, min_stake_duration: i64)`
Sets up the staking pool with specified parameters.

#### `stake_tokens(amount: u64, duration: i64)`
Stakes tokens for a specified duration to earn rewards.

#### `unstake_tokens()`
Unstakes tokens and claims accumulated rewards.

#### `burn_tokens(amount: u64)`
Burns tokens from circulation and updates supply tracking.

### Allocation Types
- `TokenSale`: Public and private sale allocations
- `TeamAdvisors`: Team and advisor allocations with cliff
- `EcosystemGrowth`: Developer grants and ecosystem expansion
- `Liquidity`: DEX liquidity and market making
- `PlatformReserve`: Treasury and operational reserves
- `BuybackBurn`: Revenue-driven deflationary mechanism
- `Marketing`: Marketing and partnership allocations

## Buyback and Burn Mechanism

### Revenue Tiers
- **Tier 1** ($0 - $100k monthly): 10% buyback
- **Tier 2** ($100k - $500k monthly): 15% buyback
- **Tier 3** ($500k - $1M monthly): 20% buyback
- **Tier 4** ($1M+ monthly): 25% buyback

### Projected Burns
- **Year 1**: 5M tokens (0.25% of supply)
- **Year 2**: 15M tokens (0.75% of supply)
- **Year 3**: 30M tokens (1.5% of supply)
- **Year 5**: 50M tokens annually (2.5% of supply)

## Development Roadmap

### Phase 1: Core Infrastructure
- [x] Token deployment and basic functionality
- [x] Vesting mechanism implementation
- [x] Staking system development
- [x] Testing suite completion

### Phase 2: Advanced Features
- [ ] Governance module integration
- [ ] Cross-chain bridge compatibility
- [ ] Mobile wallet integration
- [ ] Advanced analytics dashboard

## Contributing

### Development Guidelines
1. Follow Rust and TypeScript best practices
2. Maintain comprehensive test coverage
3. Document all public interfaces
4. Submit pull requests for review

### Code Style
- Use `cargo fmt` for Rust formatting
- Use Prettier for TypeScript formatting
- Follow Anchor framework conventions
- Include inline documentation

### Documentation
- [Anchor Framework](https://www.anchor-lang.com/)
- [Solana Documentation](https://docs.solana.com/)
- [SPL Token Program](https://spl.solana.com/token)
---
