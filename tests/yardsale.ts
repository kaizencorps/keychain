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
  createNFT, createTokenMint,
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

let currencyMint: Keypair = null;
let buyerCurrencyTokenAcct: PublicKey = null;
let sellerCurrencyTokenAcct: PublicKey = null;

console.log(`--> yardsale program id: ${yardsaleProgram.programId.toBase58()} \n --> keychain program id: ${keychainProgram.programId.toBase58()}`);

describe("yardsale", () => {
  const provider = anchor.AnchorProvider.env();
  const connection = provider.connection;
  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  let userKeychainPda: PublicKey;
  let domainPda: PublicKey;
  let proceedsAccount: Keypair = Keypair.generate();
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
    for (let i = 0; i < 5; i++) {
      const nft = await createNFT(provider);
      console.log(`created nft: ${nft.publicKey.toBase58()}`);
      nfts.push(nft.publicKey);
    }

    // create a currency mint
    currencyMint = await createTokenMint(connection, buyer, provider.wallet.publicKey);

    // currency atas for the buyer / seller
    buyerCurrencyTokenAcct = await createAssociatedTokenAccount(connection, buyer, currencyMint.publicKey, buyer.publicKey);
    sellerCurrencyTokenAcct = getAssociatedTokenAddressSync(currencyMint.publicKey, provider.wallet.publicKey);
    // sellerCurrencyTokenAcct = await createAssociatedTokenAccount(connection, buyer, currencyMint.publicKey, provider.wallet.publicKey);

    // now mint 10k tokens to buyer's currency ata and create the seller's currency ata
    const numTokens = 10000;
    const tx = new Transaction().add(
        createMintToCheckedInstruction(
            currencyMint.publicKey,
            buyerCurrencyTokenAcct,
            provider.wallet.publicKey,
            numTokens * 1e9,
            9
        ),
        createAssociatedTokenAccountInstruction(provider.wallet.publicKey, sellerCurrencyTokenAcct, provider.wallet.publicKey, currencyMint.publicKey)
    );

    let txid = await provider.sendAndConfirm(tx);

    console.log(`minted ${numTokens} tokens to buyer's ata: ${buyerCurrencyTokenAcct.toBase58()} \n`);

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
    txid = await keychainProgram.methods.createDomain(domain, renameCost).accounts({
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
    let listingItemToken = getAssociatedTokenAddressSync(nft, listingPda, true);


    // list for .5 sol
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.5);
    let txid = await yardsaleProgram.methods.listItem(price).accounts({
      domain: domainPda,
      keychain: userKeychainPda,
      authority: provider.wallet.publicKey,
      item: nft,
      authorityItemToken: fromItemToken,
      listing: listingPda,
      listingItemToken: listingItemToken,
      currency: NATIVE_MINT,
      proceeds: proceedsAccount.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      proceedsToken: null       // optional accounts set to null
    }).rpc();

    await connection.confirmTransaction(txid, "confirmed");
    console.log(`listed nft: ${nft.toBase58()} for ${price} sol: ${txid}`);

    // check that the nft is in the listing account
    let listing = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log(`listing: ${JSON.stringify(listing, null, 2)}`);

    let tokenAmount = await connection.getTokenAccountBalance(listingItemToken);
    assert.equal(tokenAmount.value.amount, 1, `nft should be in the item account: ${listingItemToken.toBase58()}`);

    // check the listing amount
    let listingPrice = listing.price.toNumber();
    assert.equal(listingPrice, price, `listing price should be ${price.toNumber()}`);
    assert.equal(listing.item.toBase58(), nft.toBase58());
    assert.equal(listing.treasury.toBase58(), treasury.publicKey.toBase58());
    expect(listing.bump).to.exist;
    assert.equal(domain, listing.domain);
    assert.equal(username, listing.keychain);
    assert.equal(listing.currency.toBase58(), NATIVE_MINT.toBase58());
    assert.equal(listing.proceeds.toBase58(), proceedsAccount.publicKey.toBase58());

    let treasuryBalanceBefore = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance before: ${treasuryBalanceBefore}`);

    let buyerItemToken = getAssociatedTokenAddressSync(nft, buyer.publicKey, false);

    let tx = new Transaction().add(
        createAssociatedTokenAccountInstruction(buyer.publicKey, buyerItemToken, buyer.publicKey, nft)
    );

    // let tx = new Transaction();

    let proceedsBalanceBefore = await connection.getBalance(proceedsAccount.publicKey);

    // now we buy it
    let ix = await yardsaleProgram.methods.purchaseItem().accounts({
      listing: listingPda,
      item: nft,
      listingItemToken,
      authorityItemToken: buyerItemToken,
      currency: NATIVE_MINT,
      proceeds: proceedsAccount.publicKey,
      authority: buyer.publicKey,
      treasury: treasury.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      // optional accounts set to null
      proceedsToken: null,
      authorityCurrencyToken: null
    }).instruction();

    tx.add(ix);
    txid = await provider.sendAndConfirm(tx, [buyer]);

    console.log(`bought nft: ${nft.toBase58()} for ${price} sol: ${txid}`);

    // check that the treasury got the sol from closing the listing
    let treasuryBalanceAfter = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance after: ${treasuryBalanceAfter}`);
    assert.ok(treasuryBalanceAfter >= treasuryBalanceBefore, "treasury should have more sol");

    // check that the seller got the proceeds
    let proceedsBalanceAfter = await connection.getBalance(proceedsAccount.publicKey);
    assert.ok(proceedsBalanceAfter > proceedsBalanceBefore, "proceeds should have more sol");

    // find out how much proceeds the seller got
    const proceedsAmount = proceedsBalanceAfter - proceedsBalanceBefore;
    assert.equal(proceedsAmount, price.toNumber(), "proceeds should be equal to the listing price");

    // check that the buyer got the nft
    let buyerItemTokenBalance = await connection.getTokenAccountBalance(buyerItemToken);
    assert.equal(buyerItemTokenBalance.value.amount, 1, `nft should be in the item account: ${buyerItemToken.toBase58()}`);

    // check that listing account got closed
    listing = await yardsaleProgram.account.listing.fetchNullable(listingPda);
    expect(listing).to.be.null;

    // check that the listing's item token account got closed
    let listingTokenAccount = await connection.getAccountInfo(listingItemToken);
    expect(listingTokenAccount).to.be.null;

  });

  it("list and buy an nft in spl", async () => {

    // the nft to list
    let nft = nfts[1];
    // user's nft token account
    let fromItemToken = getAssociatedTokenAddressSync(nft, provider.wallet.publicKey);
    let [listingPda] = findListingPda(nft, username, domain, yardsaleProgram.programId);
    // listing's ata
    let listingItemToken = getAssociatedTokenAddressSync(nft, listingPda, true);

    // list for 500 currencyMints
    let price = 500;
    let priceBN = new anchor.BN(1e9 * price);
    let txid = await yardsaleProgram.methods.listItem(priceBN).accounts({
      domain: domainPda,
      keychain: userKeychainPda,
      authority: provider.wallet.publicKey,
      item: nft,
      authorityItemToken: fromItemToken,
      listing: listingPda,
      listingItemToken: listingItemToken,
      currency: currencyMint.publicKey,
      proceedsToken: sellerCurrencyTokenAcct,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      proceeds: null,     // null since priced in spl not sol
    }).rpc();

    await connection.confirmTransaction(txid, "confirmed");
    console.log(`listed nft: ${nft.toBase58()} for ${price} spl tokens: ${currencyMint.publicKey}: ${txid}`);

    // check that the nft is in the listing account
    let listing = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log(`listing: ${JSON.stringify(listing, null, 2)}`);

    let tokenAmount = await connection.getTokenAccountBalance(listingItemToken);
    assert.equal(tokenAmount.value.amount, 1, `nft should be in the item account: ${listingItemToken.toBase58()}`);

    // check the listing amount
    let listingPrice = listing.price.toNumber();
    assert.equal(listingPrice, priceBN, `listing price should be ${price}`);
    assert.equal(listing.item.toBase58(), nft.toBase58());
    assert.equal(listing.treasury.toBase58(), treasury.publicKey.toBase58());
    expect(listing.bump).to.exist;
    assert.equal(domain, listing.domain);
    assert.equal(username, listing.keychain);
    assert.equal(listing.currency.toBase58(), currencyMint.publicKey.toBase58());
    assert.equal(listing.proceeds.toBase58(), sellerCurrencyTokenAcct.toBase58());

    let treasuryBalanceBefore = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance before: ${treasuryBalanceBefore}`);

    let buyerItemToken = getAssociatedTokenAddressSync(nft, buyer.publicKey, false);

    // create buyer's item token account
    let tx = new Transaction().add(
        createAssociatedTokenAccountInstruction(buyer.publicKey, buyerItemToken, buyer.publicKey, nft)
    );

    // let tx = new Transaction();

    let proceedsBalanceBefore = (await connection.getTokenAccountBalance(sellerCurrencyTokenAcct)).value.uiAmount;

    // now we buy it
    let ix = await yardsaleProgram.methods.purchaseItem().accounts({
      listing: listingPda,
      item: nft,
      listingItemToken,
      authorityItemToken: buyerItemToken,
      currency: currencyMint.publicKey,
      proceedsToken: sellerCurrencyTokenAcct,
      authority: buyer.publicKey,
      authorityCurrencyToken: buyerCurrencyTokenAcct,
      treasury: treasury.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      // optional accounts set to null
      proceeds: null,
    }).instruction();

    tx.add(ix);
    txid = await provider.sendAndConfirm(tx, [buyer]);

    console.log(`bought nft: ${nft.toBase58()} for ${price} sol: ${txid}`);

    // check that the treasury got the sol from closing the listing
    let treasuryBalanceAfter = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance after: ${treasuryBalanceAfter}`);
    assert.ok(treasuryBalanceAfter >= treasuryBalanceBefore, "treasury should have more sol");

    // check that the seller got the proceeds
    let proceedsBalanceAfter = (await connection.getTokenAccountBalance(sellerCurrencyTokenAcct)).value.uiAmount;
    assert.ok(proceedsBalanceAfter > proceedsBalanceBefore, "proceeds should have more currencyMints");

    console.log(`proceeds before: ${proceedsBalanceBefore}`);
    console.log(`proceeds after: ${proceedsBalanceAfter}`);

    // find out how much proceeds the seller got
    const proceedsAmount = proceedsBalanceAfter - proceedsBalanceBefore;
    assert.equal(proceedsAmount, price, "proceeds should be equal to the listing price");

    // check that the buyer got the nft
    let buyerItemTokenBalance = await connection.getTokenAccountBalance(buyerItemToken);
    assert.equal(buyerItemTokenBalance.value.amount, 1, `nft should be in the item account: ${buyerItemToken.toBase58()}`);

    // check that listing account got closed
    listing = await yardsaleProgram.account.listing.fetchNullable(listingPda);
    expect(listing).to.be.null;

    // check that the listing's item token account got closed
    let listingTokenAccount = await connection.getAccountInfo(listingItemToken);
    expect(listingTokenAccount).to.be.null;

  });

  it("deslist item", async () => {

    // the nft to list
    let nft = nfts[2];
    // user's nft token account
    let fromItemToken = getAssociatedTokenAddressSync(nft, provider.wallet.publicKey);
    let [listingPda] = findListingPda(nft, username, domain, yardsaleProgram.programId);
    // listing's ata
    let listingItemToken = getAssociatedTokenAddressSync(nft, listingPda, true);

    // list for 500 currencyMints
    let price = 500;
    let priceBN = new anchor.BN(1e9 * price);
    let txid = await yardsaleProgram.methods.listItem(priceBN).accounts({
      domain: domainPda,
      keychain: userKeychainPda,
      authority: provider.wallet.publicKey,
      item: nft,
      authorityItemToken: fromItemToken,
      listing: listingPda,
      listingItemToken: listingItemToken,
      currency: currencyMint.publicKey,
      proceedsToken: sellerCurrencyTokenAcct,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      proceeds: null,     // null since priced in spl not sol
    }).rpc();

    await connection.confirmTransaction(txid, "confirmed");
    console.log(`listed nft: ${nft.toBase58()} for ${price} spl tokens: ${currencyMint.publicKey}: ${txid}`);

    // check that the nft is in the listing account
    let listing = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log(`listing: ${JSON.stringify(listing, null, 2)}`);

    let tokenAmount = await connection.getTokenAccountBalance(listingItemToken);
    assert.equal(tokenAmount.value.amount, 1, `nft should be in the item account: ${listingItemToken.toBase58()}`);

    let treasuryBalanceBefore = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance before: ${treasuryBalanceBefore}`);

    let proceedsBalanceBefore = (await connection.getTokenAccountBalance(sellerCurrencyTokenAcct)).value.uiAmount;

    // now seller delists it
    txid = await yardsaleProgram.methods.delistItem().accounts({
      listing: listingPda,
      item: nft,
      keychain: userKeychainPda,
      authorityItemToken: fromItemToken,
      listingItemToken: listingItemToken,
      authority: provider.wallet.publicKey,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).rpc();

    console.log(`delisted nft: ${nft.toBase58()}: ${txid}`);

    // check that the treasury remains the same
    let treasuryBalanceAfter = await connection.getBalance(treasury.publicKey);
    console.log(`treasury balance after: ${treasuryBalanceAfter}`);
    assert.ok(treasuryBalanceAfter == treasuryBalanceBefore, "treasury should be same");

    let proceedsBalanceAfter = (await connection.getTokenAccountBalance(sellerCurrencyTokenAcct)).value.uiAmount;
    assert.ok(proceedsBalanceAfter == proceedsBalanceBefore, "proceeds should be same");

    // check that listing account got closed
    listing = await yardsaleProgram.account.listing.fetchNullable(listingPda);
    expect(listing).to.be.null;

    // check that the listing's item token account got closed
    let listingTokenAccount = await connection.getAccountInfo(listingItemToken);
    expect(listingTokenAccount).to.be.null;
  });

});



