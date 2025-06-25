import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VtrToken } from "../target/types/vtr_token";
import { PublicKey, Keypair } from "@solana/web3.js";

async function deploy() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.VtrToken as Program<VtrToken>;
  const authority = provider.wallet as anchor.Wallet;

  console.log("Deploying VTR Token...");
  console.log("Program ID:", program.programId.toString());
  console.log("Authority:", authority.publicKey.toString());

  // Generate mint keypair
  const mint = Keypair.generate();
  console.log("Mint:", mint.publicKey.toString());

  // Find PDAs
  const [tokenData] = await PublicKey.findProgramAddress(
    [Buffer.from("token_data"), mint.publicKey.toBuffer()],
    program.programId
  );

  const [stakingPool] = await PublicKey.findProgramAddress(
    [Buffer.from("staking_pool"), authority.publicKey.toBuffer()],
    program.programId
  );

  console.log("Token Data PDA:", tokenData.toString());
  console.log("Staking Pool PDA:", stakingPool.toString());

  // Initialize token
  const TOTAL_SUPPLY = new anchor.BN(2_000_000_000 * 10**9); // 2B tokens
  
  try {
    const tx1 = await program.methods
      .initializeToken(TOTAL_SUPPLY)
      .accounts({
        authority: authority.publicKey,
        mint: mint.publicKey,
      })
      .signers([mint])
      .rpc();
    console.log("Token initialized:", tx1);

    // Initialize staking
    const tx2 = await program.methods
      .initializeStaking(1500, new anchor.BN(30 * 24 * 3600)) // 15% APY, 30 days min
      .accounts({
        authority: authority.publicKey,
        mint: mint.publicKey,
      })
      .rpc();
    console.log("Staking initialized:", tx2);

    console.log("\n=== Deployment Complete ===");
    console.log("Save these addresses:");
    console.log("Mint:", mint.publicKey.toString());
    console.log("Token Data:", tokenData.toString());
    console.log("Staking Pool:", stakingPool.toString());

  } catch (error) {
    console.error("Deployment failed:", error);
  }
}

deploy().then(() => process.exit(0)).catch(error => {
  console.error(error);
  process.exit(1);
});