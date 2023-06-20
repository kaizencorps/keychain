import * as anchor from "@project-serum/anchor";
import {Program} from "@project-serum/anchor";

import {AccountMeta, Keypair, PublicKey, SystemProgram, Transaction} from "@solana/web3.js";
import {
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
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {expect} from "chai";

import {Keychain} from "../target/types/keychain";
import {Yardsale} from "../target/types/yardsale";
import {HeliusConnectionWrapper} from "./HeliusConnectionWrapper";
import {getRandomFakeNftMetadata, loadPublicKeysFromFile} from "./compression-helpers";
import {MetadataArgs, TokenProgramVersion} from "@metaplex-foundation/mpl-bubblegum";
import {TokenStandard} from "@metaplex-foundation/mpl-token-metadata";
import {createMintCompressedNftTx, fetchAssetId} from "./compression";
import {
  ConcurrentMerkleTreeAccount,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID
} from "@solana/spl-account-compression";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "@metaplex-foundation/mpl-bubblegum";



///// since this works with cnfts, needs to be done on devnet

function randomName() {
  let name = Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
  return name.toLowerCase();
}

// for setting up the keychain
// const domain = randomName();
const domain = 'testdomain1';
const stacheid = 'test123';
const treasury = anchor.web3.Keypair.generate();
const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

let currencyMint: Keypair = null;
let buyerCurrencyTokenAcct: PublicKey = null;
let sellerCurrencyTokenAcct: PublicKey = null;
let txid;
let tx;
let assetId;

describe("yardsale compressed NFTs", () => {

  let provider = anchor.AnchorProvider.env();
  const payer = provider.wallet.publicKey;

  const connection = provider.connection;

  let keychainProgram = anchor.workspace.Keychain as Program<Keychain>;
  let yardsaleProgram = anchor.workspace.Yardsale as Program<Yardsale>;

  console.log(`>>>>> connection: ${connection.rpcEndpoint} <<<<<<<`);
  console.log('keychainProgram: ', keychainProgram.programId.toBase58());
  console.log('yardsaleProgram: ', yardsaleProgram.programId.toBase58());

  keychainProgram = new Program<Keychain>(keychainProgram.idl, keychainProgram.programId, provider);
  yardsaleProgram = new Program<Yardsale>(yardsaleProgram.idl, yardsaleProgram.programId, provider);

  let userKeychainPda: PublicKey;
  let domainPda: PublicKey;
  let proceedsAccount: Keypair = Keypair.generate();
  let buyer: Keypair = Keypair.generate();

  console.log(`\n\n...>>> user: ${provider.wallet.publicKey.toBase58()}`);


  let treeAddress: PublicKey = null;
  let treeAuthority: PublicKey = null;
  let collectionMint: PublicKey = null;
  let collectionMetadataAccount: PublicKey = null;
  let collectionMasterEditionAccount: PublicKey = null;



  it("sets up testing env", async () => {


    // load the stored PublicKeys for ease of use
    let keys = loadPublicKeysFromFile();

    // ensure the primary script (to create the collection) was already run
    if (!keys?.collectionMint || !keys?.treeAddress)
      return console.warn("No local keys were found. Please run the `index` script");

    treeAddress = keys.treeAddress;
    treeAuthority= keys.treeAuthority;
    collectionMint= keys.collectionMint;
    collectionMetadataAccount= keys.collectionMetadataAccount;
    collectionMasterEditionAccount= keys.collectionMasterEditionAccount;

    console.log("==== Local PublicKeys loaded ====");
    console.log("Tree address:", treeAddress.toBase58());
    console.log("Tree authority:", treeAuthority.toBase58());
    console.log("Collection mint:", collectionMint.toBase58());
    console.log("Collection metadata:", collectionMetadataAccount.toBase58());
    console.log("Collection master edition:", collectionMasterEditionAccount.toBase58());


    // create the keychain domain + user's keychain

    // our domain account
    [domainPda] = findDomainPda(domain, keychainProgram.programId);
    const [domainStatePda, domainStatePdaBump] = findDomainStatePda(domain, keychainProgram.programId);

    // our keychain accounts
    [userKeychainPda] = findKeychainPda(stacheid, domain, keychainProgram.programId);
    const [userKeychainStatePda] = findKeychainStatePda(userKeychainPda, domain, keychainProgram.programId);
    // the "pointer" keychain key account
    const [userKeychainKeyPda] = findKeychainKeyPda(provider.wallet.publicKey, domain, keychainProgram.programId);

    // first create the domain if it doesn't exist
    let acct = await keychainProgram.account.currentDomain.fetchNullable(domainPda);

    if (!acct) {
      console.log("domain doesn't exist, creating...");
      tx = await keychainProgram.methods.createDomain(domain, renameCost).accounts({
        domain: domainPda,
        domainState: domainStatePda,
        authority: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
        treasury: treasury.publicKey
      }).transaction();

      txid = await provider.sendAndConfirm(tx);
      await connection.confirmTransaction(txid, 'finalized');

      console.log(`created keychain domain tx: ${txid}`);
    } else {
      console.log("domain exists, skipping...");
    }

    acct = await keychainProgram.account.currentKeyChain.fetchNullable(userKeychainPda);

    if (!acct) {
      console.log("keychain doesn't exist, creating...");

      // then create the keychain
      tx = await keychainProgram.methods.createKeychain(stacheid).accounts({
        keychain: userKeychainPda,
        keychainState: userKeychainStatePda,
        keychainKey: userKeychainKeyPda,
        domain: domainPda,
        authority: provider.wallet.publicKey,
        wallet: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).transaction();

      txid = await provider.sendAndConfirm(tx);
      await connection.confirmTransaction(txid, 'finalized');
      console.log(`created keychain for ${stacheid}. tx: ${txid}`);
    } else {
      console.log("keychain exists, skipping...");
    }

  });

  it("list and buy an nft in sol", async () => {


    // for debugging
    let assetIdKey = Keypair.generate().publicKey;

    let [listingPda] = findListingPda(assetIdKey, stacheid, domain, yardsaleProgram.programId);

    console.log('creating compressed nft listing w/listingPda: ', listingPda.toBase58());
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.0001);

    console.log('bubblegumprogram: ', BUBBLEGUM_PROGRAM_ID.toBase58());
    console.log('spl account compression program: ', SPL_ACCOUNT_COMPRESSION_PROGRAM_ID.toBase58());
    console.log('system program: ', SystemProgram.programId.toBase58());
    console.log('logwrapper program: ', SPL_NOOP_PROGRAM_ID.toBase58());

    console.log('domainPda: ', domainPda.toBase58());
    console.log('keychainPda: ', userKeychainPda.toBase58());

    // list the compressed nft
    tx = await yardsaleProgram.methods.listCompressedNft(
            [...assetIdKey.toBytes()],
            [...assetIdKey.toBytes()],
            [...assetIdKey.toBytes()],
            new anchor.BN(22),
            1,
            price,
            assetIdKey,
            )
        .accounts({
          domain: domainPda,
          keychain: userKeychainPda,
          listing: listingPda,
          // treeAuthority: domainPda,
          leafOwner: payer,
          merkleTree: domainPda,

          logWrapper: SPL_NOOP_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          systemProgram: SystemProgram.programId,

          // domain: domainPda,
          // keychain: userKeychainPda,
          // listing: listingPda,
          // treeAuthority,
          // leafOwner: payer,
          // merkleTree: treeAddress,
          // logWrapper: SPL_NOOP_PROGRAM_ID,
          // bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          // compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          // systemProgram: SystemProgram.programId,
        })
        // .remainingAccounts(proofPath)
        .transaction();

    txid = await provider.sendAndConfirm(tx);

    console.log('confirming list compressed NFT tx: ', txid);

    await connection.confirmTransaction(txid, "finalized");

    console.log(`listed compressed nft: ${assetId} for ${price} sol: ${txid}, owner listing pda: ${listingPda.toBase58()}`);

    // check that the nft is in the listing account
    /*
    let listing = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log(`listing: ${JSON.stringify(listing, null, 2)}`);

    let rpcResp = await connectionWrapper
        .getAssetsByOwner({
          ownerAddress: listingPda.toBase58(),
        });

    console.log('listing pda assets: ', rpcResp);

     */


  });




});



