import * as anchor from "@project-serum/anchor";
import {
  createTokenMint, findBazaarListingPda,
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda,
  findListingDomainPda, findSellerAccountPda, sleep
} from "./utils";
import {LAMPORTS_PER_SOL, SystemProgram, Transaction} from "@solana/web3.js";
import {Program} from "@project-serum/anchor";
const { assert } = require("chai");
const { PublicKey } = anchor.web3;
import { Keychain } from "../target/types/keychain";
import {expect} from "chai";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount, createAssociatedTokenAccountInstruction,
  createMintToCheckedInstruction,
  getAssociatedTokenAddressSync, NATIVE_MINT, TOKEN_PROGRAM_ID
} from "@solana/spl-token";

function randomName() {
  return Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
}

const keychainProgram = anchor.workspace.Keychain as Program<Keychain>;

const keychainDomain = randomName();
const listingDomainName = randomName();
let keychainDomainPda = null;
const keyCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

let txid: string;
let tx: Transaction;
let programDataAccount;

// we'll use this as the keychain domain + listing domain treasury
let treasury = anchor.web3.Keypair.generate();

const sellerName = randomName();
let sellerKeychainPda = null;
let sellerKeypair = anchor.web3.Keypair.generate();
let buyerKeypair = anchor.web3.Keypair.generate();
let sellerAccountPda = null;
let sellerAccountListingIndex = 0;


describe("bazaar", () => {

  // Configure the client to use the local cluster.
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Program client handle.
  const bazaarProg = anchor.workspace.Bazaar;
  const connection = provider.connection;

  const [listingDomainPda] = findListingDomainPda(listingDomainName, 0, bazaarProg.programId);

  it ('sets up the test', async () => {

    // airdrop 1 sol to the treasury
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(treasury.publicKey, anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
    );

    // airdrop some sol to the seller
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(sellerKeypair.publicKey, anchor.web3.LAMPORTS_PER_SOL * 10),
        "confirmed"
    );

    // airdrop some sol to the buyer
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(buyerKeypair.publicKey, anchor.web3.LAMPORTS_PER_SOL * 10),
        "confirmed"
    );

    // create the domain
    [keychainDomainPda] = findDomainPda(keychainDomain, keychainProgram.programId);
    const [domainStatePda, domainStatePdaBump] = findDomainStatePda(keychainDomain, keychainProgram.programId);
    txid = await keychainProgram.methods.createDomain(keychainDomain, keyCost).accounts({
      domain: keychainDomainPda,
      domainState: domainStatePda,
      authority: provider.wallet.publicKey,
      systemProgram: SystemProgram.programId,
      treasury: treasury.publicKey
    }).rpc();
    console.log(`created keychain domain tx: ${txid}`);

  });


  it("Creates a listing domain", async () => {

    // find the program's data account
    console.log('bazaar progid: ', bazaarProg.programId.toBase58());
    const bazaarProgramAccount = await connection.getParsedAccountInfo(bazaarProg.programId);
    // @ts-ignore
    programDataAccount = new PublicKey(bazaarProgramAccount.value.data.parsed.info.programData);
    console.log('program data account: ', programDataAccount);


    // now we can create the listing domain as an admin
    txid = await bazaarProg.methods.createListingDomain({name: listingDomainName, domainIndex: 0, treasury: treasury.publicKey})
        .accounts({
          upgradeAuthority: bazaarProg.provider.wallet.publicKey,
          program: bazaarProg.programId,
          programData: programDataAccount,
          systemProgram: anchor.web3.SystemProgram.programId,
          listingDomain: listingDomainPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

    console.log(`created listing domain: ${listingDomainName}, pda: ${listingDomainPda}, txid `, txid);

    let listingDomain = await bazaarProg.account.listingDomain.fetch(listingDomainPda);
    console.log('listing domain: ', listingDomain);

    const decodedName = new TextDecoder("utf-8").decode(new Uint8Array(listingDomain.name.filter(char => char !== 0)));
    console.log('decoded listing domain name: ', decodedName);
    assert.equal(decodedName, listingDomainName);

  });

  it("creates a seller account", async () => {

    // let's create the keychain + seller accounts in a single tx
    [sellerKeychainPda] = findKeychainPda(sellerName, keychainDomain, keychainProgram.programId);
    const [sellerKeychainStatePda] = findKeychainStatePda(sellerKeychainPda, keychainDomain, keychainProgram.programId);
    // the "pointer" keychain key account
    const [sellerKeychainKeyPda] = findKeychainKeyPda(sellerKeypair.publicKey, keychainDomain, keychainProgram.programId);

    console.log(`creating keychain domain: ${keychainDomain}...`);

    let ix = await keychainProgram.methods.createKeychain(sellerName).accounts({
      keychain: sellerKeychainPda,
      keychainState: sellerKeychainStatePda,
      keychainKey: sellerKeychainKeyPda,
      domain: keychainDomainPda,
      authority: sellerKeypair.publicKey,
      wallet: sellerKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    }).instruction();

    let tx = new Transaction().add(ix);

    [sellerAccountPda] = findSellerAccountPda(sellerKeychainPda, bazaarProg.programId);

    ix = await bazaarProg.methods.createSeller().accounts({
      keychain: sellerKeychainPda,
      sellerAccount: sellerAccountPda,
      seller: sellerKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    }).instruction();

    tx.add(ix);

    txid = await provider.sendAndConfirm(tx, [sellerKeypair]);
    console.log(`created keychain and seller account for: ${sellerName}, txid `, txid);

    let sellerAccount = await bazaarProg.account.sellerAccount.fetch(sellerAccountPda);

    console.log("seller account: ", sellerAccount);
    expect(sellerAccount.accountVersion).to.equal(0);
    expect(sellerAccount.bump).to.be.greaterThan(0);
    expect(sellerAccount.listingIndex).to.equal(0);
    expect(sellerAccount.keychain.toBase58()).to.equal(sellerKeychainPda.toBase58());
    sellerAccountListingIndex = 0;
  });

  it("creates a single item listing", async () => {
    let itemMint = await createTokenMint(connection, sellerKeypair, sellerKeypair.publicKey);

    // currency atas for the buyer / seller
    let buyerItemTokenAccount = await createAssociatedTokenAccount(connection, buyerKeypair, itemMint.publicKey, buyerKeypair.publicKey);
    let sellerItemTokenAccount = await createAssociatedTokenAccount(connection, sellerKeypair, itemMint.publicKey, sellerKeypair.publicKey);

    // now mint 10k tokens to buyer's currency ata and create the seller's currency ata
    const numTokens = 10000;
    tx = new Transaction().add(
        createMintToCheckedInstruction(
            itemMint.publicKey,
            sellerItemTokenAccount,
            sellerKeypair.publicKey,
            numTokens * 1e9,
            9
        ),
    );
    let txid = await provider.sendAndConfirm(tx, [sellerKeypair]);
    console.log(`minted ${numTokens} tokens to seller's ata: ${sellerItemTokenAccount.toBase58()} \n`);

    let currencyMint = NATIVE_MINT;
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 5);
    let quantity = new anchor.BN(9 * 100); // 9 tokens

    let [listingPda] = findBazaarListingPda(sellerAccountPda, ++sellerAccountListingIndex, bazaarProg.programId);
    let listingItemToken = getAssociatedTokenAddressSync(itemMint.publicKey, listingPda, true);

    await sleep(1000);

    let accounts = {
      listingDomain: listingDomainPda,
      seller: sellerKeypair.publicKey,
      sellerAccount: sellerAccountPda,
      keychain: sellerKeychainPda,
      listing: listingPda,
      currency: currencyMint,
      // proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: itemMint.publicKey,
      item0SellerToken: sellerItemTokenAccount,
      item0ListingToken: listingItemToken,
      item1: null,
      item1SellerToken: null,
      item1ListingToken: null,
      item2: null,
      item2SellerToken: null,
      item2ListingToken: null,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };
    console.log('accounts: ', accounts);

    // create the listing
    tx = await bazaarProg.methods.createListing({price, listingType: {bag: {}}, itemQuantities: [quantity]}).accounts({
      listingDomain: listingDomainPda,
      seller: sellerKeypair.publicKey,
      sellerAccount: sellerAccountPda,
      keychain: sellerKeychainPda,
      listing: listingPda,
      currency: currencyMint,
      proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: itemMint.publicKey,
      item0SellerToken: sellerItemTokenAccount,
      item0ListingToken: listingItemToken,
      item1: null,
      item1SellerToken: null,
      item1ListingToken: null,
      item2: null,
      item2SellerToken: null,
      item2ListingToken: null,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    }).transaction();

    console.log("create listing tx: ", tx);
    console.log("keys: ", tx.instructions[0].keys);

    txid = await provider.sendAndConfirm(tx, [sellerKeypair], {skipPreflight: true});

    console.log(`created listing: ${listingPda.toBase58()}, txid `, txid);
    const listing = await bazaarProg.account.listing.fetch(listingPda);
    console.log("listing account: ", listing);

    expect(listing.accountVersion).to.equal(0);
    expect(listing.bump).is.greaterThan(0);
    expect(listing.currency.toBase58()).to.equal(currencyMint.toBase58());
    assert.isTrue('bag' in listing.listingType);
    assert(listing.items.length == 1);
    expect(listing.items[0].quantity.toNumber()).to.equal(quantity.toNumber());
    expect(listing.items[0].itemMint.toBase58()).to.equal(itemMint.publicKey.toBase58());
    expect(listing.items[0].itemToken.toBase58()).to.equal(listingItemToken.toBase58());
    expect(listing.treasury.toBase58()).to.equal(treasury.publicKey.toBase58());


  });


});
