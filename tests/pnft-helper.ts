import { Metaplex } from "@metaplex-foundation/js";
import {
  AuthorizationData,
  Metadata,
  PROGRAM_ID as TMETA_PROG_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as AUTH_PROG_ID } from '@metaplex-foundation/mpl-token-auth-rules';
import * as anchor from "@project-serum/anchor";
import { Idl } from "@project-serum/anchor";
import { Connection, PublicKey, SystemProgram, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import { fetchNft, findTokenRecordPDA } from "./pnft-utils";
import {ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import { Yardsale } from "../target/types/yardsale";

export class PnftHelper {

  wallet: anchor.Wallet;
  provider!: anchor.Provider;
  program!: anchor.Program<Yardsale>;
  connection: Connection;
  constructor(
      connection: Connection,
      wallet: anchor.Wallet,
      idl?: Idl,
      programId?: PublicKey
  ) {
    this.wallet = wallet;
    this.connection = connection;
    this.setProvider();
    this.setProgram(idl, programId);
  }

  setProvider() {
    this.provider = new anchor.AnchorProvider(
        this.connection,
        this.wallet,
        anchor.AnchorProvider.defaultOptions()
    );
    anchor.setProvider(this.provider);
  }

  setProgram(idl?: Idl, programId?: PublicKey) {
    //instantiating program depends on the environment
    if (idl && programId) {
      //means running in prod
      this.program = new anchor.Program<Yardsale>(
          idl as any,
          programId,
          this.provider
      );
    } else {
      //means running inside test suite
      this.program = anchor.workspace.Yardsale as anchor.Program<Yardsale>;
    }
  }

  async prepPnftAccounts({ nftMetadata,
                           nftMint,
                           sourceAta,
                           destAta,
                           authData = null,
                         }: {
    nftMetadata?: PublicKey;
    nftMint: PublicKey;
    sourceAta: PublicKey;
    destAta: PublicKey;
    authData?: AuthorizationData | null;
  }) {
    let meta;
    let creators: PublicKey[] = [];
    if (nftMetadata) {
      meta = nftMetadata;
    } else {
      const nft = await fetchNft(this.provider.connection, nftMint);
      meta = nft.metadataAddress;
      creators = nft.creators.map((c) => c.address);
    }

    const inflatedMeta = await Metadata.fromAccountAddress(
        this.provider.connection,
        meta
    );
    const ruleSet = inflatedMeta.programmableConfig?.ruleSet;

    const [ownerTokenRecordPda, ownerTokenRecordBump] =
        await findTokenRecordPDA(nftMint, sourceAta);
    const [destTokenRecordPda, destTokenRecordBump] = await findTokenRecordPDA(
        nftMint,
        destAta
    );

    //retrieve edition PDA
    const mplex = new Metaplex(this.provider.connection);
    const nftEditionPda = mplex.nfts().pdas().edition({ mint: nftMint });

    //have to re-serialize due to anchor limitations
    const authDataSerialized = authData
        ? {
          payload: Object.entries(authData.payload.map).map(([k, v]) => {
            return { name: k, payload: v };
          }),
        }
        : null;

    return {
      meta,
      creators,
      ownerTokenRecordBump,
      ownerTokenRecordPda,
      destTokenRecordBump,
      destTokenRecordPda,
      ruleSet,
      nftEditionPda,
      authDataSerialized,
    };
  }

  async buildListPNFT(priceBN, {domain,
                                keychain,
                                item,
                                listing,
                                currency,
                                proceeds,
                                proceedsToken,
                                listingItemToken,
                                seller}: {
    currency: PublicKey;
    // the token account to deposit the proceeds into - necessary if currency is NOT sol (an SPL token)
    proceedsToken: PublicKey | null;
    // only specified if currency is native (SOL)
    proceeds: PublicKey | null;
    domain: PublicKey;
    keychain: PublicKey;
    item: PublicKey;
    listingItemToken: PublicKey;
    seller: PublicKey;
    listing: PublicKey;
  }) {

    const authorityItemToken = getAssociatedTokenAddressSync(item, seller);

    //pnft
    const {
      meta,
      ownerTokenRecordBump,
      ownerTokenRecordPda,
      destTokenRecordBump,
      destTokenRecordPda,
      ruleSet,
      nftEditionPda,
      authDataSerialized,
    } = await this.prepPnftAccounts({
      nftMint: item,
      destAta: listingItemToken,
      authData: null, //currently useless
      sourceAta: authorityItemToken,
    });
    const remainingAccounts = [];
    if (ruleSet) {
      remainingAccounts.push({
        pubkey: ruleSet,
        isSigner: false,
        isWritable: false,
      });
    }

    console.log(`>> authDataSerialized: `, authDataSerialized);
    console.log(`>> itemMetadata: ${meta.toBase58()}`);
    console.log(`>> edition: ${nftEditionPda.toBase58()}`);
    console.log(`>> authorityTokenRecord: ${ownerTokenRecordPda.toBase58()}`);
    console.log(`>> authorityItemToken: ${authorityItemToken.toBase58()}`);
    console.log(`>> listingItemToken: ${listingItemToken.toBase58()}`);
    console.log(`>> nftMint: ${item.toBase58()}`);

    const builder = this.program.methods
        .listPnft(priceBN, authDataSerialized, !!ruleSet)
        .accounts({
          domain,
          keychain,
          item: item,
          authorityItemToken,
          listing,
          listingItemToken,
          currency,
          proceedsToken,
          proceeds,
          seller: seller,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          itemMetadata: meta,
          edition: nftEditionPda,
          authorityTokenRecord: ownerTokenRecordPda,
          listingTokenRecord: destTokenRecordPda,
          tokenMetadataProgram: TMETA_PROG_ID,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          authorizationRulesProgram: AUTH_PROG_ID,
        });

        // .remainingAccounts(remainingAccounts);

    return builder
  }

  async buildPurchasePNFT({item,
                           listing,
                           currency,
                           proceeds,
                           proceedsToken,
                           listingItemToken,
                           buyer,
                           buyerCurrencyToken,
                           treasury,
                           ruleset
                          }: {
    currency: PublicKey;
    // the token account to deposit the proceeds into - necessary if currency is NOT sol (an SPL token)
    proceedsToken: PublicKey | null;
    // only specified if currency is native (SOL)
    proceeds: PublicKey | null;
    item: PublicKey;
    listingItemToken: PublicKey;
    buyer: PublicKey;
    buyerCurrencyToken: PublicKey | null;
    listing: PublicKey;
    treasury: PublicKey;
    ruleset: PublicKey;
  }) {

    const buyerItemToken = getAssociatedTokenAddressSync(item, buyer);

    //pnft
    const {
      meta,
      ownerTokenRecordBump,
      ownerTokenRecordPda,
      destTokenRecordBump,
      destTokenRecordPda,
      ruleSet,
      nftEditionPda,
      authDataSerialized,
    } = await this.prepPnftAccounts({
      nftMint: item,
      destAta: buyerItemToken,
      authData: null, //currently useless
      sourceAta: listingItemToken,
    });
    const remainingAccounts = [];
    if (ruleSet) {
      remainingAccounts.push({
        pubkey: ruleSet,
        isSigner: false,
        isWritable: false,
      });
    }

    console.log(`>> authDataSerialized: `, authDataSerialized);
    console.log(`>> itemMetadata: ${meta.toBase58()}`);
    console.log(`>> edition: ${nftEditionPda.toBase58()}`);
    console.log(`>> buyerTokenRecord: ${ownerTokenRecordPda.toBase58()}`);
    console.log(`>> buyerItemToken: ${buyerItemToken.toBase58()}`);
    console.log(`>> nftMint: ${item.toBase58()}`);

    if (ruleSet) {
      console.log(`>> ruleset: ${ruleSet.toBase58()}`);
    } else {
      console.log(`>> no ruleset`);
    }

    const builder = this.program.methods
        .purchasePnft()
        .accounts({
          listing,
          item,
          itemMetadata: meta,
          edition: nftEditionPda,
          buyerTokenRecord: destTokenRecordPda,
          listingTokenRecord: ownerTokenRecordPda,
          listingItemToken,
          buyerItemToken,
          currency,
          proceedsToken,
          proceeds,
          buyer,
          buyerCurrencyToken,
          treasury,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          authorizationRulesProgram: AUTH_PROG_ID,
          tokenMetadataProgram: TMETA_PROG_ID,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          ruleset
        });

    /*
    if (remainingAccounts.length > 0) {
      console.log("!! adding remaining accounts !!");
      builder.remainingAccounts(remainingAccounts);
    }
     */

    return builder;
  }

  async buildDelistPNFT({keychain,
                        item,
                        listing,
                        listingItemToken,
                        seller,
                        ruleset
                        }: {
      keychain: PublicKey;
      item: PublicKey;
      listingItemToken: PublicKey;
      seller: PublicKey;
      listing: PublicKey;
      ruleset: PublicKey;
    }) {

    const sellerItemToken = getAssociatedTokenAddressSync(item, seller);

    //pnft
    const {
      meta,
      ownerTokenRecordBump,
      ownerTokenRecordPda,
      destTokenRecordBump,
      destTokenRecordPda,
      ruleSet,
      nftEditionPda,
      authDataSerialized,
    } = await this.prepPnftAccounts({
      nftMint: item,
      destAta: sellerItemToken,
      authData: null, //currently useless
      sourceAta: listingItemToken,
    });
    const remainingAccounts = [];
    if (ruleSet) {
      remainingAccounts.push({
        pubkey: ruleSet,
        isSigner: false,
        isWritable: false,
      });
    }

    console.log(`>> authDataSerialized: `, authDataSerialized);
    console.log(`>> itemMetadata: ${meta.toBase58()}`);
    console.log(`>> edition: ${nftEditionPda.toBase58()}`);
    console.log(`>> authorityTokenRecord: ${ownerTokenRecordPda.toBase58()}`);
    console.log(`>> authorityItemToken: ${sellerItemToken.toBase58()}`);
    console.log(`>> listingItemToken: ${listingItemToken.toBase58()}`);
    console.log(`>> nftMint: ${item.toBase58()}`);

    const builder = this.program.methods
        .delistPnft()
        .accounts({
          listing,
          keychain,
          item,
          sellerItemToken,
          listingItemToken,
          seller,
          itemMetadata: meta,
          edition: nftEditionPda,
          sellerTokenRecord: ownerTokenRecordPda,
          listingTokenRecord: destTokenRecordPda,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          authorizationRulesProgram: AUTH_PROG_ID,
          tokenMetadataProgram: TMETA_PROG_ID,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          ruleset
        });

    // .remainingAccounts(remainingAccounts);

    return builder
  }




}
