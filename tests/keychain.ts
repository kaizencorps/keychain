import * as anchor from "@project-serum/anchor";
import {AnchorProvider, Program, Wallet} from "@project-serum/anchor";
import { Keychain } from "../target/types/keychain";
import * as assert from "assert";
import {Keypair, sendAndConfirmTransaction} from "@solana/web3.js";
const { SystemProgram } = anchor.web3;

const domain = 'domination';
const playername = 'silostack';
const KEYCHAIN = 'keychain';

describe("keychain", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.Keychain as Program<Keychain>;
  // const program = anchor.workspace.Keychain;


    // the keychain created by the admin (on behalf of the player)
    const adminPlayername = 'admin-player';

    // our domain account
    const [domainPda, domainPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
            Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
        ],
        program.programId
    );

    // our keychain accounts
    const [playerKeychainPda, playerKeychainPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from(anchor.utils.bytes.utf8.encode(playername)),
            Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
            Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
        ],
        program.programId
    );

    const [adminPlayerKeychainPda, adminPlayerKeychainPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from(anchor.utils.bytes.utf8.encode(adminPlayername)),
            Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
            Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
        ],
        program.programId
    );

    console.log(`domain pda: ${domainPda.toBase58()}`);
    console.log(`player keychain pda: ${playerKeychainPda.toBase58()}`);
    console.log(`admin player keychain pda: ${adminPlayerKeychainPda.toBase58()}`);
    console.log(`keychain program ID: ${program.programId.toBase58()}`);

    // the 2nd key to put on the keychain
    const key2 = anchor.web3.Keypair.generate();

    // wallet 2 provider (simulate separate connection)
    const provider2 = new AnchorProvider(
        provider.connection,
        new Wallet(key2),
        {}
    );
    const program2 = new Program(program.idl, program.programId, provider2);

    it('sets up the test', async () => {
        // airdrop some sol to the 2nd key's wallet
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(key2.publicKey, anchor.web3.LAMPORTS_PER_SOL * 0.5),
            "confirmed"
        );
    });

    it("Creates the domain and keychain", async () => {

      try {
        await program.account.domain.fetch(domainPda);
        assert.fail("domain account shouldn't exist");
      } catch (err) {
        // expected
      }

      // check doesn't exist yet
      let keychain = null;
      try {
          await program.account.keyChain.fetch(playerKeychainPda);
          assert.fail("keychain shouldn't exist");
      } catch (err) {
          // expected
      }

      // another way to check if it exists
      let accountInfo = await provider.connection.getAccountInfo(playerKeychainPda);
      assert.ok(accountInfo == null);

      let txid = await program.rpc.createDomain(domain, {
          accounts: {
              domain: domainPda,
              authority: provider.wallet.publicKey,
              systemProgram: SystemProgram.programId
          }
      });
      console.log(`created domain tx: ${txid}`);

      let domainAcct = await program.account.domain.fetch(domainPda);
        // if stored as byte array
        let domainName  = new TextDecoder("utf-8").decode(new Uint8Array(domainAcct.name));
        // console.log('domain: ', domainAcct);
        console.log('domain: ', domainPda.toBase58());
        console.log('-- name: ', domainAcct.name);
        console.log('-- authority: ', domainAcct.authority.toBase58());
        console.log('-- bump: ', domainAcct.bump);

      const adminPlayerWallet = new Keypair();

      // create keychain (admin since the domain's authority = signer/authority)
        txid = await program.methods.createKeychain(playername).accounts({
            keychain: playerKeychainPda,
            domain: domainPda,
            authority: provider.wallet.publicKey,
            wallet: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
        }).rpc();

      console.log(`created 1st keychain tx: ${txid}`);
      keychain = await program.account.keyChain.fetch(playerKeychainPda);
      console.log('keychain: ', keychain);
      console.log('-- domain: ', keychain.domain.toBase58());
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

      // create another
      const player2 = 'player2';
      const [player2KeychainPda, keychainPdaBump2] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from(anchor.utils.bytes.utf8.encode(player2)),
            Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
            Buffer.from(anchor.utils.bytes.utf8.encode("keychain")),
        ],
        program.programId
      );

      const player2Wallet = new Keypair();
      // player creates his own keychain (and done using a transaction)
      const tx = await program.methods.createKeychain('player2').accounts({
        keychain: player2KeychainPda,
          domain: domainPda,
        // user: provider.wallet.publicKey,
        authority: provider.wallet.publicKey,
          wallet: player2Wallet.publicKey,
        systemProgram: SystemProgram.programId,
      }).transaction();

      tx.feePayer = provider.wallet.publicKey;
      tx.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash
      // const signedTx = await provider.wallet.signTransaction(tx);
      // txid = await provider.connection.sendRawTransaction(signedTx.serialize());
      txid = await provider.sendAndConfirm(tx);

      console.log(`created 2nd keychain tx: ${txid}`);

      // now let's fetch that fuckin thing
      keychain = await program.account.keyChain.fetch(player2KeychainPda);
      console.log('keychain: ', keychain);
        console.log('-- domain: ', keychain.domain.toBase58());
        console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

      accountInfo = await provider.connection.getAccountInfo(playerKeychainPda);
      assert.ok(accountInfo != null);
      console.log("accountinfo: ", accountInfo);

      // try to create another from same playername/app
      try {
          await program.rpc.createKeychain(playername, {
              accounts: {
                  keychain: playerKeychainPda,
                  domain: domainPda,
                  authority: provider.wallet.publicKey,
                  wallet: provider.wallet.publicKey,
                  systemProgram: SystemProgram.programId,
              }
          });
          assert.fail("shouldn't be able to create same keychain again");
      } catch (err) {
          // expected
      }
  });

  it("Adds a key to the keychain and verifies it", async () => {
      await program.rpc.addKey(key2.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              user: provider.wallet.publicKey,
          }
      });

      let keychain = await program.account.keyChain.fetch(playerKeychainPda);

      // not verified yet
      assert.ok(!keychain.keys[1].verified, 'added key should be verified');

      // try to add again and we fail (already there)
      program.rpc.addKey(key2.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              user: provider.wallet.publicKey,
          }
      }).then(() => {
          assert.fail("shoudln't be able to add same key again");
      }).catch(() => {
          // expected
      });

      await program2.rpc.verifyKey({
          accounts: {
              keychain: playerKeychainPda,
              user: key2.publicKey,
          }
      });

      // now the 2nd key is verified
      keychain = await program.account.keyChain.fetch(playerKeychainPda);

      // verified!
      assert.ok(keychain.keys[1].verified, 'added key should be verified');

  });

  it("removes a key from the keychain", async () => {
      // we'll remove the original key (simulate it potentially being a custodial key)
      await program2.rpc.removeKey(provider.wallet.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              user: key2.publicKey,
          }
      });

      let keychain = await program.account.keyChain.fetch(playerKeychainPda);
      assert.ok(keychain.numKeys == 1, 'should only be 1 key on the keychain');
      assert.ok(keychain.keys.length == 1, 'should only be 1 key on the keychain');
  });

  it("closes an empty keychain account", async () => {
     // now wallet2 will remove itself (key2) from the keychain (the only key)
     await program2.rpc.removeKey(key2.publicKey, {
         accounts: {
            keychain: playerKeychainPda,
            user: key2.publicKey,
         }
      });

      // account shouldn't exist now
      program.account.keyChain.fetch(playerKeychainPda).then(() => {
          assert.fail("account shouldn't exist");
      }).catch(() => {
          // expected
      });
  });

});
