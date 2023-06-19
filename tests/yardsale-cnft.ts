import * as anchor from "@project-serum/anchor";
import {Program} from "@project-serum/anchor";

import {AccountMeta, Keypair, PublicKey, SystemProgram, Transaction} from "@solana/web3.js";
import {
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda,
  findListingPda, findListingPdaByName,
} from "./utils";
import * as assert from "assert";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID, transfer,
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



///// since this works with cnfts, needs to be done on devnet w/helius (or das supported rpc)

/// this test expects a tree it can mint from to be set up already using messhall/compression/createTree

function randomName() {
  let name = Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
  return name.toLowerCase();
}

// for setting up the keychain
// const domain = randomName();
const domain = 'testdomain1';
const stacheid = 'test123';
let treasury = anchor.web3.Keypair.generate().publicKey;
const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);

let currencyMint: Keypair = null;
let buyerCurrencyTokenAcct: PublicKey = null;
let sellerCurrencyTokenAcct: PublicKey = null;
let txid;
let tx;
let assetId;
let assetId2;

let buyer: Keypair = Keypair.generate();

describe("yardsale compressed NFTs", () => {
  let provider = anchor.AnchorProvider.env();
  const payer = provider.wallet.publicKey;

  const RPC_URL = 'https://rpc-devnet.helius.xyz/?api-key=df2f8e0d-099d-4110-b63e-7b5f6a53673e';

  let baseRpc = RPC_URL.substring(0, RPC_URL.indexOf('?') + 1);
  console.log('baseRpc: ', baseRpc);

  const connectionWrapper = new HeliusConnectionWrapper(RPC_URL);
  provider = new anchor.AnchorProvider(connectionWrapper, provider.wallet, {});

  let keychainProgram = anchor.workspace.Keychain as Program<Keychain>;
  let yardsaleProgram = anchor.workspace.Yardsale as Program<Yardsale>;

  console.log('keychainProgram: ', keychainProgram.programId.toBase58());
  console.log('yardsaleProgram: ', yardsaleProgram.programId.toBase58());

  keychainProgram = new Program<Keychain>(keychainProgram.idl, keychainProgram.programId, provider);
  yardsaleProgram = new Program<Yardsale>(yardsaleProgram.idl, yardsaleProgram.programId, provider);

  let userKeychainPda: PublicKey;
  let domainPda: PublicKey;
  let proceedsAccount: Keypair = Keypair.generate();
  let buyer: Keypair = Keypair.generate();

  console.log(`\n\n...>>> user: ${provider.wallet.publicKey.toBase58()}`);


  it("sets up testing env", async () => {

    // mint a bunch of compressed nfts - all this from the compressed-nfts repo

    // load the stored PublicKeys for ease of use
    let keys = loadPublicKeysFromFile();

    // ensure the primary script (to create the collection) was already run
    if (!keys?.collectionMint || !keys?.treeAddress)
      return console.warn("No local keys were found. Please run the `createTree` script in the messhall/compression");

    const treeAddress: PublicKey = keys.treeAddress;
    const treeAuthority: PublicKey = keys.treeAuthority;
    const collectionMint: PublicKey = keys.collectionMint;
    const collectionMetadataAccount: PublicKey = keys.collectionMetadataAccount;
    const collectionMasterEditionAccount: PublicKey = keys.collectionMasterEditionAccount;

    console.log("==== Local PublicKeys loaded ====");
    console.log("Tree address:", treeAddress.toBase58());
    console.log("Tree authority:", treeAuthority.toBase58());
    console.log("Collection mint:", collectionMint.toBase58());
    console.log("Collection metadata:", collectionMetadataAccount.toBase58());
    console.log("Collection master edition:", collectionMasterEditionAccount.toBase58());


    // see if the user already has any assets
    let rpcResp = await connectionWrapper
        .getAssetsByOwner({
          ownerAddress: payer.toBase58(),
        });

    // first, mint a compressed nft from the tree

      const data = getRandomFakeNftMetadata(payer);
      const compressedNFTMetadata: MetadataArgs = {
        ...data,
        editionNonce: 0,
        uses: null,
        collection: null,
        primarySaleHappened: false,
        sellerFeeBasisPoints: 0,
        isMutable: false,
        // values taken from the Bubblegum package
        tokenProgramVersion: TokenProgramVersion.Original,
        // @ts-ignore
        tokenStandard: TokenStandard.NonFungible,
      };

      // fully mint a single compressed NFT to the payer
      console.log(`Minting a single compressed NFT to ${payer.toBase58()}...`);

      tx = createMintCompressedNftTx(
          connectionWrapper,
          payer,
          treeAddress,
          collectionMint,
          collectionMetadataAccount,
          collectionMasterEditionAccount,
          compressedNFTMetadata,
          // mint to this specific wallet (in this case, the tree owner aka `payer`)
          payer,
      );

      txid = await provider.sendAndConfirm(tx);

      console.log("confirming mint tx: ", txid);

      await provider.connection.confirmTransaction(txid, 'finalized');

      // get the asset id
      assetId = await fetchAssetId(txid, treeAddress, connectionWrapper);

      console.log(`Minted a single compressed NFT to ${payer.toBase58()}, txid: ${txid}, assetId: ${assetId}`);


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
        treasury: treasury
      }).transaction();

      txid = await provider.sendAndConfirm(tx);
      await connectionWrapper.confirmTransaction(txid, 'finalized');

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
      await connectionWrapper.confirmTransaction(txid, 'finalized');
      console.log(`created keychain for ${stacheid}. tx: ${txid}`);
    } else {
      console.log("keychain exists, skipping...");
    }

    console.log("--- setup complete! ---\n\n");

  });

  it("list a cnft in sol", async () => {

    console.log('fetching asset info for assetId: ', assetId);
    let assetIdKey = new PublicKey(assetId);

    // some of this stuff is redundant from the previous test, but demoing how to do it
    let asset = await connectionWrapper.getAsset(assetIdKey);
    console.log('fetched asset: ', asset);
    let assetProof = await connectionWrapper.getAssetProof(assetIdKey);
    let treeAddress = new PublicKey(asset.compression.tree)
    let treeAccount = await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connectionWrapper,
        treeAddress
    );
    const treeAuthority = treeAccount.getAuthority();

    /*
    const [treeAuthority, _bump2] = anchor.web3.PublicKey.findProgramAddressSync(
        [treeAddress.toBuffer()],
        BUBBLEGUM_PROGRAM_ID,
    );

     */

    const canopyDepth = treeAccount.getCanopyDepth();

    // get "proof path" from asset proof, these are the accounts that need to be passed to the program as remaining accounts
    // may also be empty if tree is small enough, and canopy depth is large enough
    const proofPath: AccountMeta[] = assetProof.proof
        .map((node: string) => ({
          pubkey: new PublicKey(node),
          isSigner: false,
          isWritable: false,
        }))
        .slice(0, assetProof.proof.length - (!!canopyDepth ? canopyDepth : 0))

    console.log(`canopy depth: ${canopyDepth}, asset proof.proof.length: ${assetProof.proof.length}. proof path length: ${proofPath.length}`);

    // get root, data hash, creator hash, nonce, and index from asset and asset proof
    const root = [...new PublicKey(assetProof.root.trim()).toBytes()];
    const dataHash = [
      ...new PublicKey(asset.compression.data_hash.trim()).toBytes(),
    ];
    const creatorHash = [
      ...new PublicKey(asset.compression.creator_hash.trim()).toBytes(),
    ];
    const nonce = asset.compression.leaf_id;
    const index = asset.compression.leaf_id;

    let [listingPda] = findListingPda(assetIdKey, stacheid, domain, yardsaleProgram.programId);

    console.log('creating compressed nft listing w/listingPda: ', listingPda.toBase58());
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.00001);

    console.log('bubblegumprogram: ', BUBBLEGUM_PROGRAM_ID.toBase58());
    console.log('spl account compression program: ', SPL_ACCOUNT_COMPRESSION_PROGRAM_ID.toBase58());
    console.log('system program: ', SystemProgram.programId.toBase58());
    console.log('logwrapper program: ', SPL_NOOP_PROGRAM_ID.toBase58());

    console.log('domainPda: ', domainPda.toBase58());
    console.log('keychainPda: ', userKeychainPda.toBase58());


    // list the compressed nft
    tx = await yardsaleProgram.methods.listCompressedNft(
          assetIdKey,
          root,
          dataHash,
          creatorHash,
          new anchor.BN(nonce),
          index,
          price
        )
        .accounts({
          domain: domainPda,
          keychain: userKeychainPda,
          listing: listingPda,
          currency: NATIVE_MINT,
          proceedsToken: null,
          proceeds: payer,
          treeAuthority,
          leafOwner: payer,
          merkleTree: treeAddress,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(proofPath)
        .transaction();

    txid = await provider.sendAndConfirm(tx);

    console.log('confirming list compressed NFT tx: ', txid);

    await connectionWrapper.confirmTransaction(txid, "finalized");

    console.log(`listed compressed nft: ${assetId} for ${price} sol: ${txid}, owner listing pda: ${listingPda.toBase58()}`);

    // check that the nft is in the listing account
    let listing = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log(`listing: ${JSON.stringify(listing, null, 2)}`);

    let rpcResp = await connectionWrapper
        .getAssetsByOwner({
          ownerAddress: listingPda.toBase58(),
        });

    console.log('listing pda assets: ', rpcResp);
    expect(rpcResp.total).to.equal(1);
    expect(rpcResp.items.length).to.equal(1);
    expect(rpcResp.items[0].id === assetId, "listing pda should have assetId");

    // get the listing and make sure the asset id is the item
    let listingAccount = await yardsaleProgram.account.listing.fetch(listingPda);
    console.log('listing account: ', listingAccount);
    expect(listingAccount.item.toBase58()).to.equal(assetId);
    // there's no item token, so it just gets set to the asset id too
    expect(listingAccount.itemToken.toBase58()).to.equal(assetId);
    expect(listingAccount.price.toNumber()).to.equal(price.toNumber());
    expect(listingAccount.currency.toBase58()).to.equal(NATIVE_MINT.toBase58());
    expect(listingAccount.proceeds.toBase58()).to.equal(payer.toBase58());
    expect('compressed' in listingAccount.itemType, 'item type should be compressed');

    // set the proper treasury to be used for purchasing below
    treasury = listingAccount.treasury;
  });

  it("delist and relist a cnft in sol", async () => {

    let assetIdKey = new PublicKey(assetId);

    // some of this stuff is redundant from the previous test, but demoing how to do it
    let asset = await connectionWrapper.getAsset(assetIdKey);
    let assetProof = await connectionWrapper.getAssetProof(assetIdKey);
    let treeAddress = new PublicKey(asset.compression.tree)
    let treeAccount = await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connectionWrapper,
        treeAddress
    );
    let treeAuthority = treeAccount.getAuthority();
    let canopyDepth = treeAccount.getCanopyDepth();

    // get "proof path" from asset proof, these are the accounts that need to be passed to the program as remaining accounts
    // may also be empty if tree is small enough, and canopy depth is large enough
    let proofPath: AccountMeta[] = assetProof.proof
        .map((node: string) => ({
          pubkey: new PublicKey(node),
          isSigner: false,
          isWritable: false,
        }))
        .slice(0, assetProof.proof.length - (!!canopyDepth ? canopyDepth : 0))

    console.log(`canopy depth: ${canopyDepth}, asset proof.proof.length: ${assetProof.proof.length}. proof path length: ${proofPath.length}`);

    // get root, data hash, creator hash, nonce, and index from asset and asset proof
    let root = [...new PublicKey(assetProof.root.trim()).toBytes()];
    let dataHash = [
      ...new PublicKey(asset.compression.data_hash.trim()).toBytes(),
    ];
    let creatorHash = [
      ...new PublicKey(asset.compression.creator_hash.trim()).toBytes(),
    ];
    let nonce = asset.compression.leaf_id;
    let index = asset.compression.leaf_id;

    let [listingPda] = findListingPda(assetIdKey, stacheid, domain, yardsaleProgram.programId);

    // delist this bitch
    tx = await yardsaleProgram.methods.delistCnft(
            root,
            dataHash,
            creatorHash,
            new anchor.BN(nonce),
            index,
        )
        .accounts({
          listing: listingPda,
          keychain: userKeychainPda,
          authority: payer,
          treeAuthority,
          merkleTree: treeAddress,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(proofPath)
        .transaction();

    txid = await provider.sendAndConfirm(tx);
    console.log('confirming delist compressed NFT tx: ', txid);
    await connectionWrapper.confirmTransaction(txid, "finalized");
    console.log(`delisted compressed nft: ${assetId}: ${txid}`);

    // confirm it's back in payer's possession
    let rpcResp = await connectionWrapper
        .getAssetsByOwner({
          ownerAddress: payer.toBase58(),
        });

    const foundAssets = rpcResp.items.filter((item) => item.id === assetId);
    expect(foundAssets.length).to.equal(1);

    // confirm that listing was destroyed
    let listingAccount = await yardsaleProgram.account.listing.fetchNullable(listingPda);
    expect(listingAccount).to.be.null;

    //////////////////////////////////// now we relist it
    //////////////////////////////////// now we relist it
    //////////////////////////////////// now we relist it

    console.log('relisting compressed nft listing w/listingPda: ', listingPda.toBase58());
    let price = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.00001);

    // recalculate the proofs and shit
    asset = await connectionWrapper.getAsset(assetIdKey);
    assetProof = await connectionWrapper.getAssetProof(assetIdKey);
    treeAddress = new PublicKey(asset.compression.tree)
    treeAccount = await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connectionWrapper,
        treeAddress
    );
    treeAuthority = treeAccount.getAuthority();
    canopyDepth = treeAccount.getCanopyDepth();

    proofPath = assetProof.proof
        .map((node: string) => ({
          pubkey: new PublicKey(node),
          isSigner: false,
          isWritable: false,
        }))
        .slice(0, assetProof.proof.length - (!!canopyDepth ? canopyDepth : 0))

    // get root, data hash, creator hash, nonce, and index from asset and asset proof
    root = [...new PublicKey(assetProof.root.trim()).toBytes()];
    dataHash = [
      ...new PublicKey(asset.compression.data_hash.trim()).toBytes(),
    ];
    creatorHash = [
      ...new PublicKey(asset.compression.creator_hash.trim()).toBytes(),
    ];
    nonce = asset.compression.leaf_id;
    index = asset.compression.leaf_id;

    // list the compressed nft
    tx = await yardsaleProgram.methods.listCompressedNft(
            assetIdKey,
            root,
            dataHash,
            creatorHash,
            new anchor.BN(nonce),
            index,
            price
        )
        .accounts({
          domain: domainPda,
          keychain: userKeychainPda,
          listing: listingPda,
          currency: NATIVE_MINT,
          proceedsToken: null,
          proceeds: payer,
          treeAuthority,
          leafOwner: payer,
          merkleTree: treeAddress,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(proofPath)
        .transaction();

    txid = await provider.sendAndConfirm(tx);

    console.log('confirming list compressed NFT tx: ', txid);

    await connectionWrapper.confirmTransaction(txid, "finalized");

    console.log(`relisted compressed nft: ${assetId} for ${price} sol: ${txid}, owner listing pda: ${listingPda.toBase58()}`);
  });

  it("purchase cnft", async () => {

    // transfer some sol to the buyer
    let transferAmount =
        anchor.web3.LAMPORTS_PER_SOL * 0.0001 + (await connectionWrapper.getMinimumBalanceForRentExemption(0));

    let tx = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: buyer.publicKey,
          lamports: transferAmount,
        })
    );
    let txid = await provider.sendAndConfirm(tx, [], {commitment: "finalized"});

    // now get the tree info for the purchase
    let assetIdKey = new PublicKey(assetId);

    // some of this stuff is redundant from the previous test, but demoing how to do it
    let asset = await connectionWrapper.getAsset(assetIdKey);
    console.log('fetched asset: ', asset);
    let assetProof = await connectionWrapper.getAssetProof(assetIdKey);
    let treeAddress = new PublicKey(asset.compression.tree)
    let treeAccount = await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connectionWrapper,
        treeAddress
    );
    const treeAuthority = treeAccount.getAuthority();

    const canopyDepth = treeAccount.getCanopyDepth();

    // get "proof path" from asset proof, these are the accounts that need to be passed to the program as remaining accounts
    // may also be empty if tree is small enough, and canopy depth is large enough
    const proofPath: AccountMeta[] = assetProof.proof
        .map((node: string) => ({
          pubkey: new PublicKey(node),
          isSigner: false,
          isWritable: false,
        }))
        .slice(0, assetProof.proof.length - (!!canopyDepth ? canopyDepth : 0))

    console.log(`canopy depth: ${canopyDepth}, asset proof.proof.length: ${assetProof.proof.length}. proof path length: ${proofPath.length}`);

    // get root, data hash, creator hash, nonce, and index from asset and asset proof
    const root = [...new PublicKey(assetProof.root.trim()).toBytes()];
    const dataHash = [
      ...new PublicKey(asset.compression.data_hash.trim()).toBytes(),
    ];
    const creatorHash = [
      ...new PublicKey(asset.compression.creator_hash.trim()).toBytes(),
    ];
    const nonce = asset.compression.leaf_id;
    const index = asset.compression.leaf_id;

    let [listingPda] = findListingPda(assetIdKey, stacheid, domain, yardsaleProgram.programId);

    // now buyer can make the purchase
    tx = await yardsaleProgram.methods.purchaseCnft(root, dataHash, creatorHash, new anchor.BN(nonce), index).accounts( {
          listing: listingPda,
        treasury,
        currency: NATIVE_MINT,
        proceedsToken: null,
        proceeds: payer,
        buyerCurrencyToken: null,
        tokenProgram: TOKEN_PROGRAM_ID,
        treeAuthority,
          merkleTree: treeAddress,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        newLeafOwner: buyer.publicKey,
      })
        .remainingAccounts(proofPath)
        .transaction();

    txid = await provider.sendAndConfirm(tx, [buyer]);

    console.log('confirming list compressed NFT tx: ', txid);

    await connectionWrapper.confirmTransaction(txid, "finalized");

    console.log(`purchased compressed nft: ${assetId}: ${txid}, owner listing pda: ${listingPda.toBase58()}`);

    // check listing pda is gone
    let listing = await yardsaleProgram.account.listing.fetchNullable(listingPda);
    expect(listing).to.be.null;

    // check that the nft is in the buyer's account
    let rpcResp = await connectionWrapper
        .getAssetsByOwner({
          ownerAddress: buyer.publicKey.toBase58(),
        });

    console.log('buyer pda assets: ', rpcResp);
    expect(rpcResp.total).to.equal(1);
    expect(rpcResp.items.length).to.equal(1);
    expect(rpcResp.items[0].id === assetId, "buyer should have the cNFT");

  });




});



