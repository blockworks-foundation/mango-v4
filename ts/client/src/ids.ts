import { Program } from '@coral-xyz/anchor';
import { Cluster, PublicKey } from '@solana/web3.js';
import fetch from 'cross-fetch';
import ids from '../ids.json';
import { Bank, MintInfo } from './accounts/bank';
import { PerpMarket } from './accounts/perp';
import { MangoV4 } from './mango_v4';
export class Id {
  constructor(
    public cluster: Cluster,
    public name: string,
    public publicKey: string,
    public serum3ProgramId: string,
    public mangoProgramId: string,
    public banks: {
      name: string;
      mint: string;
      tokenIndex: number;
      publicKey: string;
      active: boolean;
      decimals: number;
    }[],
    public stubOracles: { name: string; publicKey: string }[],
    public mintInfos: { name: string; publicKey: string }[],
    public serum3Markets: {
      name: string;
      publicKey: string;
      active: boolean;
      marketExternal: string;
    }[],
    public perpMarkets: { name: string; publicKey: string; active: boolean }[],
  ) {}

  public getBanks(): PublicKey[] {
    return Array.from(
      this.banks
        .filter((bank) => bank.active)
        .map((bank) => new PublicKey(bank.publicKey)),
    );
  }

  public getStubOracles(): PublicKey[] {
    return Array.from(
      this.stubOracles.map((stubOracle) => new PublicKey(stubOracle.publicKey)),
    );
  }

  public getMintInfos(): PublicKey[] {
    return Array.from(
      this.mintInfos.map((mintInfo) => new PublicKey(mintInfo.publicKey)),
    );
  }

  public getSerum3Markets(): PublicKey[] {
    return Array.from(
      this.serum3Markets
        .filter((serum3Market) => serum3Market.active)
        .map((serum3Market) => new PublicKey(serum3Market.publicKey)),
    );
  }

  public getSerum3ExternalMarkets(): PublicKey[] {
    return Array.from(
      this.serum3Markets
        .filter((serum3Market) => serum3Market.active)
        .map((serum3Market) => new PublicKey(serum3Market.marketExternal)),
    );
  }

  public getPerpMarkets(): PublicKey[] {
    return Array.from(
      this.perpMarkets.map((perpMarket) => new PublicKey(perpMarket.publicKey)),
    );
  }

  // DEPRECATED
  static fromIdsByName(name: string): Id {
    const groupConfig = ids.groups.find((id) => id['name'] === name);
    if (!groupConfig) throw new Error(`No group config ${name} found in Ids!`);
    return new Id(
      groupConfig.cluster as Cluster,
      groupConfig.name,
      groupConfig.publicKey,
      groupConfig.serum3ProgramId,
      groupConfig.mangoProgramId,
      groupConfig['banks'],
      groupConfig['stubOracles'],
      groupConfig['mintInfos'],
      groupConfig['serum3Markets'],
      groupConfig['perpMarkets'],
    );
  }

  // DEPRECATED
  static fromIdsByPk(groupPk: PublicKey): Id {
    const groupConfig = ids.groups.find(
      (id) => id['publicKey'] === groupPk.toString(),
    );
    if (!groupConfig)
      throw new Error(`No group config ${groupPk.toString()} found in Ids!`);
    return new Id(
      groupConfig.cluster as Cluster,
      groupConfig.name,
      groupConfig.publicKey,
      groupConfig.serum3ProgramId,
      groupConfig.mangoProgramId,
      groupConfig['banks'],
      groupConfig['stubOracles'],
      groupConfig['mintInfos'],
      groupConfig['serum3Markets'],
      groupConfig['perpMarkets'],
    );
  }

  static async fromApi(groupPk: PublicKey): Promise<Id> {
    const groupMetadataApiUrl = 'https://api.mngo.cloud/data/v4/group-metadata';
    const response = await fetch(groupMetadataApiUrl);
    const jsonData = await response.json();

    const groupConfig = jsonData.groups.find(
      (group) => group.publicKey === groupPk.toString(),
    );

    return new Id(
      groupConfig.cluster as Cluster,
      groupConfig.name,
      groupConfig.publicKey,
      groupConfig.serum3ProgramId,
      groupConfig.mangoProgramId,
      groupConfig.tokens.flatMap((t) =>
        t.banks.map((b) => ({
          name: t.symbol,
          mint: t.mint,
          tokenIndex: t.tokenIndex,
          bankNum: b.bankNum,
          publicKey: b.publicKey,
          active: t.active,
          decimals: t.decimals,
        })),
      ),
      groupConfig.stubOracles.map((s) => ({
        mint: s.mint,
        publicKey: s.publicKey,
      })),
      groupConfig.tokens.map((t) => ({
        name: t.symbol,
        mint: t.mint,
        tokenIndex: t.tokenIndex,
        publicKey: t.mintInfo,
        active: t.active,
      })),
      groupConfig.serum3Markets.map((s) => ({
        name: s.name,
        publicKey: s.publicKey,
        marketExternal: s.serumMarketExternal,
        active: s.active,
      })),
      groupConfig.perpMarkets.map((p) => ({
        name: p.name,
        publicKey: p.publicKey,
        active: p.active,
      })),
    );
  }

  static async fromGetProgramAccounts(groupPk: PublicKey, program: Program<MangoV4>): Promise<Id> {

    const groupAccount = await program.account.group.fetch(groupPk)

    const gg = await program.provider.connection.getProgramAccounts(new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'), {filters: [
      {
        memcmp: {
          bytes: groupPk.toBase58(),
          offset: 8,
        },
      },
    ]}

    )
    // console.log(gg)

    const banks = gg.filter(g => g.account.data.length == 3064).map((tuple) => Bank.from(tuple.pubkey, program.coder.accounts.decode("bank", tuple.account.data)))
    console.log(banks)



    return Id.fromApi(groupPk)

    // // this can be just one GPA call so far, and then use a case switch statement on discriminator
    // const banks = (await program.account.bank.all([
    //   {
    //     memcmp: {
    //       bytes: groupPk.toBase58(),
    //       offset: 8,
    //     },
    //   },
    // ])).map((tuple) => Bank.from(tuple.publicKey, tuple.account));

    // const mintInfos = (await program.account.mintInfo.all([
    //   {
    //     memcmp: {
    //       bytes: groupPk.toBase58(),
    //       offset: 8,
    //     },
    //   },
    // ])).map((tuple) => MintInfo.from(tuple.publicKey, tuple.account));

    // const stubOracles = (await program.account.stubOracle.all([
    //   {
    //     memcmp: {
    //       bytes: groupPk.toBase58(),
    //       offset: 8,
    //     },
    //   },
    // ])).map((tuple) => MintInfo.from(tuple.publicKey, tuple.account));

    // const perpMarkets = (await program.account.perpMarket.all([
    //   {
    //     memcmp: {
    //       bytes: groupPk.toBase58(),
    //       offset: 8,
    //     },
    //   },
    // ])).map((tuple) => PerpMarket.from(tuple.publicKey, tuple.account));



    // const groupConfig = jsonData.groups.find(
    //   (group) => group.publicKey === groupPk.toString(),
    // );

  //   return new Id(
  //     groupConfig.cluster as Cluster,
  //     groupConfig.name,
  //     groupPk,
  //     groupAccount.
  //     mango_V4
  //     groupConfig.tokens.flatMap((t) =>
  //       t.banks.map((b) => ({
  //         name: t.symbol,
  //         mint: t.mint,
  //         tokenIndex: t.tokenIndex,
  //         bankNum: b.bankNum,
  //         publicKey: b.publicKey,
  //         active: t.active,
  //         decimals: t.decimals,
  //       })),
  //     ),
  //     groupConfig.stubOracles.map((s) => ({
  //       mint: s.mint,
  //       publicKey: s.publicKey,
  //     })),
  //     groupConfig.tokens.map((t) => ({
  //       name: t.symbol,
  //       mint: t.mint,
  //       tokenIndex: t.tokenIndex,
  //       publicKey: t.mintInfo,
  //       active: t.active,
  //     })),
  //     groupConfig.serum3Markets.map((s) => ({
  //       name: s.name,
  //       publicKey: s.publicKey,
  //       marketExternal: s.serumMarketExternal,
  //       active: s.active,
  //     })),
  //     groupConfig.perpMarkets.map((p) => ({
  //       name: p.name,
  //       publicKey: p.publicKey,
  //       active: p.active,
  //     })),
  //   );
  }
}
