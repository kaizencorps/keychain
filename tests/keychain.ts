import * as anchor from "@project-serum/anchor";
import {AnchorProvider, Program, Wallet} from "@project-serum/anchor";
import { Keychain } from "../target/types/keychain";
import * as assert from "assert";
import {Keypair, PublicKey, sendAndConfirmTransaction, Transaction} from "@solana/web3.js";
const { SystemProgram } = anchor.web3;

const KEYCHAIN = 'keychain';

function randomName() {
    return Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
}

const domain = randomName();
const treasury = anchor.web3.Keypair.generate();
const playername = randomName();
const adminPlayername = randomName();

const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);


describe("keychain", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  // standard program using the provider/account in anchor.toml
  const program = anchor.workspace.Keychain as Program<Keychain>;

    // original wallet that sets up the keychain
    const randomPlayerKeypair = anchor.web3.Keypair.generate();

    // original wallet provider
    const randomPlayerProvider = new AnchorProvider(
        provider.connection,
        new Wallet(randomPlayerKeypair),
        {}
    );
    const randomPlayerProgram = new Program(program.idl, program.programId, randomPlayerProvider);

    // the keychain created by the admin (on behalf of the player)
    const adminPlayername = 'admin-' + randomName();

    // 2nd wallet/key
    const key2 = anchor.web3.Keypair.generate();

    console.log("program id: ", program.programId.toBase58());

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

    // the "pointer" keychain key account
    const [playerKeychainKeyPda, playerKeychainKeyPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            randomPlayerKeypair.publicKey.toBuffer(),
            Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
            Buffer.from(anchor.utils.bytes.utf8.encode(KEYCHAIN)),
        ],
        program.programId
    );

    const [key2KeyPda, key2KeyPdaBump] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            key2.publicKey.toBuffer(),
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

    console.log(`domain: ${domain}`);
    console.log(`domain pda: ${domainPda.toBase58()}`);
    console.log(`treasury: ${treasury.publicKey.toBase58()}`);
    console.log(`player keychain pda: ${playerKeychainPda.toBase58()}`);
    console.log(`admin player keychain pda: ${adminPlayerKeychainPda.toBase58()}`);
    console.log(`keychain program ID: ${program.programId.toBase58()}`);

    it('sets up the test', async () => {
        // airdrop some sol to the random key's wallet + the 2nd key we're adding
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(randomPlayerKeypair.publicKey, anchor.web3.LAMPORTS_PER_SOL * 0.5),
            "confirmed"
        );
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

      let txid;

      txid = await program.rpc.createDomain(domain, renameCost, {
          accounts: {
              domain: domainPda,
              authority: provider.wallet.publicKey,
              systemProgram: SystemProgram.programId,
              treasury: treasury.publicKey
          }
      });
      console.log(`created domain tx: ${txid}`);

      let domainAcct = await program.account.domain.fetch(domainPda);
        // if stored as byte array
        let domainName  = new TextDecoder("utf-8").decode(new Uint8Array(domainAcct.name));
        // console.log('domain: ', domainAcct);
        console.log('domain: ', domainPda.toBase58());
        console.log('-- name: ', domainAcct.name);
        console.log('-- treasury: ', domainAcct.treasury.toBase58());
        console.log('-- authority: ', domainAcct.authority.toBase58());
        console.log('-- reaname cost: ', domainAcct.keychainCost.toNumber() / anchor.web3.LAMPORTS_PER_SOL);
        console.log('-- bump: ', domainAcct.bump);

        console.log(`player keychain key pda: ${playerKeychainKeyPda.toBase58()}`);

        // create keychain (this is an admin since domain's authority = signer/authority)
        // admins can create keychains for anybody, but otherwise the authority and wallet (initial key) need to match
        txid = await randomPlayerProgram.methods.createKeychain(playername).accounts({
            keychain: playerKeychainPda,
            key: playerKeychainKeyPda,
            domain: domainPda,
            authority: randomPlayerKeypair.publicKey,
            wallet: randomPlayerKeypair.publicKey,
            systemProgram: SystemProgram.programId,
        }).rpc();

      console.log(`created 1st keychain tx: ${txid}`);

      keychain = await program.account.keyChain.fetch(playerKeychainPda);
      console.log('keychain: ', keychain);
      console.log('-- domain: ', keychain.domain.toBase58());
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

        const keychainKey = await program.account.keyChainKey.fetch(playerKeychainKeyPda);
        console.log('\nkeychain key: ', keychainKey);
        console.log('-- keychain: ', keychainKey.keychain.toBase58());
        console.log('-- key: ', keychainKey.key.toBase58());

      // try to create another from same playername/app
      try {
          await program.rpc.createKeychain(playername, {
              accounts: {
                  keychain: playerKeychainPda,
                  key: playerKeychainKeyPda,
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
      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance before adding key: ", treasuryBalance);

      let txid = await randomPlayerProgram.rpc.addKey(key2.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              domain: domainPda,
              // an existing key
              authority: randomPlayerKeypair.publicKey,
              // key: key2KeyPda,
          }
      });
      console.log(`added key ${key2.publicKey.toBase58()} to keychain: ${txid}`);

      treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance after adding key (should still be 0, gets updated on the verify): ", treasuryBalance);

      let keychain = await program.account.keyChain.fetch(playerKeychainPda);
      try {
          let key = await program.account.keyChainKey.fetch(key2KeyPda);
          assert.fail("key account shouldn't exist");
      } catch (err) {
          // expected - the key account gets created on verify
      }

      // not verified yet
      assert.ok(!keychain.keys[1].verified, 'added key should be verified');

      // try to add again and we fail (already there)
      randomPlayerProgram.rpc.addKey(key2.publicKey, {
          accounts: {
              domain: domainPda,
              keychain: playerKeychainPda,
              // key: key2KeyPda,
              authority: randomPlayerKeypair.publicKey,
          }
      }).then(() => {
          assert.fail("shoudln't be able to add same key again");
      }).catch(() => {
          // expected
      });

      // now the key2 account needs to verify
      let tx = new Transaction();

      // const signedTx = await provider.wallet.signTransaction(tx);
      // txid = await provider.connection.sendRawTransaction(signedTx.serialize());

      const provider2 = new AnchorProvider(
          provider.connection,
          new Wallet(key2),
          {}
      );
      const program2 = new Program(program.idl, program.programId, provider2);
      txid = await program2.methods.verifyKey().accounts({
          keychain: playerKeychainPda,
          key: key2KeyPda,
          domain: domainPda,
          treasury: treasury.publicKey,
          authority: key2.publicKey,
          systemProgram: SystemProgram.programId
      }).rpc();

      let key = await program.account.keyChainKey.fetch(key2KeyPda);
      console.log(`created key account from verifying key: ${key2KeyPda}`);

      /*
      const key2Wallet = new Wallet(key2);
      let ix = await program.methods.verifyKey().accounts({
              keychain: playerKeychainPda,
              authority: key2.publicKey,
      }).instruction();
      tx.add(ix);
      tx.feePayer = provider.wallet.publicKey;
      tx.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash
      tx = await key2Wallet.signTransaction(tx);
      txid = await sendAndConfirmTransaction(provider.connection, tx, [key2]);
       */
      // txid = await provider.connection.sendRawTransaction(tx.serialize());
      console.log(`verified key2: ${txid}`);

      // now the 2nd key is verified
      keychain = await program.account.keyChain.fetch(playerKeychainPda);

      // verified!
      assert.ok(keychain.keys[1].verified, 'added key should be verified');

  });

  it("removes a key from the keychain", async () => {
      let keychain = await program.account.keyChain.fetch(playerKeychainPda);
      console.log(`numkeys: ${keychain.numKeys}`);
      console.log(`keys length: ${keychain.keys.length}`);
      for (let x = 0; x < keychain.keys.length; x++) {
          console.log(`--key ${x}: ${keychain.keys[x].key.toBase58()}`);
      }

      // we'll remove the original key (simulate it potentially being a custodial key)
      await randomPlayerProgram.rpc.removeKey(randomPlayerKeypair.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              key: playerKeychainKeyPda,
              domain: domainPda,
              authority: randomPlayerKeypair.publicKey,
              treasury: treasury.publicKey
          }
      });

      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance: ", treasuryBalance);

      try {
          let keyAccount = await program.account.keyChainKey.fetch(playerKeychainKeyPda);
          assert.fail("key account should no longer exist");
      } catch (err) {
          //expected
      }

      keychain = await program.account.keyChain.fetch(playerKeychainPda);
      console.log(`after numkeys: ${keychain.numKeys}`);
      console.log(`after keys length: ${keychain.keys.length}`);
      for (let x = 0; x < keychain.keys.length; x++) {
          console.log(`--key ${x}: ${keychain.keys[x].key.toBase58()}`);
      }
      assert.ok(keychain.numKeys == 1, 'should only be 1 key on the keychain');
      assert.ok(keychain.keys.length == 1, 'should only be 1 key on the keychain');
  });

  it("closes an empty keychain account", async () => {
      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance before removing key (and deleting keychain): ", treasuryBalance);
      let userBalance = await provider.connection.getBalance(key2.publicKey);
      console.log("user balance before removing key (and deleting keychain): ", userBalance);

     // now wallet2 will remove itself (key2) from the keychain (the only key)
      let tx = await program.methods.removeKey(key2.publicKey).accounts({
          domain: domainPda,
          keychain: playerKeychainPda,
          key: key2KeyPda,
          authority: key2.publicKey,
          treasury: treasury.publicKey
      }).transaction();
      let txid = await sendAndConfirmTransaction(provider.connection, tx, [key2]);
      console.log(`removed key and closed keychain account: ${txid}`);

      treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance after removing key: ", treasuryBalance);
      userBalance = await provider.connection.getBalance(key2.publicKey);
      console.log("user balance after removing key (and deleting keychain): ", userBalance);

      program.account.keyChainKey.fetch(key2KeyPda).then(() => {
          assert.fail("key account shouldn't exist");
      }).catch(() => {
          // expected
      });

      // keychain account shouldn't exist now
      program.account.keyChain.fetch(playerKeychainPda).then(() => {
          assert.fail("keychain account shouldn't exist");
      }).catch(() => {
          // expected
      });
  });

});
