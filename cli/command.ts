import { program } from "commander";
import { PublicKey } from "@solana/web3.js";
import {
  configProject,
  createBondingCurve,
  setClusterConfig,
  swap,
} from "./scripts";

program.version("0.0.1");

programCommand("config").action(async (directory, cmd) => {
  const { env, keypair, rpc } = cmd.opts();

  console.log("Solana Cluster:", env);
  console.log("Keypair Path:", keypair);
  console.log("RPC URL:", rpc);

  await setClusterConfig(env, keypair, rpc);

  await configProject();
});

programCommand("curve").action(async (directory, cmd) => {
  const { env, keypair, rpc } = cmd.opts();

  console.log("Solana Cluster:", env);
  console.log("Keypair Path:", keypair);
  console.log("RPC URL:", rpc);

  await setClusterConfig(env, keypair, rpc);

  await createBondingCurve();
});

programCommand("swap")
  .option("-t, --token <string>", "token address")
  .option("-a, --amount <number>", "swap amount")
  .option("-s, --style <string>", "0: buy token, 1: sell token")
  .action(async (directory, cmd) => {
    const { env, keypair, rpc, token, amount, style } = cmd.opts();

    console.log("Solana Cluster:", env);
    console.log("Keypair Path:", keypair);
    console.log("RPC URL:", rpc);

    await setClusterConfig(env, keypair, rpc);

    if (token === undefined) {
      console.log("Error token address");
      return;
    }

    if (amount === undefined) {
      console.log("Error swap amount");
      return;
    }

    if (style === undefined) {
      console.log("Error swap style");
      return;
    }

    await swap(new PublicKey(token), amount, style);
  });


function programCommand(name: string) {
  return program
    .command(name)
    .option(
      //  mainnet-beta, testnet, devnet
      "-e, --env <string>",
      "Solana cluster env name",
      "devnet"
    )
    .option(
      "-r, --rpc <string>",
      "Solana cluster RPC name",
      "https://devnet.helius-rpc.com/?api-key=facb2b5c-c0d2-44b1-8538-986b895bf122"
    )
    .option(
      "-k, --keypair <string>",
      "Solana wallet Keypair Path",
      "./keys/EgBcC7KVQTh1QeU3qxCFsnwZKYMMQkv6TzgEDkKvSNLv.json"
    );
}

program.parse(process.argv);

/*

  yarn script config
  yarn script curve     //catch token_address
  yarn script swap -t BstvSKkKPjowbd9CycqPP8khVFM7W3sxxvsF2psqkcTq -a 2000000000 -s 0

*/