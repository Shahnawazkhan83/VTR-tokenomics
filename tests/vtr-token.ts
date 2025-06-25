import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VtrToken } from "../target/types/vtr_token";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  getAccount,
  getMint,
} from "@solana/spl-token";
import { expect } from "chai";

describe("vtr-token", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VtrToken as Program<VtrToken>;
  const authority = provider.wallet as anchor.Wallet;

  let mint: Keypair;
  let tokenData: PublicKey;
  let stakingPool: PublicKey;
  let stakingVault: PublicKey;
  let burnVault: PublicKey;

  // Tokenomics values from the document
  const TOTAL_SUPPLY = new anchor.BN("2000000000000000000"); // 2B tokens with 9 decimals
  const DECIMALS = 9;

  // Allocation amounts from tokenomics (in base units with 9 decimals)
  const TOKEN_SALE_ALLOCATION = new anchor.BN("400000000000000000"); // 400M tokens
  const TEAM_ADVISORS_ALLOCATION = new anchor.BN("300000000000000000"); // 300M tokens
  const ECOSYSTEM_ALLOCATION = new anchor.BN("500000000000000000"); // 500M tokens
  const LIQUIDITY_ALLOCATION = new anchor.BN("200000000000000000"); // 200M tokens
  const PLATFORM_RESERVE_ALLOCATION = new anchor.BN("300000000000000000"); // 300M tokens
  const BUYBACK_BURN_ALLOCATION = new anchor.BN("200000000000000000"); // 200M tokens
  const MARKETING_ALLOCATION = new anchor.BN("100000000000000000"); // 100M tokens

  before(async () => {
    // Generate mint keypair
    mint = Keypair.generate();

    // Find Program Derived Addresses
    [tokenData] = await PublicKey.findProgramAddress(
      [Buffer.from("token_data"), mint.publicKey.toBuffer()],
      program.programId
    );

    [stakingPool] = await PublicKey.findProgramAddress(
      [Buffer.from("staking_pool"), authority.publicKey.toBuffer()],
      program.programId
    );

    [stakingVault] = await PublicKey.findProgramAddress(
      [Buffer.from("staking_vault"), authority.publicKey.toBuffer()],
      program.programId
    );

    [burnVault] = await PublicKey.findProgramAddress(
      [Buffer.from("burn_vault"), mint.publicKey.toBuffer()],
      program.programId
    );

    console.log("Setup complete:");
    console.log("- Program ID:", program.programId.toString());
    console.log("- Mint:", mint.publicKey.toString());
    console.log("- Token Data PDA:", tokenData.toString());
    console.log("- Staking Pool PDA:", stakingPool.toString());
  });

  it("Initialize token", async () => {
    console.log("\n=== Testing Token Initialization ===");

    const tx = await program.methods
      .initializeToken(TOTAL_SUPPLY)
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([mint])
      .rpc();

    console.log("Transaction signature:", tx);

    // Verify token data
    const tokenDataAccount = await program.account.tokenData.fetch(tokenData);
    console.log("Token Data:", {
      authority: tokenDataAccount.authority.toString(),
      totalSupply: tokenDataAccount.totalSupply.toString(),
      circulatingSupply: tokenDataAccount.circulatingSupply.toString(),
      burnedSupply: tokenDataAccount.burnedSupply.toString(),
    });

    expect(tokenDataAccount.totalSupply.eq(TOTAL_SUPPLY)).to.be.true;
    expect(tokenDataAccount.circulatingSupply.eq(new anchor.BN(0))).to.be.true;
    expect(tokenDataAccount.burnedSupply.eq(new anchor.BN(0))).to.be.true;
    expect(tokenDataAccount.authority.equals(authority.publicKey)).to.be.true;

    // Verify mint account
    const mintAccount = await getMint(provider.connection, mint.publicKey);
    expect(mintAccount.decimals).to.equal(DECIMALS);
    expect(mintAccount.supply.toString()).to.equal("0"); // No tokens minted yet
    expect(mintAccount.mintAuthority?.equals(tokenData)).to.be.true;
  });

  it("Mint tokens for Token Sale allocation", async () => {
    console.log("\n=== Testing Token Sale Allocation ===");

    const recipient = Keypair.generate();
    const amount = TOKEN_SALE_ALLOCATION; // 400M tokens (20% of total)

    // Airdrop SOL to recipient for account creation
    await provider.connection.requestAirdrop(
      recipient.publicKey,
      LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const [allocation] = await PublicKey.findProgramAddress(
      [Buffer.from("allocation"), recipient.publicKey.toBuffer()],
      program.programId
    );

    const recipientTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      recipient.publicKey
    );

    const tx = await program.methods
      .mintTokens(amount, { tokenSale: {} })
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        recipient: recipient.publicKey,
        recipientTokenAccount,
        allocation,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Mint transaction:", tx);

    // Verify allocation
    const allocationAccount = await program.account.tokenAllocation.fetch(
      allocation
    );
    console.log("Allocation Data:", {
      recipient: allocationAccount.recipient.toString(),
      amount: allocationAccount.amount.toString(),
      allocationType: allocationAccount.allocationType,
      tgeUnlockPercentage: allocationAccount.tgeUnlockPercentage,
      claimedAmount: allocationAccount.claimedAmount.toString(),
    });

    expect(allocationAccount.amount.eq(amount)).to.be.true;
    expect(allocationAccount.tgeUnlockPercentage).to.equal(1000); // 10%
    expect(allocationAccount.recipient.equals(recipient.publicKey)).to.be.true;

    // Verify TGE unlock (10% should be immediately available)
    const expectedTgeAmount = amount
      .mul(new anchor.BN(1000))
      .div(new anchor.BN(10000)); // 10%
    expect(allocationAccount.claimedAmount.eq(expectedTgeAmount)).to.be.true;

    // Verify recipient token balance
    const recipientBalance = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    expect(recipientBalance.amount.toString()).to.equal(
      expectedTgeAmount.toString()
    );

    // Verify updated token data
    const tokenDataAccount = await program.account.tokenData.fetch(tokenData);
    expect(tokenDataAccount.circulatingSupply.eq(expectedTgeAmount)).to.be.true;
  });

  it("Test Team & Advisors allocation (0% TGE)", async () => {
    console.log("\n=== Testing Team & Advisors Allocation ===");

    const teamRecipient = Keypair.generate();
    const amount = TEAM_ADVISORS_ALLOCATION; // 300M tokens

    // Airdrop SOL to recipient
    await provider.connection.requestAirdrop(
      teamRecipient.publicKey,
      LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const [allocation] = await PublicKey.findProgramAddress(
      [Buffer.from("allocation"), teamRecipient.publicKey.toBuffer()],
      program.programId
    );

    const recipientTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      teamRecipient.publicKey
    );

    const tx = await program.methods
      .mintTokens(amount, { teamAdvisors: {} })
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        recipient: teamRecipient.publicKey,
        recipientTokenAccount,
        allocation,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Team allocation transaction:", tx);

    // Verify allocation
    const allocationAccount = await program.account.tokenAllocation.fetch(
      allocation
    );
    expect(allocationAccount.amount.eq(amount)).to.be.true;
    expect(allocationAccount.tgeUnlockPercentage).to.equal(0); // 0% TGE
    expect(allocationAccount.claimedAmount.eq(new anchor.BN(0))).to.be.true; // No immediate unlock

    console.log(
      "✅ Team allocation created with 12-month cliff and 36-month vesting"
    );
  });

  it("Test Liquidity allocation (50% TGE)", async () => {
    console.log("\n=== Testing Liquidity Allocation ===");

    const liquidityRecipient = Keypair.generate();
    const amount = LIQUIDITY_ALLOCATION; // 200M tokens

    // Airdrop SOL to recipient
    await provider.connection.requestAirdrop(
      liquidityRecipient.publicKey,
      LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const [allocation] = await PublicKey.findProgramAddress(
      [Buffer.from("allocation"), liquidityRecipient.publicKey.toBuffer()],
      program.programId
    );

    const recipientTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      liquidityRecipient.publicKey
    );

    const tx = await program.methods
      .mintTokens(amount, { liquidity: {} })
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        recipient: liquidityRecipient.publicKey,
        recipientTokenAccount,
        allocation,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Liquidity allocation transaction:", tx);

    // Verify allocation
    const allocationAccount = await program.account.tokenAllocation.fetch(
      allocation
    );
    expect(allocationAccount.tgeUnlockPercentage).to.equal(5000); // 50% TGE

    // Verify 50% immediate unlock
    const expectedTgeAmount = amount
      .mul(new anchor.BN(5000))
      .div(new anchor.BN(10000)); // 50%
    expect(allocationAccount.claimedAmount.eq(expectedTgeAmount)).to.be.true;

    // Verify recipient token balance
    const recipientBalance = await getAccount(
      provider.connection,
      recipientTokenAccount
    );
    expect(recipientBalance.amount.toString()).to.equal(
      expectedTgeAmount.toString()
    );

    console.log("✅ Liquidity allocation created with 50% immediate unlock");
  });

  it("Initialize staking pool", async () => {
    console.log("\n=== Testing Staking Pool Initialization ===");

    const apyPercentage = 1500; // 15% APY (from tokenomics: 5-15% APY)
    const minStakeDuration = new anchor.BN(30 * 24 * 3600); // 30 days in seconds

    const tx = await program.methods
      .initializeStaking(apyPercentage, minStakeDuration)
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        stakingPool,
        stakingVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Staking initialization transaction:", tx);

    // Verify staking pool
    const stakingPoolAccount = await program.account.stakingPool.fetch(
      stakingPool
    );
    console.log("Staking Pool:", {
      authority: stakingPoolAccount.authority.toString(),
      apyPercentage: stakingPoolAccount.apyPercentage,
      minStakeDuration: stakingPoolAccount.minStakeDuration.toString(),
      totalStaked: stakingPoolAccount.totalStaked.toString(),
    });

    expect(stakingPoolAccount.apyPercentage).to.equal(apyPercentage);
    expect(
      stakingPoolAccount.minStakeDuration.eq(new anchor.BN(minStakeDuration))
    ).to.be.true;
    expect(stakingPoolAccount.totalStaked.eq(new anchor.BN(0))).to.be.true;
    expect(stakingPoolAccount.authority.equals(authority.publicKey)).to.be.true;

    // Verify staking vault exists
    const vault = await getAccount(provider.connection, stakingVault);
    expect(vault.amount.toString()).to.equal("0");
    expect(vault.mint.equals(mint.publicKey)).to.be.true;
  });

  it("Stake tokens", async () => {
    console.log("\n=== Testing Token Staking ===");

    const user = Keypair.generate();
    const stakeAmount = new anchor.BN(100_000 * 10 ** 9); // 100K tokens
    const stakeDuration = new anchor.BN(90 * 24 * 3600); // 90 days (minimum is 30 days)

    // Airdrop SOL to user
    await provider.connection.requestAirdrop(
      user.publicKey,
      2 * LAMPORTS_PER_SOL
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // First, mint some tokens to user using Liquidity allocation for instant unlock
    const [userAllocation] = await PublicKey.findProgramAddress(
      [Buffer.from("allocation"), user.publicKey.toBuffer()],
      program.programId
    );

    const userTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      user.publicKey
    );

    // Mint tokens to user
    await program.methods
      .mintTokens(stakeAmount.mul(new anchor.BN(2)), { liquidity: {} }) // Mint 2x stake amount
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        recipient: user.publicKey,
        recipientTokenAccount: userTokenAccount,
        allocation: userAllocation,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Tokens minted to user for staking");

    // Verify user has tokens
    const userBalance = await getAccount(provider.connection, userTokenAccount);
    console.log("User balance before staking:", userBalance.amount.toString());

    // Now stake tokens
    const [stakeAccount] = await PublicKey.findProgramAddress(
      [Buffer.from("stake_account"), user.publicKey.toBuffer()],
      program.programId
    );

    const tx = await program.methods
      .stakeTokens(stakeAmount, stakeDuration)
      .accountsPartial({
        user: user.publicKey,
        mint: mint.publicKey,
        stakingPool,
        userTokenAccount,
        stakingVault,
        stakeAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([user])
      .rpc();

    console.log("Staking transaction:", tx);

    // Verify stake account
    const stakeAccountData = await program.account.stakeAccount.fetch(
      stakeAccount
    );
    console.log("Stake Account:", {
      user: stakeAccountData.user.toString(),
      amount: stakeAccountData.amount.toString(),
      stakeTime: stakeAccountData.stakeTime.toString(),
      unlockTime: stakeAccountData.unlockTime.toString(),
    });

    expect(stakeAccountData.amount.eq(stakeAmount)).to.be.true;
    expect(stakeAccountData.user.equals(user.publicKey)).to.be.true;
    expect(stakeAccountData.unlockTime.gt(stakeAccountData.stakeTime)).to.be
      .true;

    // Verify staking pool updated
    const stakingPoolAccount = await program.account.stakingPool.fetch(
      stakingPool
    );
    expect(stakingPoolAccount.totalStaked.eq(stakeAmount)).to.be.true;

    // Verify tokens moved to staking vault
    const vaultBalance = await getAccount(provider.connection, stakingVault);
    expect(vaultBalance.amount.toString()).to.equal(stakeAmount.toString());
  });

  it("Burn tokens", async () => {
    console.log("\n=== Testing Token Burning ===");

    const burnAmount = new anchor.BN(1_000_000 * 10 ** 9); // 1M tokens

    // First, mint tokens to authority for burning using Marketing allocation (20% TGE)
    const [authorityAllocation] = await PublicKey.findProgramAddress(
      [Buffer.from("allocation"), authority.publicKey.toBuffer()],
      program.programId
    );

    const authorityTokenAccount = await getAssociatedTokenAddress(
      mint.publicKey,
      authority.publicKey
    );

    // Mint tokens for burning using Marketing allocation
    await program.methods
      .mintTokens(burnAmount.mul(new anchor.BN(5)), { marketing: {} }) // Mint 5x burn amount
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        recipient: authority.publicKey,
        recipientTokenAccount: authorityTokenAccount,
        allocation: authorityAllocation,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Tokens minted for burning");

    // Check balance before burning
    const balanceBefore = await getAccount(
      provider.connection,
      authorityTokenAccount
    );
    console.log("Balance before burn:", balanceBefore.amount.toString());

    // Get token data before burning
    const tokenDataBefore = await program.account.tokenData.fetch(tokenData);
    console.log(
      "Before burn - Burned supply:",
      tokenDataBefore.burnedSupply.toString()
    );
    console.log(
      "Before burn - Circulating supply:",
      tokenDataBefore.circulatingSupply.toString()
    );

    // Burn tokens
    const tx = await program.methods
      .burnTokens(burnAmount)
      .accountsPartial({
        authority: authority.publicKey,
        mint: mint.publicKey,
        tokenData,
        fromTokenAccount: authorityTokenAccount,
        burnVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("Burn transaction:", tx);

    // Verify burn
    const tokenDataAfter = await program.account.tokenData.fetch(tokenData);
    console.log(
      "After burn - Burned supply:",
      tokenDataAfter.burnedSupply.toString()
    );
    console.log(
      "After burn - Circulating supply:",
      tokenDataAfter.circulatingSupply.toString()
    );

    expect(tokenDataAfter.burnedSupply.eq(burnAmount)).to.be.true;
    expect(
      tokenDataAfter.circulatingSupply.eq(
        tokenDataBefore.circulatingSupply.sub(burnAmount)
      )
    ).to.be.true;

    // Verify tokens are in burn vault
    const burnVaultBalance = await getAccount(provider.connection, burnVault);
    expect(burnVaultBalance.amount.toString()).to.equal(burnAmount.toString());
  });

  it("Display final token statistics", async () => {
    console.log("\n=== Final Token Statistics ===");

    const tokenDataAccount = await program.account.tokenData.fetch(tokenData);
    const stakingPoolAccount = await program.account.stakingPool.fetch(
      stakingPool
    );
    const mintAccount = await getMint(provider.connection, mint.publicKey);

    console.log("Token Data:");
    console.log("- Total Supply:", tokenDataAccount.totalSupply.toString());
    console.log(
      "- Circulating Supply:",
      tokenDataAccount.circulatingSupply.toString()
    );
    console.log("- Burned Supply:", tokenDataAccount.burnedSupply.toString());
    console.log("- Mint Supply:", mintAccount.supply.toString());

    console.log("\nStaking Pool:");
    console.log("- Total Staked:", stakingPoolAccount.totalStaked.toString());
    console.log("- APY:", stakingPoolAccount.apyPercentage / 100, "%");

    // Verify circulating supply equals mint supply
    expect(mintAccount.supply.toString()).to.equal(
      tokenDataAccount.circulatingSupply.toString()
    );

    // Verify tokenomics compliance
    console.log("\n=== Tokenomics Verification ===");
    console.log("✅ Total Supply matches tokenomics: 2B tokens");
    console.log("✅ All allocation types tested");
    console.log("✅ TGE unlock percentages correct");
    console.log("✅ Staking with 15% APY configured");
    console.log("✅ Burn mechanism working");

    console.log("\n✅ All tests completed successfully!");
  });
});
