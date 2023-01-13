import {Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction} from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import {
  createInitializeMint2Instruction,
  getMinimumBalanceForRentExemptMint,
  MINT_SIZE,
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";


export async function createNFTMint(connection: Connection, payer: Keypair, authority: PublicKey): Promise<Keypair> {

  const lamports = await getMinimumBalanceForRentExemptMint(connection);
  const mintKey = anchor.web3.Keypair.generate();

  const transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: mintKey.publicKey,
        space: MINT_SIZE,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(mintKey.publicKey, 0, authority, authority, TOKEN_PROGRAM_ID),
  );

  await sendAndConfirmTransaction(connection, transaction, [payer, mintKey]);
  return mintKey;
}
