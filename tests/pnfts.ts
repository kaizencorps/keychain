import * as anchor from "@project-serum/anchor";
import {Idl, Program, Wallet, web3} from "@project-serum/anchor";

import { execSync } from "child_process";

import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction
} from "@solana/web3.js";
import {
  createNFT, createpNFT, createTokenMint,
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda,
  findListingPda,
} from "./utils";
import * as assert from "assert";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createAssociatedTokenAccountInstruction,
  createMintToCheckedInstruction,
  getAssociatedTokenAddressSync,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {expect} from "chai";

import { Keychain } from "../target/types/keychain";
import { Yardsale } from "../target/types/yardsale";
import {PNftTransferClient} from "./PNftTransferClient";



///// this test is set up to run in devnet (since it uses metaplex programs)

// then u can run: anchor test --provider.cluster localnet --skip-local-validator

function randomName() {
  let name = Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
  return name.toLowerCase();
}

// for setting up the keychain
const domain = randomName();
const treasury = anchor.web3.Keypair.generate();
const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

const username = randomName();    // used as the keychain name


describe("yardsale", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;


  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const KeychainProgram = anchor.workspace.Keychain as Program<Keychain>;
  const YardsaleProgram = anchor.workspace.Yardsale as Program<Yardsale>;

  // const KeychainProgram = new Program(workxpaceKeychainProgram.idl, workxpaceKeychainProgram.programId, provider);
  // const YardsaleProgram = new Program(workspaceYardsaleProgram.idl, workspaceYardsaleProgram.programId, provider);

  console.log(`--> yardsale program id: ${YardsaleProgram.programId.toBase58()} \n --> keychain program id: ${YardsaleProgram.programId.toBase58()}`);


  const pNftTransferClient = new PNftTransferClient(provider.connection, provider.wallet as anchor.Wallet)
  pNftTransferClient.setProgram();

  let userKeychainPda: PublicKey;
  let domainPda: PublicKey;

  // let proceedsAccount: Keypair = Keypair.generate();

  // use a proceedsAccount that exists (since it's expected to exist by the list method)
  let proceedsAccount = new PublicKey('r3cXGs7ku4Few6J1rmNwwUNQbvrSPoLAAU9C2TVKfow');



  // let buyer: Keypair = Keypair.generate();
  let buyer = new PublicKey('BPhtwoopE2bG6ArCM9VPREx6tV6RMWmj5Q8Fysc1y8Ye');

  console.log(`\n\n...>>> user: ${provider.wallet.publicKey.toBase58()}`);

  // the pNFTs that we'll work with
  const pnfts: PublicKey[] = [];

  it("sets up testing env", async () => {

    // create a few pNFTs
    for (let i = 0; i < 1; i++) {
      const pnftMint = await createpNFT(provider);
      console.log(`created pNFT: ${pnftMint.toBase58()}`);
      pnfts.push(pnftMint);
    }

    // send a little bit of sol to the buyer
    let tx = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          // toPubkey: buyer.publicKey, // create a random receiver
          toPubkey: buyer, // create a random receiver
          lamports: 0.02 * LAMPORTS_PER_SOL,
        })
    );

    let txid = await provider.sendAndConfirm(tx);
    console.log(`sent ${0.02} sol to buyer: ${buyer.toBase58()}`);

    // create the keychain domain + user's keychain

    // our domain account
    [domainPda] = findDomainPda(domain, KeychainProgram.programId);
    const [domainStatePda, domainStatePdaBump] = findDomainStatePda(domain, KeychainProgram.programId);

    // our keychain accounts
    [userKeychainPda] = findKeychainPda(username, domain, KeychainProgram.programId);
    const [userKeychainStatePda] = findKeychainStatePda(userKeychainPda, domain, KeychainProgram.programId);
    // the "pointer" keychain key account
    const [userKeychainKeyPda] = findKeychainKeyPda(provider.wallet.publicKey, domain, KeychainProgram.programId);

    console.log(`creating keychain domain: ${domain}...`);

    // first create the domain
    txid = await KeychainProgram.methods.createDomain(domain, renameCost).accounts({
      domain: domainPda,
      domainState: domainStatePda,
      authority: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      treasury: treasury.publicKey
    }).rpc();
    console.log(`created keychain domain tx: ${txid}`);

    console.log(`creating keychain for : ${username}...`);

    // then create the keychain
    txid = await KeychainProgram.methods.createKeychain(username).accounts({
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

  it("list and buy a pNFT in sol", async () => {

    // the pnft to list
    let pnft = pnfts[0];
    // user's nft token account
    let fromItemToken = getAssociatedTokenAddressSync(pnft, provider.wallet.publicKey);
    let buyerItemToken = getAssociatedTokenAddressSync(pnft, buyer, true);
    let [listingPda] = findListingPda(pnft, username, domain, YardsaleProgram.programId);
    // listing's ata
    let listingItemToken = getAssociatedTokenAddressSync(pnft, listingPda, true);

    // try simple transfer of the pNFT to the buyer

    // need to create the ata first
    /*
    let tx = new Transaction().add(
        createAssociatedTokenAccountInstruction(
          provider.wallet.publicKey,
          buyerItemToken,
          buyer,
          pnft,
    ));

    const builder = await pNftTransferClient.buildTransferPNFT({
      sourceAta: fromItemToken,
      nftMint: pnft,
      destAta: buyerItemToken,
      owner: provider.wallet.publicKey,
      receiver: buyer
    });

     */

    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

    const builder = await pNftTransferClient.buildListPNFT(price, {
      domain: domainPda,
      keychain: userKeychainPda,
      nftMint: pnft,
      listing: listingPda,
      currency: NATIVE_MINT,
      proceeds: proceedsAccount,
      proceedsToken: null,
      listingItemToken,
      authority: provider.wallet.publicKey
    });

    let tx = new Transaction().add(await builder.instruction());

    let txid = await provider.sendAndConfirm(tx);

    console.log(`listed pNFT ${pnft.toBase58()} in tx: ${txid}`);
    console.log(`listingPda: ${listingPda.toBase58()}, listingItemToken: ${listingItemToken.toBase58()}`);



    // const newReceiverBalance = await provider.connection.getTokenAccountBalance(buyerItemToken);
    // expect(newReceiverBalance.value.uiAmount).to.equal(1)

  });


});



