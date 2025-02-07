import * as anchor from "@coral-xyz/anchor";
import { BN, Program, web3 } from "@coral-xyz/anchor";
import fs from "fs";

import { Keypair, Connection, PublicKey, SystemProgram, TransactionInstruction, SYSVAR_RENT_PUBKEY, ComputeBudgetProgram, Transaction, TransactionMessage, AddressLookupTableProgram , VersionedTransaction} from "@solana/web3.js";

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
  TEST_INITIAL_REAL_TOKEN_RESERVES,
  SEED_BONDING_CURVE,
  SEED_CONFIG
} from "../lib/constant";
import { createMarket } from "../lib/create-market";
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import { web3JsRpc } from '@metaplex-foundation/umi-rpc-web3js';
import { keypairIdentity, publicKey, transactionBuilder, TransactionBuilder, Umi } from '@metaplex-foundation/umi';
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, NATIVE_MINT, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { fromWeb3JsKeypair, toWeb3JsPublicKey } from '@metaplex-foundation/umi-web3js-adapters';
import AmmImpl, { PROGRAM_ID } from '@mercurial-finance/dynamic-amm-sdk';
import VaultImpl, { getVaultPdas } from '@mercurial-finance/vault-sdk';
import { SEEDS, METAPLEX_PROGRAM } from '@mercurial-finance/dynamic-amm-sdk/dist/cjs/src/amm/constants';
import { createProgram } from "@mercurial-finance/dynamic-amm-sdk/dist/cjs/src/amm/utils";
import { derivePoolAddressWithConfig, getOrCreateATAInstruction, deriveMintMetadata, deriveLockEscrowPda} from './util'


let solConnection: Connection = null;
let program: Program<PumpMeteora> = null;
let payer: NodeWallet = null;
let provider: anchor.Provider = null;
let umi: Umi;

// Address of the deployed program.
let programId;

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

  provider = anchor.getProvider();
  const rpcUrl = rpc ? rpc : web3.clusterApiUrl(cluster);
  umi = createUmi(rpcUrl).use(web3JsRpc(provider.connection));

  // Generate the program client from IDL.
  program = anchor.workspace.PumpMeteora as Program<PumpMeteora>;
  programId = program.programId.toBase58();
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

export const FEE_RECEIVER = publicKey("3bM4hewuZFZgNXvLWwaktXMa8YHgxsnnhaRfzxJV944P");
export const METEORA_CONFIG = publicKey("BdfD7rrTZEWmf8UbEBPVpvM3wUqyrR8swjAy5SNT8gJ2");

export const migrate = async (mint: string) => {
  const { ammProgram, vaultProgram } = createProgram(provider.connection, null);
  const eventAuthority = PublicKey.findProgramAddressSync([Buffer.from("__event_authority")], new PublicKey(PROGRAM_ID))[0];

  // const global_config = PublicKey.findProgramAddressSync([Buffer.from("config")], programId)[0];

  const configPda = PublicKey.findProgramAddressSync(
    [Buffer.from(SEED_CONFIG)],
    program.programId
  )[0];
  const configAccount = await program.account.config.fetch(configPda);

  const tokenAMint = NATIVE_MINT;

  // Needs to be dynamic
  const tokenBMint = new PublicKey(mint);

  // Needs to as defined in smart contract
  const config = toWeb3JsPublicKey(METEORA_CONFIG);
  const feeReceiver = toWeb3JsPublicKey(FEE_RECEIVER);

  const bondingCurve = PublicKey.findProgramAddressSync([Buffer.from(SEED_BONDING_CURVE), tokenBMint.toBytes()], program.programId)[0];

  const poolPubkey = derivePoolAddressWithConfig(tokenAMint, tokenBMint, config, ammProgram.programId);

  const [
      { vaultPda: aVault, tokenVaultPda: aTokenVault, lpMintPda: aLpMintPda },
      { vaultPda: bVault, tokenVaultPda: bTokenVault, lpMintPda: bLpMintPda },
  ] = [getVaultPdas(tokenAMint, vaultProgram.programId), getVaultPdas(tokenBMint, vaultProgram.programId)];

  let aVaultLpMint = aLpMintPda;
  let bVaultLpMint = bLpMintPda;
  let preInstructions: Array<TransactionInstruction> = [];

  // Vault creation Ixs
  const [aVaultAccount, bVaultAccount] = await Promise.all([
      vaultProgram.account.vault.fetchNullable(aVault),
      vaultProgram.account.vault.fetchNullable(bVault),
  ]);

  if (!aVaultAccount) {
      const createVaultAIx = await VaultImpl.createPermissionlessVaultInstruction(provider.connection, payer.publicKey, tokenAMint);
      createVaultAIx && preInstructions.push(createVaultAIx);

  } else {
      aVaultLpMint = aVaultAccount.lpMint; // Old vault doesn't have lp mint pda
  }
  if (!bVaultAccount) {
      const createVaultBIx = await VaultImpl.createPermissionlessVaultInstruction(provider.connection, payer.publicKey, tokenBMint);
      createVaultBIx && preInstructions.push(createVaultBIx);

  } else {
      bVaultLpMint = bVaultAccount.lpMint; // Old vault doesn't have lp mint pda
  }

  const [lpMint] = PublicKey.findProgramAddressSync(
      [Buffer.from(SEEDS.LP_MINT), poolPubkey.toBuffer()],
      ammProgram.programId,
  );
  const [[aVaultLp], [bVaultLp]] = [
      PublicKey.findProgramAddressSync([aVault.toBuffer(), poolPubkey.toBuffer()], ammProgram.programId),
      PublicKey.findProgramAddressSync([bVault.toBuffer(), poolPubkey.toBuffer()], ammProgram.programId),
  ];

  const [[payerTokenB, payerTokenBIx], [payerTokenA, payerTokenAIx]] = await Promise.all([
      getOrCreateATAInstruction(tokenBMint, payer.publicKey, provider.connection ),
      getOrCreateATAInstruction(tokenAMint, payer.publicKey, provider.connection ),
  ]);


  // Create Native Mint SOL ATA for sol escrow
  payerTokenAIx && preInstructions.push(payerTokenAIx);
  payerTokenBIx && preInstructions.push(payerTokenBIx);

  const [feeReceiverTokenAccount, feeReceiverTokenAccountIx] = await getOrCreateATAInstruction(tokenBMint, feeReceiver, provider.connection, payer.publicKey);
  feeReceiverTokenAccountIx && preInstructions.push(feeReceiverTokenAccountIx);


  const bondingCurveTokenB = getAssociatedTokenAddressSync(tokenBMint, bondingCurve, true);

  const [[protocolTokenAFee], [protocolTokenBFee]] = [
      PublicKey.findProgramAddressSync(
          [Buffer.from(SEEDS.FEE), tokenAMint.toBuffer(), poolPubkey.toBuffer()],
          ammProgram.programId,
      ),
      PublicKey.findProgramAddressSync(
          [Buffer.from(SEEDS.FEE), tokenBMint.toBuffer(), poolPubkey.toBuffer()],
          ammProgram.programId,
      ),
  ];

  // LP ata of bonding curve
  const payerPoolLp = getAssociatedTokenAddressSync(lpMint,  payer.publicKey);

  const setComputeUnitLimitIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 20_000_000,
  });
  let latestBlockHash = await provider.connection.getLatestBlockhash(
      provider.connection.commitment,
  );

  if (preInstructions.length) {
      const preInstructionTx = new Transaction({
          feePayer: payer.publicKey,
          ...latestBlockHash,
      }).add(...preInstructions);

      preInstructionTx.sign(payer.payer);
      const preInxSim = await solConnection.simulateTransaction(preInstructionTx)

      const txHash = await provider.sendAndConfirm(preInstructionTx, [], {
          commitment: "finalized",
      });
  }

  const [mintMetadata, _mintMetadataBump] = deriveMintMetadata(lpMint);
  const [tokenBMetadata, _tokenBMetadataBump] = deriveMintMetadata(lpMint);

  // Escrow for claim authority Payer
  const [lockEscrowPK] = deriveLockEscrowPda(poolPubkey,  payer.publicKey, ammProgram.programId);
  const [escrowAta, createEscrowAtaIx] = await getOrCreateATAInstruction(lpMint, lockEscrowPK, solConnection, payer.publicKey);

  const [lockEscrowPK1] = deriveLockEscrowPda(poolPubkey,  feeReceiver, ammProgram.programId);
  const [escrowAta1, createEscrowAtaIx1] = await getOrCreateATAInstruction(lpMint, lockEscrowPK1, solConnection, payer.publicKey);

  console.log("create txLockPool  transaction start");

  const txLockPool = await program.methods
      .lockPool()
      .accounts({
          tokenMint: tokenBMint,
          pool: poolPubkey,
          lpMint,
          aVaultLp,
          bVaultLp,
          tokenBMint,
          aVault,
          bVault,
          aVaultLpMint,
          bVaultLpMint,
          payerPoolLp,
          payer: payer.publicKey,
          feeReceiver,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          lockEscrow: lockEscrowPK,
          lockEscrow1: lockEscrowPK1,
          escrowVault: escrowAta,
          escrowVault1: escrowAta1,
          meteoraProgram: PROGRAM_ID,
          eventAuthority,
      })
      .transaction();

  console.log("create txLockPool  transaction end");

  console.log("create txCreatePool  transaction start");
      
  const txCreatePool = await program.methods
      .createPool()
      .accounts({
          tokenMint: tokenBMint,
          teamWallet: configAccount.teamWallet,
          pool: poolPubkey,
          config,
          lpMint,
          aVaultLp,
          bVaultLp,
          tokenAMint,
          tokenBMint,
          aVault,
          bVault,
          aVaultLpMint,
          bVaultLpMint,
          payerTokenA,
          payerTokenB,
          payerPoolLp,
          protocolTokenAFee,
          protocolTokenBFee,
          payer: payer.publicKey,
          mintMetadata,
          rent: SYSVAR_RENT_PUBKEY,
          metadataProgram: METAPLEX_PROGRAM,
          vaultProgram: vaultProgram.programId,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          meteoraProgram: PROGRAM_ID,
          eventAuthority,
      })
      .transaction();

  console.log("create txCreatePool transaction end");


  /// create meteora pool ///
  const creatTx = new web3.Transaction({
      feePayer: payer.publicKey,
      ...latestBlockHash,
  }).add(setComputeUnitLimitIx).add(txCreatePool)

  const slot = await provider.connection.getSlot()

  const [lookupTableInst, lookupTableAddress] =
      AddressLookupTableProgram.createLookupTable({
          authority: payer.publicKey,
          payer: payer.publicKey,
          recentSlot: slot - 200,
      });

  const addresses = [
      // feeReceiver,
      // feeReceiverTokenAccount,
      poolPubkey,
      config,
      lpMint,
      tokenAMint,
      tokenBMint,
      aVault,
      bVault,
      aTokenVault,
      bTokenVault,
      aVaultLp,
      bVaultLp,
      aVaultLpMint,
      bVaultLpMint,
      payerTokenA,
      payerTokenB,
      payerPoolLp,
      protocolTokenAFee,
      protocolTokenBFee,
      payer.publicKey,
      mintMetadata,
      SYSVAR_RENT_PUBKEY,
      METAPLEX_PROGRAM,
      vaultProgram.programId,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      SystemProgram.programId,
      new PublicKey(PROGRAM_ID),
  ]

  const addAddressesInstruction1 = AddressLookupTableProgram.extendLookupTable({
      payer: payer.publicKey,
      authority: payer.publicKey,
      lookupTable: lookupTableAddress,
      addresses: addresses.slice(0, 30)
  });

  latestBlockHash = await provider.connection.getLatestBlockhash(
      provider.connection.commitment,
  );

  const lutMsg1 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: latestBlockHash.blockhash,
      instructions: [lookupTableInst, addAddressesInstruction1]
  }).compileToV0Message();

  const lutVTx1 = new VersionedTransaction(lutMsg1);
  lutVTx1.sign([payer.payer])

  const lutId1 = await provider.connection.sendTransaction(lutVTx1)
  const lutConfirm1 = await provider.connection.confirmTransaction(lutId1, 'finalized')
  await sleep(2000);
  const lookupTableAccount = await provider.connection.getAddressLookupTable(lookupTableAddress, { commitment: 'finalized' })

  const createTxMsg = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: latestBlockHash.blockhash,
      instructions: creatTx.instructions
  }).compileToV0Message([lookupTableAccount.value]);

  const createVTx = new VersionedTransaction(createTxMsg);
  createVTx.sign([payer.payer])

  const sim = await provider.connection.simulateTransaction(createVTx, { sigVerify: true })

  console.log('migrate sim', sim)
  const id = await provider.connection.sendTransaction(createVTx, { skipPreflight: false })
  console.log('migrate id', id)
  const confirm = await provider.connection.confirmTransaction(id)
  console.log('migrate confirm', confirm)

  /// create meteora pool ///
  const lockTx = new web3.Transaction({
    feePayer: payer.publicKey,
    ...latestBlockHash,
}).add(setComputeUnitLimitIx).add(txLockPool)

  //// lock pool /////
  const lockPoolTxMsg = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: latestBlockHash.blockhash,
      instructions: lockTx.instructions
      // }).compileToV0Message();
  }).compileToV0Message([lookupTableAccount.value]);

  const lockPoolVTx = new VersionedTransaction(lockPoolTxMsg);
  lockPoolVTx.sign([payer.payer])

  const lockPoolSim = await provider.connection.simulateTransaction(lockPoolVTx, { sigVerify: true })
  console.log('lockPoolSim', lockPoolSim)
  const lockPoolId = await provider.connection.sendTransaction(lockPoolVTx, { skipPreflight: true })
  console.log('lockPoolId', lockPoolId)
  const lockPoolConfirm = await provider.connection.confirmTransaction(lockPoolId)
  console.log('lockPoolConfirm', lockPoolConfirm)

  return lockPoolId;
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
} 