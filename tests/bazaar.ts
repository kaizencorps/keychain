import * as anchor from "@project-serum/anchor";
import {
  createTokenMint, findBazaarListingPda,
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda,
  findListingDomainPda, findSellerAccountPda, getSolBalance, isWithinPercentageThreshold
} from "./utils";
import {Keypair, LAMPORTS_PER_SOL, SystemProgram, Transaction} from "@solana/web3.js";
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

  let item0Mint: Keypair;
  let item1Mint: Keypair;
  let item2Mint: Keypair;

  // @ts-ignore
  let sellerItem0TokenAccount: PublicKey;
  // @ts-ignore
  let sellerItem1TokenAccount: PublicKey;
  // @ts-ignore
  let sellerItem2TokenAccount: PublicKey;
  // @ts-ignore
  let buyerItem0TokenAccount: PublicKey;
  // @ts-ignore
  let buyerItem1TokenAccount: PublicKey;
  // @ts-ignore
  let buyerItem2TokenAccount: PublicKey;

  it ('sets up the test', async () => {

    // airdrop 1 sol to the treasury
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(treasury.publicKey, anchor.web3.LAMPORTS_PER_SOL),
        "confirmed"
    );

    // airdrop some sol to the seller
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(sellerKeypair.publicKey, anchor.web3.LAMPORTS_PER_SOL * 100),
        "confirmed"
    );

    // airdrop some sol to the buyer
    await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(buyerKeypair.publicKey, anchor.web3.LAMPORTS_PER_SOL * 100),
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

    // mint some item tokens to seller's account
    item0Mint = await createTokenMint(connection, sellerKeypair, sellerKeypair.publicKey);
    item1Mint = await createTokenMint(connection, sellerKeypair, sellerKeypair.publicKey, 0);
    item2Mint = await createTokenMint(connection, sellerKeypair, sellerKeypair.publicKey, 0);

    sellerItem0TokenAccount = await createAssociatedTokenAccount(connection, sellerKeypair, item0Mint.publicKey, sellerKeypair.publicKey);
    sellerItem1TokenAccount = await createAssociatedTokenAccount(connection, sellerKeypair, item1Mint.publicKey, sellerKeypair.publicKey);
    sellerItem2TokenAccount = await createAssociatedTokenAccount(connection, sellerKeypair, item2Mint.publicKey, sellerKeypair.publicKey);

    buyerItem0TokenAccount = await createAssociatedTokenAccount(connection, buyerKeypair, item0Mint.publicKey, buyerKeypair.publicKey);
    buyerItem1TokenAccount = await createAssociatedTokenAccount(connection, buyerKeypair, item1Mint.publicKey, buyerKeypair.publicKey);
    buyerItem2TokenAccount = await createAssociatedTokenAccount(connection, buyerKeypair, item2Mint.publicKey, buyerKeypair.publicKey);

    // now mint 10k tokens to seller's item0Mint ata and create the seller's currency ata
    const numTokens = 10000;
    tx = new Transaction().add(
        // item0 = token
        createMintToCheckedInstruction(
            item0Mint.publicKey,
            sellerItem0TokenAccount,
            sellerKeypair.publicKey,
            numTokens * 1e9,
            9
        ),
        // item1/2 = SFT
        createMintToCheckedInstruction(
            item1Mint.publicKey,
            sellerItem1TokenAccount,
            sellerKeypair.publicKey,
            numTokens,
            0
        ),
        createMintToCheckedInstruction(
            item2Mint.publicKey,
            sellerItem2TokenAccount,
            sellerKeypair.publicKey,
            numTokens,
            0
        ),
    );
    txid = await provider.sendAndConfirm(tx, [sellerKeypair]);
    console.log(`minted ${numTokens} tokens to seller's item token accounts`);
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

  it("creates a single item listing - unit type", async () => {

    let currencyMint = NATIVE_MINT;
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.5);

    // so, selling 9 tokens for 5 sol each
    let listingQty = new anchor.BN(9); // 9 items

    let [listingPda] = findBazaarListingPda(sellerAccountPda, ++sellerAccountListingIndex, bazaarProg.programId);
    let listingItem0Token = getAssociatedTokenAddressSync(item0Mint.publicKey, listingPda, true);
    let listingItem1Token = getAssociatedTokenAddressSync(item1Mint.publicKey, listingPda, true);

    let accounts = {
      listingDomain: listingDomainPda,
      seller: sellerKeypair.publicKey,
      sellerAccount: sellerAccountPda,
      keychain: sellerKeychainPda,
      listing: listingPda,
      currency: currencyMint,
      proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: item0Mint.publicKey,
      item0SellerToken: sellerItem0TokenAccount,
      item0ListingToken: listingItem0Token,
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

    // try to create the listing (will fail cause we're unit listing a token)
    tx = await bazaarProg.methods.createListing({price, listingType: {unit: {}}, itemQuantities: [listingQty]}).accounts(accounts).transaction();

    // can't unit list a token
    try {
      txid = await provider.sendAndConfirm(tx, [sellerKeypair], {skipPreflight: true});
      assert.fail("should have thrown an error");
    } catch (e) {
      // expected
    }

    // change the item to sft
    accounts = {...accounts,
      item0: item1Mint.publicKey,
      item0SellerToken: sellerItem1TokenAccount,
      item0ListingToken: listingItem1Token,
    }

    // create the listing - this time unit listting for sft
    tx = await bazaarProg.methods.createListing({price, listingType: {unit: {}}, itemQuantities: [listingQty]}).accounts(accounts).transaction();

    txid = await provider.sendAndConfirm(tx, [sellerKeypair], {skipPreflight: true});

    console.log(`created listing: ${listingPda.toBase58()}, txid `, txid);
    const listing = await bazaarProg.account.listing.fetch(listingPda);
    console.log("listing account: ", listing);

    expect(listing.accountVersion).to.equal(0);
    expect(listing.price.toNumber()).to.equal(price.toNumber());
    expect(listing.bump).is.greaterThan(0);
    expect(listing.currency.toBase58()).to.equal(currencyMint.toBase58());
    assert.isTrue('unit' in listing.listingType);
    assert(listing.items.length == 1);
    expect(listing.items[0].quantity.toNumber()).to.equal(listingQty.toNumber());
    expect(listing.items[0].itemMint.toBase58()).to.equal(item1Mint.publicKey.toBase58());
    expect(listing.items[0].itemToken.toBase58()).to.equal(listingItem1Token.toBase58());
    expect(listing.treasury.toBase58()).to.equal(treasury.publicKey.toBase58());
    expect(listing.listingIndex).to.equal(sellerAccountListingIndex);

    let purchaseQty = new anchor.BN(3);

    // now purchase
    tx = await bazaarProg.methods.buy(purchaseQty).accounts({
      buyer: buyerKeypair.publicKey,
      buyerCurrencyToken: null,
      listing: listingPda,
      sellerAccount: sellerAccountPda,
      currency: currencyMint,
      proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: item1Mint.publicKey,
      item0BuyerToken: buyerItem1TokenAccount,
      item0ListingToken: listingItem1Token,
      item1: null,
      item1BuyerToken: null,
      item1ListingToken: null,
      item2: null,
      item2BuyerToken: null,
      item2ListingToken: null,
      treasury: treasury.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    }).transaction();

    let buyerSolBalance = await getSolBalance(connection, buyerKeypair.publicKey);
    let sellerSolBalance = await getSolBalance(connection, sellerKeypair.publicKey);
    let treasurySolBalance = await getSolBalance(connection, treasury.publicKey);

    console.log(`buyer sol balance: ${buyerSolBalance}, seller sol balance: ${sellerSolBalance}, treasury sol balance: ${treasurySolBalance}`);

    txid = await provider.sendAndConfirm(tx, [buyerKeypair], {skipPreflight: true});
    console.log(`purchased listing: ${listingPda.toBase58()}, txid `, txid);
    console.log(`after purchase: buyer sol balance: ${buyerSolBalance}, seller sol balance: ${sellerSolBalance}, treasury sol balance: ${treasurySolBalance}`);

    // check listing item0 account amount = 9 - 3 = 6
    let tokenAmount = await connection.getTokenAccountBalance(listingItem1Token);
    expect(tokenAmount.value.amount).equals(listingQty.sub(purchaseQty).toString());

    // check buyer received items
    tokenAmount = await connection.getTokenAccountBalance(buyerItem1TokenAccount);
    expect(tokenAmount.value.amount).equals(purchaseQty.toString());

    // check buyer paid 3 * 0.5 = 1.5 sol, and seller got 1.5 sol
    let newBuyerSolBalance = await getSolBalance(connection, buyerKeypair.publicKey);
    let newSellerSolBalance = await getSolBalance(connection, sellerKeypair.publicKey);

    assert.isTrue(isWithinPercentageThreshold(buyerSolBalance - (0.5 * 3), newBuyerSolBalance, 5));
    assert.isTrue(isWithinPercentageThreshold(sellerSolBalance + (0.5 * 3), newSellerSolBalance, 5));

    let newTreasurySolBalance = await getSolBalance(connection, treasury.publicKey);

    // treasury didn't get any sol cause listing not closed
    expect(newTreasurySolBalance).is.equal(treasurySolBalance);
  });

  it("creates a double item listing - bag type", async () => {

    let currencyMint = NATIVE_MINT;
    let priceInSol = 5;
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * priceInSol);
    let item0Quantity = new anchor.BN(9 * 1e9); // 9 tokens
    let item1Quantity = new anchor.BN(2);       // 2 SFTs

    let [listingPda] = findBazaarListingPda(sellerAccountPda, ++sellerAccountListingIndex, bazaarProg.programId);
    let listingItem0Token = getAssociatedTokenAddressSync(item0Mint.publicKey, listingPda, true);
    let listingItem1Token = getAssociatedTokenAddressSync(item1Mint.publicKey, listingPda, true);

    let accounts = {
      listingDomain: listingDomainPda,
      seller: sellerKeypair.publicKey,
      sellerAccount: sellerAccountPda,
      keychain: sellerKeychainPda,
      listing: listingPda,
      currency: currencyMint,
      proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: item0Mint.publicKey,
      item0SellerToken: sellerItem0TokenAccount,
      item0ListingToken: listingItem0Token,
      item1: item1Mint.publicKey,
      item1SellerToken: sellerItem1TokenAccount,
      item1ListingToken: listingItem1Token,
      item2: null,
      item2SellerToken: null,
      item2ListingToken: null,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };

    // create the listing
    tx = await bazaarProg.methods.createListing({price, listingType: {bag: {}}, itemQuantities: [item0Quantity, item1Quantity]})
        .accounts(accounts).transaction();

    txid = await provider.sendAndConfirm(tx, [sellerKeypair], {skipPreflight: true});

    console.log(`created listing: ${listingPda.toBase58()}, txid `, txid);
    const listing = await bazaarProg.account.listing.fetch(listingPda);
    // console.log("listing account: ", listing);

    expect(listing.accountVersion).to.equal(0);
    expect(listing.bump).is.greaterThan(0);
    // expect(listing.price.toNumber()).to.equal(price.toNumber());
    expect(listing.currency.toBase58()).to.equal(currencyMint.toBase58());
    assert.isTrue('bag' in listing.listingType);
    assert(listing.items.length == 2);
    expect(listing.items[0].quantity.toNumber()).to.equal(item0Quantity.toNumber());
    expect(listing.items[1].quantity.toNumber()).to.equal(item1Quantity.toNumber());
    expect(listing.items[0].itemMint.toBase58()).to.equal(item0Mint.publicKey.toBase58());
    expect(listing.items[1].itemMint.toBase58()).to.equal(item1Mint.publicKey.toBase58());
    expect(listing.items[0].itemToken.toBase58()).to.equal(listingItem0Token.toBase58());
    expect(listing.items[1].itemToken.toBase58()).to.equal(listingItem1Token.toBase58());
    expect(listing.treasury.toBase58()).to.equal(treasury.publicKey.toBase58());

    // check the listing ata token amounts
    let tokenAmount = await connection.getTokenAccountBalance(listingItem0Token);
    expect(tokenAmount.value.amount).equals(item0Quantity.toString());
    tokenAmount = await connection.getTokenAccountBalance(listingItem1Token);
    expect(tokenAmount.value.amount).equals(item1Quantity.toString());

    // now make a purchase
    tx = await bazaarProg.methods.buy(new anchor.BN(1)).accounts({
      buyer: buyerKeypair.publicKey,
      buyerCurrencyToken: null,
      listing: listingPda,
      sellerAccount: sellerAccountPda,
      currency: currencyMint,
      proceedsToken: null,
      proceeds: sellerKeypair.publicKey,
      item0: item0Mint.publicKey,
      item0BuyerToken: buyerItem0TokenAccount,
      item0ListingToken: listingItem0Token,
      item1: item1Mint.publicKey,
      item1BuyerToken: buyerItem1TokenAccount,
      item1ListingToken: listingItem1Token,
      item2: null,
      item2BuyerToken: null,
      item2ListingToken: null,
      treasury: treasury.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    }).transaction();

    let buyerSolBalance = await getSolBalance(connection, buyerKeypair.publicKey);
    let sellerSolBalance = await getSolBalance(connection, sellerKeypair.publicKey);
    let treasurySolBalance = await getSolBalance(connection, treasury.publicKey);
    let buyerItem0TokenBalance = await connection.getTokenAccountBalance(buyerItem0TokenAccount);
    let buyerItem1TokenBalance = await connection.getTokenAccountBalance(buyerItem1TokenAccount);

    console.log(`buyer sol balance: ${buyerSolBalance}, seller sol balance: ${sellerSolBalance}, treasury sol balance: ${treasurySolBalance}`);

    txid = await provider.sendAndConfirm(tx, [buyerKeypair], {skipPreflight: true});
    console.log(`purchased listing: ${listingPda.toBase58()}, txid `, txid);

    let newBuyerSolBalance = await getSolBalance(connection, buyerKeypair.publicKey);
    let newSellerSolBalance = await getSolBalance(connection, sellerKeypair.publicKey);
    let newTreasurySolBalance = await getSolBalance(connection, treasury.publicKey);
    console.log(`after purchase: buyer sol balance: ${newBuyerSolBalance}, seller sol balance: ${newSellerSolBalance}, treasury sol balance: ${newTreasurySolBalance}`);

    // check that the listing ata accounts got closed and the listing account too
    let listingAccount = await connection.getAccountInfo(listingPda);
    let listingItem0TokenAccount = await connection.getAccountInfo(listingItem0Token);
    let listingItem1TokenAccount = await connection.getAccountInfo(listingItem1Token);
    expect(listingAccount).to.be.null;
    expect(listingItem0TokenAccount).to.be.null;
    expect(listingItem1TokenAccount).to.be.null;

    // check buyer received items
    let newBuyerItem0TokenBalance = await connection.getTokenAccountBalance(buyerItem0TokenAccount);
    let newBuyerItem1TokenBalance = await connection.getTokenAccountBalance(buyerItem1TokenAccount);
    expect(newBuyerItem0TokenBalance.value.uiAmount).equals(buyerItem0TokenBalance.value.uiAmount + item0Quantity.div(new anchor.BN(1e9)).toNumber());
    expect(newBuyerItem1TokenBalance.value.uiAmount).equals(buyerItem1TokenBalance.value.uiAmount + item1Quantity.toNumber());

    // check buyer paid 5 sol, and seller got 5 sol

    assert.isTrue(isWithinPercentageThreshold(buyerSolBalance - priceInSol, newBuyerSolBalance, 5));
    assert.isTrue(isWithinPercentageThreshold(sellerSolBalance + priceInSol, newSellerSolBalance, 5));

    // treasury got some sol
    expect(newTreasurySolBalance).is.greaterThan(treasurySolBalance);
  });

});

