import {BlockhashWithExpiryBlockHeight, Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction} from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import {
  createAssociatedTokenAccountInstruction,
  createInitializeMint2Instruction, createMintToCheckedInstruction, getAssociatedTokenAddress,
  getMinimumBalanceForRentExemptMint,
  MINT_SIZE,
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import {AnchorProvider, Program, Provider, web3} from "@project-serum/anchor";
import {Metaplex, WalletAdapter, walletAdapterIdentity} from "@metaplex-foundation/js";
import {TokenStandard} from "@metaplex-foundation/mpl-token-metadata";

export const DOMAIN = 'domination';
export const KEYCHAIN = 'keychain';
export const YARDSALE = 'yardsale';


// bazaar constants
export const SELLER = 'seller';
export const LISTING_DOMAIN = 'listing_domain';
export const LISTING = 'listing';
export const DOMAIN_INDEX = "domain_index";

export const DOMAIN_STATE = 'domain_state';

export const KEYCHAIN_SPACE = 'keychains';
export const KEYCHAIN_STATE_SPACE = 'keychain_states';
export const KEY_SPACE = 'keys';

export const LISTINGS_SPACE = 'listings';

export const PROFILE = 'profile';

// const keychainProgram = anchor.workspace.Keychain as Program<Keychain>;
// const profileProgram = anchor.workspace.Profile as Program<Profile>;
// const yardsaleProgram = anchor.workspace.Profile as Program<Yardsale>;

export async function createTokenMint(connection: Connection, payer: Keypair, authority: PublicKey): Promise<Keypair> {

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
      createInitializeMint2Instruction(mintKey.publicKey, 9, authority, authority, TOKEN_PROGRAM_ID),
  );

  await sendAndConfirmTransaction(connection, transaction, [payer, mintKey]);
  return mintKey;
}

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

export async function createNFT(provider: Provider): Promise<Keypair> {

  const lamports = await getMinimumBalanceForRentExemptMint(provider.connection);
  const mint = anchor.web3.Keypair.generate();
  let ata = await getAssociatedTokenAddress(mint.publicKey, provider.publicKey, false);

  // note: doesn't create the metadata
  const transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: provider.publicKey,
        newAccountPubkey: mint.publicKey,
        space: MINT_SIZE,
        lamports,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(mint.publicKey, 0, provider.publicKey, provider.publicKey, TOKEN_PROGRAM_ID),
      createAssociatedTokenAccountInstruction(provider.publicKey, ata, provider.publicKey, mint.publicKey),
      createMintToCheckedInstruction(mint.publicKey, ata, provider.publicKey, 1, 0),
      // to create the metaplex metadata
      /*
      createCreateMetadataAccountV3Instruction(
          {
            metadata: tokenMetadataPubkey,
            mint: mint.publicKey,
            mintAuthority: feePayer,
            payer: feePayer,
            updateAuthority: feePayer,
          },
          {
            createMetadataAccountArgsV3: {
              data: datav2,
              isMutable: true,
              collectionDetails: null
            },
          }
      ),
      createCreateMasterEditionV3Instruction(
          {
            edition: masterEditionPubkey,
            mint: mint.publicKey,
            updateAuthority: feePayer,
            mintAuthority: feePayer,
            payer: feePayer,
            metadata: tokenMetadataPubkey,
          },
          {
            createMasterEditionArgs: {
              maxSupply: 0,
            },
          }
      )
       */

  );
  await provider.sendAndConfirm(transaction, [mint]);
  return mint;
}

export async function createpNFT(provider: AnchorProvider, ruleSet: PublicKey | null = null): Promise<PublicKey> {

  const metaplex = Metaplex.make(provider.connection).use(walletAdapterIdentity(provider.wallet as WalletAdapter));

  const txBuilder = await metaplex.nfts().builders().create({
    uri: "https://famousfoxes.com/metadata/7777.json",
    name: "pNFT #333",
    sellerFeeBasisPoints: 100,
    symbol: 'pFFOX',
    creators: [{
      address: provider.publicKey,
      share: 100,
    },
    ],
    isMutable: true,
    isCollection: false,
    tokenStandard: TokenStandard.ProgrammableNonFungible,
    ruleSet: ruleSet
  });

  // let txid = await provider.sendAndConfirm(txBuilder.toTransaction(await constructBlockhashWithExpiryBlockHeight(provider.connection)));
  let {signature, confirmResponse} = await metaplex.rpc().sendAndConfirmTransaction(txBuilder);

  const { mintAddress } = txBuilder.getContext();
  console.log(`   pNFT mint Success!🎉`);
  console.log(`   Mint Address: ${mintAddress.toString()}, txid: ${signature}`);

  if (ruleSet) {
    console.log(`   RuleSet: ${ruleSet.toString()} `);
  } else {
    console.log(`   RuleSet: None`);
  }

  console.log(`   Minted NFT: https://explorer.solana.com/address/${mintAddress.toString()}?cluster=devnet`);
  console.log(`   Tx: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
  return mintAddress;
}

async function constructBlockhashWithExpiryBlockHeight(connection: web3.Connection): Promise<BlockhashWithExpiryBlockHeight> {
  return await connection.getLatestBlockhash();
}


export const findDomainPda = (domain: string, keychainprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN))],
      keychainprogid
  );
}

export const findDomainStatePda = (domain: string, keychainprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
          Buffer.from(anchor.utils.bytes.utf8.encode(DOMAIN_STATE)),
          Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
          Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN))],
      keychainprogid
  );
}

// finds the keychain pda for the given playername (for the domination domain)
export const findKeychainPda = (name: string, domain: string, keychainprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode(name)),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN_SPACE)),
        Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
      ],
      keychainprogid,
  );
};

export const findKeychainStatePda = (keychainPda: PublicKey, domain: string, keychainprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        keychainPda.toBuffer(),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN_STATE_SPACE)),
        Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
      ],
      keychainprogid,
  );
};

// find the keychain KEY pda for the given wallet address (for the domination domain)
export const findKeychainKeyPda = (walletAddress: PublicKey, domain: string, keychainprogid: PublicKey): [PublicKey, number] => {
  // const [keychainPda, keychainPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        walletAddress.toBuffer(),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEY_SPACE)),
        Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
        Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
      ],
      keychainprogid,
  );
};

export const findProfilePda = (keychainPda: PublicKey, profileprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        keychainPda.toBuffer(),
        Buffer.from(anchor.utils.bytes.utf8.encode(PROFILE)),
      ],
      profileprogid
  );
}

export const findListingPda = (nftMint: PublicKey, keychainName: string, domain: string, yardsaleprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        nftMint.toBuffer(),
        Buffer.from(anchor.utils.bytes.utf8.encode(LISTINGS_SPACE)),
        Buffer.from(anchor.utils.bytes.utf8.encode(keychainName)),
        Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
        Buffer.from(anchor.utils.bytes.utf8.encode(YARDSALE)),
      ],
      yardsaleprogid,
  );
}

export function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}


/// bazaar utils
export const findListingDomainPda = (domainName: string, domainIndex: number, bazaarprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode(LISTING_DOMAIN)),
        Buffer.from(anchor.utils.bytes.utf8.encode(domainName)),
        Buffer.from(anchor.utils.bytes.utf8.encode(DOMAIN_INDEX)),
        Buffer.from([domainIndex]),
      ],
      bazaarprogid,
  );
}

export const findSellerAccountPda = (keychainPda: PublicKey, bazaarprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode(SELLER)),
        keychainPda.toBuffer(),
      ],
      bazaarprogid,
  );
}


export const findBazaarListingPda = (sellerAccount: PublicKey, sellerAccountListingIndex: number, bazaarprogid: PublicKey): [PublicKey, number] => {
  return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode(LISTING)),
        sellerAccount.toBuffer(),
        new anchor.BN(sellerAccountListingIndex).toArrayLike(Buffer, "le", 4),
        // Buffer.from([sellerAccountListingIndex]),
      ],
      bazaarprogid,
  );
}
