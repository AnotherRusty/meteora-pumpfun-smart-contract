import * as anchor from "@coral-xyz/anchor";
import { BN, Program, web3 } from "@coral-xyz/anchor";
import fs from "fs";

import { Keypair, Connection, PublicKey } from "@solana/web3.js";

import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

import { PumpMeteora } from "../target/types/pump_meteora";
import {
  createConfigTx,
  createBondingCurveTx,
  swapTx,
} from "../lib/scripts";
import { execTx } from "../lib/util";
import {
  TEST_DECIMALS,
  TEST_INIT_BONDING_CURVE,
  TEST_NAME,
  TEST_SYMBOL,
  TEST_TOKEN_SUPPLY,
  TEST_URI,
  TEST_VIRTUAL_RESERVES,
  TEST_INITIAL_VIRTUAL_TOKEN_RESERVES,
  TEST_INITIAL_VIRTUAL_SOL_RESERVES,
  TEST_INITIAL_REAL_TOKEN_RESERVES
} from "../lib/constant";
import { createMarket } from "../lib/create-market";

let solConnection: Connection = null;
let program: Program<PumpMeteora> = null;
let payer: NodeWallet = null;

/**
 * Set cluster, provider, program
 * If rpc != null use rpc, otherwise use cluster param
 * @param cluster - cluster ex. mainnet-beta, devnet ...
 * @param keypair - wallet keypair
 * @param rpc - rpc
 */
export const setClusterConfig = async (
  cluster: web3.Cluster,
  keypair: string,
  rpc?: string
) => {
  if (!rpc) {
    solConnection = new web3.Connection(web3.clusterApiUrl(cluster));
  } else {
    solConnection = new web3.Connection(rpc);
  }

  const walletKeypair = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync(keypair, "utf-8"))),
    { skipValidation: true }
  );
  payer = new NodeWallet(walletKeypair);

  console.log("Wallet Address: ", payer.publicKey.toBase58());

  anchor.setProvider(
    new anchor.AnchorProvider(solConnection, payer, {
      skipPreflight: true,
      commitment: "confirmed",
    })
  );

  // Generate the program client from IDL.
  program = anchor.workspace.PumpMeteora as Program<PumpMeteora>;

  console.log("ProgramId: ", program.programId.toBase58());
};

export const configProject = async () => {
  // Create a dummy config object to pass as argument.
  const newConfig = {
    authority: payer.publicKey,
    pendingAuthority: PublicKey.default,

    teamWallet: payer.publicKey,

    initBondingCurve: TEST_INIT_BONDING_CURVE,
    platformBuyFee: 0.69, // Example fee: 0.69%
    platformSellFee: 0.69, // Example fee: 0.69%
    platformMigrationFee: 0.69, //  Example fee: 0.69%

    curveLimit: new BN(62_000_000_000), //  Example limit: 42 SOL

    lamportAmountConfig: {
      range: { min: new BN(15_000_000_000), max: new BN(20_000_000_000) },
    },
    tokenSupplyConfig: {
      range: { min: new BN(1_000_000_000), max: new BN(1_000_000_000) },
    },
    tokenDecimalsConfig: { range: { min: 6, max: 6 } },

    initial_virtual_token_reserves_config: TEST_INITIAL_VIRTUAL_TOKEN_RESERVES,
    initial_virtual_sol_reserves_config: TEST_INITIAL_VIRTUAL_SOL_RESERVES,
    initial_real_token_reserves_config: TEST_INITIAL_REAL_TOKEN_RESERVES,
  };

  const tx = await createConfigTx(
    payer.publicKey,
    newConfig,
    solConnection,
    program
  );

  await execTx(tx, solConnection, payer);
};

export const createBondingCurve = async () => {
  const tx = await createBondingCurveTx(
    TEST_DECIMALS,
    TEST_TOKEN_SUPPLY,
    TEST_VIRTUAL_RESERVES,

    //  metadata
    TEST_NAME,
    TEST_SYMBOL,
    TEST_URI,

    payer.publicKey,

    solConnection,
    program
  );

  await execTx(tx, solConnection, payer);
};

export const swap = async (
  token: PublicKey,

  amount: number,
  style: number
) => {
  const tx = await swapTx(
    payer.publicKey,
    token,
    amount,
    style,
    solConnection,
    program
  );

  await execTx(tx, solConnection, payer);
};