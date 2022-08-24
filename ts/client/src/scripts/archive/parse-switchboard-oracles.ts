import { AnchorProvider } from '@project-serum/anchor';
import { Connection, PublicKey } from '@solana/web3.js';
import { parseSwitchboardOracle } from '../../accounts/oracle';

async function main() {
  const options = AnchorProvider.defaultOptions();

  async function foo(obj) {
    let connection = new Connection(obj.net, options);
    let ai = await connection.getAccountInfo(new PublicKey(obj.pk));
    console.log(
      `${obj.name} price ${(await parseSwitchboardOracle(ai!)).toNumber()}`,
    );
  }

  for (const oracle of [
    {
      name: 'devnet mngo v1',
      pk: '8k7F9Xb36oFJsjpCKpsXvg4cgBRoZtwNTc3EzG5Ttd2o',
      net: 'https://mango.devnet.rpcpool.com',
    },
    {
      name: 'devnet sol v2',
      pk: 'GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR',
      net: 'https://mango.devnet.rpcpool.com',
    },
    {
      name: 'mainnet btc v2',
      pk: '3HtmwdXJPAdMZ73fTGeCFgbDQZGLZWpmsm3JAB5quGJN',
      net: 'http://api.mainnet-beta.solana.com/',
    },
    {
      name: 'mainnet sol v2',
      pk: 'GvDMxPzN1sCj7L26YDK2HnMRXEQmQ2aemov8YBtPS7vR',
      net: 'http://api.mainnet-beta.solana.com/',
    },
  ]) {
    await foo(oracle);
  }

  process.exit();
}

main();
