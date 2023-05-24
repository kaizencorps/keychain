import * as anchor from "@project-serum/anchor";
import {AnchorProvider, Program, Wallet} from "@project-serum/anchor";
import { Keychain } from "../target/types/keychain";
import { Profile } from "../target/types/profile";
import * as assert from "assert";
import {Keypair, PublicKey, sendAndConfirmTransaction, Transaction} from "@solana/web3.js";
import {createAssociatedTokenAccount, createMint, mintToChecked} from "@solana/spl-token";
import {
  createNFTMint,
  findDomainPda,
  findDomainStatePda,
  findKeychainKeyPda,
  findKeychainPda,
  findKeychainStatePda, findProfilePda
} from "./utils";
import {expect} from "chai";
const { SystemProgram } = anchor.web3;

const KEYCHAIN = 'keychain';
const PROFILE = 'profile';
const KEYCHAIN_SPACE = 'keychains';
const KEY_SPACE = 'keys';

function randomName() {
    return Math.random().toString(36).substring(2, 5) + Math.random().toString(36).substring(2, 5);
}

const domain = randomName();
// const domain = 'domain1';

const treasury = anchor.web3.Keypair.generate();
const keychainName = randomName();
const adminPlayername = randomName();

const renameCost = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL * 0.01);


describe("keychain", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env()
    anchor.setProvider(provider);

    // standard program using the provider/account in anchor.toml
    const keychainProgram = anchor.workspace.Keychain as Program<Keychain>;

    // original wallet that sets up the keychain
    const randomPlayerKeypair = anchor.web3.Keypair.generate();

    // original wallet provider
    const randomPlayerProvider = new AnchorProvider(
        provider.connection,
        new Wallet(randomPlayerKeypair),
        {}
    );
    const randomPlayerProgram = new Program(keychainProgram.idl, keychainProgram.programId, randomPlayerProvider);

    // the keychain created by the admin (on behalf of the player)
    const adminPlayername = 'admin-' + randomName();

    // 2nd wallet/key
    const key2 = anchor.web3.Keypair.generate();
    // 3rd wallet/key
    const key3 = anchor.web3.Keypair.generate();

    console.log("program id: ", keychainProgram.programId.toBase58());

    // our domain account
    const [domainPda, domainPdaBump] = findDomainPda(domain, keychainProgram.programId);
    const [domainStatePda, domainStatePdaBump] = findDomainStatePda(domain, keychainProgram.programId);

    // our keychain accounts
      const [playerKeychainPda, playerKeychainPdaBump] = findKeychainPda(keychainName, domain, keychainProgram.programId);
      const [playerKeychainStatePda, playerKeychainStatePdaBump] = findKeychainStatePda(playerKeychainPda, domain, keychainProgram.programId);
  // the "pointer" keychain key account
      const [playerKeychainKeyPda, playerKeychainKeyPdaBump] = findKeychainKeyPda(randomPlayerKeypair.publicKey, domain, keychainProgram.programId);

    const [key2KeyPda, key2KeyPdaBump] = findKeychainKeyPda(key2.publicKey, domain, keychainProgram.programId);
    const [key3KeyPda, key3KeyPdaBump] = findKeychainKeyPda(key3.publicKey, domain, keychainProgram.programId);
    const [adminPlayerKeychainPda, adminPlayerKeychainPdaBump] = findKeychainPda(adminPlayername, domain, keychainProgram.programId);

    console.log(`domain: ${domain}`);
    console.log(`domain pda: ${domainPda.toBase58()}`);
    console.log(`domain state pda: ${domainStatePda.toBase58()}`);
    console.log(`treasury: ${treasury.publicKey.toBase58()}`);
    console.log(`player keychain pda: ${playerKeychainPda.toBase58()}`);
    console.log(`admin player keychain pda: ${adminPlayerKeychainPda.toBase58()}`);
    console.log(`keychain program ID: ${keychainProgram.programId.toBase58()}`);

    // we'll use these later
    let nftMint: Keypair = null;
    let nftAccount: PublicKey = null;

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
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(key3.publicKey, anchor.web3.LAMPORTS_PER_SOL * 0.5),
            "confirmed"
        );

        // create a fake nft
        nftMint = await createNFTMint(provider.connection, key2, key2.publicKey);
        console.log('created nft mint: ', nftMint.publicKey.toBase58());
        nftAccount = await createAssociatedTokenAccount(
            provider.connection, // connection
            key2, // fee payer
            nftMint.publicKey, // mint
            key2.publicKey // owner,
        );
        // mint the nft
        let txhash = await mintToChecked(
            provider.connection, // connection
            key2, // fee payer
            nftMint.publicKey, // mint
            nftAccount, // receiver (sholud be a token account)
            key2, // mint authority
            1, // amount. if your decimals is 8, you mint 10^8 for 1 token.
            0 // decimals
        );
        console.log('minted nft in tx: ', txhash);

    });

    it("Creates the domain and keychain", async () => {

      try {
        await keychainProgram.account.currentDomain.fetch(domainPda);
        assert.fail("domain account shouldn't exist");
      } catch (err) {
        // expected
      }

      // check doesn't exist yet
      let keychain = null;
      try {
          await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
          assert.fail("keychain shouldn't exist");
      } catch (err) {
          // expected
      }

      // another way to check if it exists
      let accountInfo = await provider.connection.getAccountInfo(playerKeychainPda);
      assert.ok(accountInfo == null);

      let txid;

      txid = await keychainProgram.methods.createDomain(domain, renameCost).accounts({
            domain: domainPda,
            domainState: domainStatePda,
            authority: provider.wallet.publicKey,
            systemProgram: SystemProgram.programId,
            treasury: treasury.publicKey
      }).rpc();
      console.log(`created domain tx: ${txid}`);

      let domainAcct = await keychainProgram.account.currentDomain.fetch(domainPda);
      let domainStateAcct = await keychainProgram.account.domainState.fetch(domainStatePda);

        // if stored as byte array
        // console.log('domain: ', domainAcct);
        console.log('domain: ', domainPda.toBase58());
        console.log('-- name: ', domainAcct.name);
        console.log('-- treasury: ', domainAcct.treasury.toBase58());
        console.log('-- authority: ', domainAcct.authority.toBase58());
        console.log('-- key cost: ', domainAcct.keyCost.toNumber() / anchor.web3.LAMPORTS_PER_SOL);
        console.log('-- bump: ', domainAcct.bump);

        console.log('domain state: ', domainStatePda.toBase58());
        console.log('-- version: ', domainStateAcct.version);
        console.log('-- domain: ', domainStateAcct.domain.toBase58());

        console.log(`player keychain key pda: ${playerKeychainKeyPda.toBase58()}`);

        // create keychain (this is an admin since domain's authority = signer/authority)
        // admins can create keychains for anybody, but otherwise the authority and wallet (initial key) need to match
        txid = await randomPlayerProgram.methods.createKeychain(keychainName).accounts({
            keychain: playerKeychainPda,
            keychainState: playerKeychainStatePda,
            keychainKey: playerKeychainKeyPda,
            domain: domainPda,
            authority: randomPlayerKeypair.publicKey,
            wallet: randomPlayerKeypair.publicKey,
            systemProgram: SystemProgram.programId,
        }).rpc();

      console.log(`created 1st keychain tx: ${txid}`);

      keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
      let keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
      console.log('keychain: ', keychain);
      console.log('-- version: ', keychainState.version);
      console.log('-- name: ', keychain.name);
      console.log('-- domain: ', keychain.domain);
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

        const keychainKey = await keychainProgram.account.keyChainKey.fetch(playerKeychainKeyPda);
        console.log('\nkeychain key: ', keychainKey);
        console.log('-- keychain: ', keychainKey.keychain.toBase58());
        console.log('-- key: ', keychainKey.key.toBase58());

      // try to create another from same playername/app
      try {
          await keychainProgram.rpc.createKeychain(keychainName, {
              accounts: {
                  keychain: playerKeychainPda,
                  keychainState: playerKeychainStatePda,
                  keychainKey: playerKeychainKeyPda,
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

    /*  removed from testing since this is now a super-admin function (for security)
    it("creates an old keychain and upgrades it ", async () => {

      const keychainName = 'keychainv1';
      const [oldkeychainPda, oldkeychainBump] = findKeychainPda(keychainName, domain, keychainProgram.programId);
      const [oldkeychainStatePda, oldkeychainStateBump] = findKeychainStatePda(oldkeychainPda, domain, keychainProgram.programId);
      const oldkey = anchor.web3.Keypair.generate();
      const [oldkeychainKey, oldkeychainKeyBump] = findKeychainKeyPda(oldkey.publicKey, domain, keychainProgram.programId);

      console.log('creating old keychain: ', oldkeychainPda.toBase58());

      let txid = await keychainProgram.methods.createKeychainV1(keychainName).accounts({
        keychain: oldkeychainPda,
        keychainState: oldkeychainStatePda,
        key: oldkeychainKey,
        domain: domainPda,
        authority: provider.wallet.publicKey,
        wallet: oldkey.publicKey,
        systemProgram: SystemProgram.programId,
      }).rpc();

      console.log('created old keychain tx: ', txid);
      let keychain = await keychainProgram.account.keyChainV1.fetch(oldkeychainPda);
      let keychainState = await keychainProgram.account.keyChainState.fetch(oldkeychainStatePda);
      console.log('old keychain (v1): ', keychain);
      console.log('-- state keychain version: ', keychainState.keychainVersion);
      console.log('-- state keychain: ', keychainState.keychain.toBase58());
      console.log('-- domain: ', keychain.domain);
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());

      // now try upgrading it

      txid = await keychainProgram.methods.upgradeKeychain().accounts({
        authority: provider.wallet.publicKey,
        keychain: oldkeychainPda,
        keychainState: oldkeychainStatePda,
        systemProgram: SystemProgram.programId,
      }).rpc();

      console.log(`called upgrade tx: ${txid}`);

      keychain = await keychainProgram.account.currentKeyChain.fetch(oldkeychainPda);
      keychainState = await keychainProgram.account.keyChainState.fetch(oldkeychainStatePda);
      console.log('old keychain (upgraded): ', keychain);
      console.log('-- state keychain version: ', keychainState.keychainVersion);
      console.log('-- state keychain: ', keychainState.keychain.toBase58());
      console.log('-- name: ', keychain.name);
      console.log('-- domain: ', keychain.domain);
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1: ', keychain.keys[0].key.toBase58());
    });
     */

  it("Adds a key to the keychain and verifies it", async () => {
      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance before adding key: ", treasuryBalance);

      let txid = await randomPlayerProgram.rpc.addKey(key2.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              keychainState: playerKeychainStatePda,
              authority: randomPlayerKeypair.publicKey,
          }
      });
      console.log(`added key ${key2.publicKey.toBase58()} to keychain: ${txid}`);

      treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance after adding key (should still be 0, gets updated on the verify): ", treasuryBalance);

      let keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
      try {
          let key = await keychainProgram.account.keyChainKey.fetch(key2KeyPda);
          assert.fail("key account shouldn't exist");
      } catch (err) {
          // expected - the key account gets created on verify
      }

      // try to add again and we fail (already there)
      try {
        await randomPlayerProgram.methods.addKey(key2.publicKey).accounts({
          keychain: playerKeychainPda,
          keychainState: playerKeychainStatePda,
          authority: randomPlayerKeypair.publicKey,
        }).rpc();
        assert.fail("shouldn't be able to add same key again");
      } catch (err) {
        // expected
      }

      // see if we can remove it with the optional key account not there (this works but will mess up the rest of the test)
    /*
    txid = await randomPlayerProgram.methods.removeKey(key2.publicKey).accounts({
      // don't include the key account
        keychain: playerKeychainPda,
        keychainState: playerKeychainStatePda,
        domain: domainPda,
        key: null,
        authority: randomPlayerKeypair.publicKey,
        treasury: treasury.publicKey
    }).rpc();
      console.log(">>>>> REMOVED KEY!!!! ", txid);
     */


      // now the key2 account needs to verify

      const provider2 = new AnchorProvider(
          provider.connection,
          new Wallet(key2),
          {}
      );

      txid = await keychainProgram.methods.verifyKey().accounts({
          domain: domainPda,
          keychain: playerKeychainPda,
          keychainState: playerKeychainStatePda,
          keychainKey: key2KeyPda,
          authority: key2.publicKey,
          treasury: treasury.publicKey,
          systemProgram: SystemProgram.programId
      }).signers([key2]).rpc();

      let key = await keychainProgram.account.keyChainKey.fetch(key2KeyPda);
      console.log(`created key account after verifying key: ${key2KeyPda}`);
      console.log('-- key: ', key.key.toBase58());
      console.log('-- keychain: ', key.keychain.toBase58());

      keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
      let keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
      console.log('keychain: ', keychain);
      console.log('keychain state: ', keychainState);
      console.log('-- version: ', keychainState.keychainVersion);
      console.log('-- name: ', keychain.name);
      console.log('-- domain: ', keychain.domain);
      console.log('-- num keys: ', keychain.numKeys);
      console.log('-- keys: ', keychain.keys);
      console.log('-- key 1 address: ', keychain.keys[0].key.toBase58());
      console.log('-- key 1: ', keychain.keys[0].key);
      if (keychain.keys.length > 1) {
        console.log('-- key 2 address: ', keychain.keys[1].key.toBase58());
        console.log('-- key 2: ', keychain.keys[1].key);
      }

      // now the 2nd key is verified
      keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);

      // verified!
      assert.ok(keychain.keys.length == 2, '2nd key added');

  });

      it("Adds ANOTHER key to the keychain verifies it, then approves", async () => {
        let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
        console.log("treasury balance before adding 3rd key: ", treasuryBalance);

        // we'll use key2 to add key3
        let txid = await randomPlayerProgram.methods.addKey(key3.publicKey).accounts({
            keychain: playerKeychainPda,
            keychainState: playerKeychainStatePda,
            authority: key2.publicKey,
        }).signers([key2]).rpc();
        console.log(`added key ${key3.publicKey.toBase58()} to keychain: ${txid}`);

        // now the key3 account needs to verify
        txid = await keychainProgram.methods.verifyKey().accounts({
          domain: domainPda,
          keychain: playerKeychainPda,
          keychainState: playerKeychainStatePda,
          keychainKey: key3KeyPda,
          authority: key3.publicKey,
          treasury: treasury.publicKey,
          systemProgram: SystemProgram.programId
        }).signers([key3]).rpc();

        // check the votes. since key2 voted, value should be 2
        let keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
        console.log('keychain state after verifying key3: ', keychainState);
        assert.ok(keychainState.pendingAction.votes.data == 2, 'votes should be 2 since key 2 voted by adding');

        // since threshold is 2, we'll need to approve this 3rd key with the 1st key

        let key = await keychainProgram.account.keyChainKey.fetch(key3KeyPda);
        console.log(`created key account after verifying key3: ${key3KeyPda}`);

        // keychain state will still have a pending action
        keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
        expect(keychainState.pendingAction).to.exist;
        console.log('keychain state after verifying key2: ', keychainState);

        // so now we vote w/2nd key - which shouldn't change anything since already voted
        await randomPlayerProgram.methods.votePendingAction(true).accounts({
            keychain: playerKeychainPda,
            keychainState: playerKeychainStatePda,
            keychainKey: key3KeyPda,
            authority: key2.publicKey,
        }).signers([key2]).rpc();

        // still exists, same number of votes
        keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
        expect(keychainState.pendingAction).to.exist;

        // so now we vote w/1st key - which should execute the add
        await randomPlayerProgram.methods.votePendingAction(true).accounts({
          keychain: playerKeychainPda,
          keychainState: playerKeychainStatePda,
          keychainKey: key3KeyPda,
          authority: randomPlayerKeypair.publicKey,
        }).rpc();

        keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
        expect(keychainState.pendingAction).to.be.null;

        // now there should be 3 keys
        let keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);

        assert.ok(keychain.keys.length == 3, '3rd key added');
      });

  it("removes a key from the keychain", async () => {
      let keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
      console.log(`numkeys: ${keychain.numKeys}`);
      console.log(`keys length: ${keychain.keys.length}`);
      for (let x = 0; x < keychain.keys.length; x++) {
          console.log(`--key ${x}: ${keychain.keys[x].key.toBase58()}`);
      }

      // we'll remove the original key (simulate it potentially being a custodial key)
      await randomPlayerProgram.rpc.removeKey(randomPlayerKeypair.publicKey, {
          accounts: {
              keychain: playerKeychainPda,
              keychainState: playerKeychainStatePda,
              keychainKey: playerKeychainKeyPda,
              authority: randomPlayerKeypair.publicKey,
          }
      });

      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance: ", treasuryBalance);

      // keyaccount still exists
      let keyAccount = await keychainProgram.account.keyChainKey.fetch(playerKeychainKeyPda);

      // so now we vote w/2nd key - which should execute the removal
      await randomPlayerProgram.methods.votePendingAction(true).accounts({
        keychain: playerKeychainPda,
        keychainState: playerKeychainStatePda,
        keychainKey: playerKeychainKeyPda,
        authority: key2.publicKey,
      }).signers([key2]).rpc();

      let keychainState = await keychainProgram.account.keyChainState.fetch(playerKeychainStatePda);
      expect(keychainState.pendingAction).to.be.null;

      let keychainKeyAccount = await keychainProgram.account.keyChainKey.fetchNullable(playerKeychainKeyPda);
      expect(keychainKeyAccount).to.be.null;

      keychain = await keychainProgram.account.currentKeyChain.fetch(playerKeychainPda);
      console.log(`after numkeys: ${keychain.numKeys}`);
      console.log(`after keys length: ${keychain.keys.length}`);
      for (let x = 0; x < keychain.keys.length; x++) {
          console.log(`--key ${x}: ${keychain.keys[x].key.toBase58()}`);
      }
      assert.ok(keychain.numKeys == 2, 'should only be 2 key on the keychain');
      assert.ok(keychain.keys.length == 2, 'should only be 2 key on the keychain');
  });

  it("closes an empty keychain account", async () => {
      let treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance before removing key (and deleting keychain): ", treasuryBalance);
      let userBalance = await provider.connection.getBalance(key3.publicKey);
      console.log("key3 balance before removing key (and deleting keychain): ", userBalance);

     // now wallet2 will remove itself (key2) from the keychain
      let tx = await keychainProgram.methods.removeKey(key2.publicKey).accounts({
          keychain: playerKeychainPda,
          keychainState: playerKeychainStatePda,
          keychainKey: key2KeyPda,
          authority: key2.publicKey,
      }).transaction();
      let txid = await sendAndConfirmTransaction(provider.connection, tx, [key2]);

      // now we need to approve with the last key (key3)
      await randomPlayerProgram.methods.votePendingAction(true).accounts({
        keychain: playerKeychainPda,
        keychainState: playerKeychainStatePda,
        keychainKey: key2KeyPda,
        authority: key3.publicKey,
      }).signers([key3]).rpc();

      // now key3 can remove itself (and thus close the entire keychain)
      txid = await keychainProgram.methods.removeKey(key3.publicKey).accounts({
        keychain: playerKeychainPda,
        keychainState: playerKeychainStatePda,
        keychainKey: key3KeyPda,
        authority: key3.publicKey,
      }).signers([key3]).rpc();

      console.log(`removed key and closed keychain account: ${txid}`);

      treasuryBalance = await provider.connection.getBalance(treasury.publicKey);
      console.log("treasury balance after removing keychain: ", treasuryBalance);
      userBalance = await provider.connection.getBalance(key3.publicKey);
      console.log("key3 balance after removing last key (and deleting keychain): ", userBalance);

      let keychain = await keychainProgram.account.currentKeyChain.fetchNullable(playerKeychainPda);
      expect(keychain).to.be.null;

      let keychainState = await keychainProgram.account.keyChainState.fetchNullable(playerKeychainStatePda);
      expect(keychainState).to.be.null;

      let key3KeyAccount = await keychainProgram.account.keyChainKey.fetchNullable(key3KeyPda);
      expect(key3KeyAccount).to.be.null;
  });


    it('destroys the domain', async () => {

        // program.state.address()

        /* commenting out - not sure how to get the program data account address - currently can only be done by the deployer
        let tx = await program.methods.closeAccount().accounts({
            account: domainPda,
            authority: provider.wallet.publicKey,
            program: program.programId,
            // note: don't know how to get this guy when the shit gets deployed
            programData: new anchor.web3.PublicKey('GfaotbMSYQqjKYmCRPMwr7bGtbfRWZBqYvUuabL29o2W'),
        }).rpc();

        console.log(`destroyed domain: ${tx}`);

        try {
            await program.account.domain.fetch(domainPda);
            assert.fail("domain account shouldn't exist");
        } catch (err) {
            // expected
        }
         */

    });

}


);
