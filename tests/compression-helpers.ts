import fs from "fs";
import path from "path";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";

// define some default locations
const DEFAULT_DATA_DIR_NAME = "data";
const DEFAULT_PUBLIC_KEY_FILE = "keys.json";
const DEFAULT_DEMO_DATA_FILE = "demo.json";

/*
  Load locally stored PublicKey addresses
*/
export function loadPublicKeysFromFile(
    absPath: string = `${DEFAULT_DATA_DIR_NAME}/${DEFAULT_PUBLIC_KEY_FILE}`,
) {
  try {
    if (!absPath) throw Error("No path provided");
    if (!fs.existsSync(absPath)) throw Error("Keys file (keys.json) does not exist.");

    // load the public keys from the file
    const data = JSON.parse(fs.readFileSync(absPath, { encoding: "utf-8" })) || {};

    // convert all loaded keyed values into valid public keys
    for (const [key, value] of Object.entries(data)) {
      data[key] = new PublicKey(value as string) ?? "";
    }

    return data;
  } catch (err) {
    console.warn("Unable to load local file", err);
    throw err;
  }
}

/*
  Locally save a demo data to the filesystem for later retrieval
*/
export function saveDemoDataToFile(
    name: string,
    newData: any,
    absPath: string = `${DEFAULT_DATA_DIR_NAME}/${DEFAULT_DEMO_DATA_FILE}`,
) {
  try {
    let data: object = {};

    // fetch all the current values, when the storage file exists
    if (fs.existsSync(absPath))
      data = JSON.parse(fs.readFileSync(absPath, { encoding: "utf-8" })) || {};

    data = { ...data, [name]: newData };

    // actually save the data to the file
    fs.writeFileSync(absPath, JSON.stringify(data), {
      encoding: "utf-8",
    });

    return data;
  } catch (err) {
    console.warn("Unable to save to file");
    // console.warn(err);
  }

  // always return an object
  return {};
}

/*
  Locally save a PublicKey addresses to the filesystem for later retrieval
*/
export function savePublicKeyToFile(
    name: string,
    publicKey: PublicKey,
    absPath: string = `${DEFAULT_DATA_DIR_NAME}/${DEFAULT_PUBLIC_KEY_FILE}`,
) {
  try {
    // if (!absPath) throw Error("No path provided");
    // if (!fs.existsSync(absPath)) throw Error("File does not exist.");

    // fetch all the current values
    let data: any = loadPublicKeysFromFile(absPath);

    // convert all loaded keyed values from PublicKeys to strings
    for (const [key, value] of Object.entries(data)) {
      data[key as any] = (value as PublicKey).toBase58();
    }
    data = { ...data, [name]: publicKey.toBase58() };

    // actually save the data to the file
    fs.writeFileSync(absPath, JSON.stringify(data), {
      encoding: "utf-8",
    });

    // reload the keys for sanity
    data = loadPublicKeysFromFile(absPath);

    return data;
  } catch (err) {
    console.warn("Unable to save to file", err);
    throw err;
  }
  // always return an object
  return {};
}

/*
  Load a locally stored JSON keypair file and convert it to a valid Keypair
*/
/*
export function loadKeypairFromFile(absPath: string) {
  try {
    if (!absPath) throw Error("No path provided");
    if (!fs.existsSync(absPath)) throw Error("File does not exist.");

    // load the keypair from the file
    const keyfileBytes = JSON.parse(fs.readFileSync(absPath, { encoding: "utf-8" }));
    // parse the loaded secretKey into a valid keypair
    const keypair = Keypair.fromSecretKey(new Uint8Array(keyfileBytes));
    return keypair;
  } catch (err) {
    // return false;
    throw err;
  }
}
*/

/*
  Save a locally stored JSON keypair file for later importing
*/
/*
export function saveKeypairToFile(
    keypair: Keypair,
    fileName: string,
    dirName: string = DEFAULT_DATA_DIR_NAME,
) {
  fileName = path.join(dirName, `${fileName}.json`);

  // create the `dirName` directory, if it does not exists
  if (!fs.existsSync(`./${dirName}/`)) fs.mkdirSync(`./${dirName}/`);

  // remove the current file, if it already exists
  if (fs.existsSync(fileName)) fs.unlinkSync(fileName);

  // write the `secretKey` value as a string
  fs.writeFileSync(fileName, `[${keypair.secretKey.toString()}]`, {
    encoding: "utf-8",
  });

  return fileName;
}
*/

/*
  Compute the Solana explorer address for the various data
*/
export function explorerURL({
                              address,
                              txSignature,
                              cluster,
                            }: {
  address?: string;
  txSignature?: string;
  cluster?: "devnet" | "testnet" | "mainnet" | "mainnet-beta";
}) {
  let baseUrl: string;
  //
  if (address) baseUrl = `https://explorer.solana.com/address/${address}`;
  else if (txSignature) baseUrl = `https://explorer.solana.com/tx/${txSignature}`;
  else return "[unknown]";

  // auto append the desired search params
  const url = new URL(baseUrl);
  url.searchParams.append("cluster", cluster || "devnet");
  return url.toString() + "\n";
}

/*
  Helper function to extract a transaction signature from a failed transaction's error message
*/
export async function extractSignatureFromFailedTransaction(
    connection: Connection,
    err: any,
    fetchLogs?: boolean,
) {
  if (err?.signature) return err.signature;

  // extract the failed transaction's signature
  const failedSig = new RegExp(/^((.*)?Error: )?(Transaction|Signature) ([A-Z0-9]{32,}) /gim).exec(
      err?.message?.toString(),
  )?.[4];

  // ensure a signature was found
  if (failedSig) {
    // when desired, attempt to fetch the program logs from the cluster
    if (fetchLogs)
      await connection
          .getTransaction(failedSig, {
            maxSupportedTransactionVersion: 0,
          })
          .then(tx => {
            console.log(`\n==== Transaction logs for ${failedSig} ====`);
            console.log(explorerURL({ txSignature: failedSig }), "");
            console.log(tx?.meta?.logMessages ?? "No log messages provided by RPC");
            console.log(`==== END LOGS ====\n`);
          });
    else {
      console.log("\n========================================");
      console.log(explorerURL({ txSignature: failedSig }));
      console.log("========================================\n");
    }
  }

  // always return the failed signature value
  return failedSig;
}

/*
  Standard number formatter
*/
export function numberFormatter(num: number, forceDecimals = false) {
  // set the significant figures
  const minimumFractionDigits = num < 1 || forceDecimals ? 10 : 2;

  // do the formatting
  return new Intl.NumberFormat(undefined, {
    minimumFractionDigits,
  }).format(num);
}

/*
  Display a separator in the console, with our without a message
*/
export function printConsoleSeparator(message?: string) {
  console.log("\n===============================================");
  console.log("===============================================\n");
  if (message) console.log(message);
}

const fakeNftData = [
  {
    name: "cNFT Fox #",
    symbol: "cFOX",
    uri: "https://famousfoxes.com/metadata/###.json",
    rangeLower: 1,
    rangeUpper: 7777
  },
  {
    name: "cNFT Kai #",
    symbol: "cKAI",
    uri: "https://kai1.kaizencorps.com/g1/###.json",
    rangeLower: 1,
    rangeUpper: 5000
  },
  {
    name: "cNFT Coder #",
    symbol: "cSSC",
    uri: "https://shdw-drive.genesysgo.net/8yHTE5Cz3hwcTdghynB2jgLuvKyRgKEz2n5XvSiXQabG/###.json",
    rangeLower: 1,
    rangeUpper: 9999
  },
];

export function getRandomFakeNftMetadata(feePayer: PublicKey) {
  let nft = fakeNftData[Math.floor(Math.random() * fakeNftData.length)];
  let id = Math.floor(Math.random() * (nft.rangeUpper - nft.rangeLower)) + nft.rangeLower;
  let symbol = nft.symbol;
  let name = nft.name + id;
  return {
    name,
    symbol,
    uri: nft.uri.replace("###", id.toString()),
    sellerFeeBasisPoints: 100,
    creators: [
      {
        address: feePayer,
        verified: true,
        share: 100,
      },
    ],
    collection: null,
    uses: null,
  };
}
