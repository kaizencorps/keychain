import * as anchor from "@project-serum/anchor";
import {Idl, Program, Wallet, web3} from "@project-serum/anchor";

import { execSync } from "child_process";

import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction
} from "@solana/web3.js";
import {
  createNFT,
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda,
  findListingPda,
} from "./utils";
import * as assert from "assert";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  NATIVE_MINT, TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {expect} from "chai";

import { Keychain } from "../target/types/keychain";
import { Yardsale } from "../target/types/yardsale";

const keychainProgram = anchor.workspace.Keychain as Program<Keychain>;
const yardsaleProgram = anchor.workspace.Yardsale as Program<Yardsale>;


///// this test is set up to run against a local validator with the assumptions:
///// 1. the keychain program is deployed to the local validator at the address in the keychain idl
///// 2. the key set up in anchor.toml is funded with SOL (to deploy stache)

// then u can run: anchor test --provider.cluster localnet --skip-local-validator

const deployKeychain = () => {
  const deployCmd = `solana program deploy --url localhost -v --program-id $(pwd)/../keychain/target/deploy/keychain-keypair.json $(pwd)/../keychain/target/deploy/keychain.so`;
  execSync(deployCmd);
};

function randomName() {
  let name = Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
  return name.toLowerCase();
}

// for setting up the keychain
const domain = randomName();
const treasury = anchor.web3.Keypair.generate();
const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

const username = randomName();    // used as the keychain name

console.log(`--> yardsale program id: ${yardsaleProgram.programId.toBase58()} \n --> keychain program id: ${keychainProgram.programId.toBase58()}`);

describe("yardsale", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  let userKeychainPda: PublicKey;
  let domainPda: PublicKey;
  let saleAccount: Keypair = Keypair.generate();
  let buyer: Keypair = Keypair.generate();

  console.log(`\n\n...>>> user: ${provider.wallet.publicKey.toBase58()}`);

  const nfts: PublicKey[] = [];

  it("sets up testing env", async () => {
    // Add your test here.
    // const tx = await program.methods.initialize().rpc();

    // just deploy by yourself
    // console.log(`deploying Keychain...`);
    // deployKeychain();
    // console.log("✔ Keychain Program deployed.");

    await connection.confirmTransaction(
        await connection.requestAirdrop(provider.wallet.publicKey, anchor.web3.LAMPORTS_PER_SOL * 50),
        "confirmed"
    );

    await connection.confirmTransaction(
        await connection.requestAirdrop(buyer.publicKey, anchor.web3.LAMPORTS_PER_SOL * 50),
        "confirmed"
    );

    // create a few nfts to sell
    for (let i = 0; i < 1; i++) {
      const nft = await createNFT(provider);
      console.log(`created nft: ${nft.publicKey.toBase58()}`);
      nfts.push(nft.publicKey);
    }

    // create the keychain domain + user's keychain

    // our domain account
    [domainPda] = findDomainPda(domain, keychainProgram.programId);
    const [domainStatePda, domainStatePdaBump] = findDomainStatePda(domain, keychainProgram.programId);

    // our keychain accounts
    [userKeychainPda] = findKeychainPda(username, domain, keychainProgram.programId);
    const [userKeychainStatePda] = findKeychainStatePda(userKeychainPda, domain, keychainProgram.programId);
    // the "pointer" keychain key account
    const [userKeychainKeyPda] = findKeychainKeyPda(provider.wallet.publicKey, domain, keychainProgram.programId);

    console.log(`creating keychain domain: ${domain}...`);

    // first create the domain
    let txid = await keychainProgram.methods.createDomain(domain, renameCost).accounts({
      domain: domainPda,
      domainState: domainStatePda,
      authority: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      treasury: treasury.publicKey
    }).rpc();
    console.log(`created keychain domain tx: ${txid}`);

    console.log(`creating keychain for : ${username}...`);

    // then create the keychain
    txid = await keychainProgram.methods.createKeychain(username).accounts({
      keychain: userKeychainPda,
      keychainState: userKeychainStatePda,
      keychainKey: userKeychainKeyPda,
      domain: domainPda,
      authority: provider.wallet.publicKey,
      wallet: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
    }).rpc();

    console.log(`created keychain for ${username}. tx: ${txid}`);
  });

  it("list and buy an nft in sol", async () => {

    // the nft to list
    let nft = nfts[0];
    // user's nft token account
    let fromItemToken = getAssociatedTokenAddressSync(nft, provider.wallet.publicKey);
    let [listingPda] = findListingPda(nft, username, domain, yardsaleProgram.programId);
    // listing's ata
    let itemToken = getAssociatedTokenAddressSync(nft, listingPda, true);

    // list for .5 sol
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.5);
    let txid = await yardsaleProgram.methods.listItem(price).accounts({
      domain: domainPda,
      keychain: userKeychainPda,
      authority: provider.wallet.publicKey,
      item: nft,
      fromItemToken,
      listing: listingPda,
      itemToken,
      currency: NATIVE_MINT,
      saleAccount: saleAccount.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      saleToken: null       // optional accounts set to null
    }).rpc();

    console.log(`listed nft: ${nft.toBase58()} for ${price} sol: ${txid}`);

    let buyerItemToken = getAssociatedTokenAddressSync(nft, buyer.publicKey, false);

    let tx = new Transaction().add(
        createAssociatedTokenAccountInstruction(buyer.publicKey, buyerItemToken, buyer.publicKey, nft)
    );

    // now we buy it
    let ix = await yardsaleProgram.methods.purchaseItem().accounts({
      listing: listingPda,
      item: nft,
      itemToken,
      toItemToken: buyerItemToken,
      currency: NATIVE_MINT,
      saleAccount: saleAccount.publicKey,
      authority: buyer.publicKey,
      treasury: treasury.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      // optional accounts set to null
      saleToken: null,
      buyerToken: null
    }).instruction();

    tx.add(ix);
    txid = await provider.sendAndConfirm(tx, [buyer]);

    console.log(`bought nft: ${nft.toBase58()} for ${price} sol: ${txid}`);

  });

});



