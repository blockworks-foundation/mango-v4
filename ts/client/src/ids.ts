import { Cluster, PublicKey } from '@solana/web3.js';
import fetch from 'cross-fetch';
import ids from '../ids.json';
export class Id {
  constructor(
    public cluster: Cluster,
    public name: string,
    public publicKey: string,
    public serum3ProgramId: string,
    public mangoProgramId: string,
    public banks: { name: string; publicKey: string; active: boolean }[],
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
        .filter((perpMarket) => perpMarket.active)
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
        .filter((perpMarket) => perpMarket.active)
        .map((serum3Market) => new PublicKey(serum3Market.publicKey)),
    );
  }

  public getPerpMarkets(): PublicKey[] {
    return Array.from(
      this.perpMarkets
        .filter((perpMarket) => perpMarket.active)
        .map((perpMarket) => new PublicKey(perpMarket.publicKey)),
    );
  }

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
    const groupMetadataApiUrl =
      'https://mango-transaction-log.herokuapp.com/v4/group-metadata';
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
      })),
      groupConfig.serum3Markets.map((s) => ({
        name: s.name,
        publicKey: s.publicKey,
        marketExternal: s.marketExternal,
      })),
      groupConfig.perpMarkets.map((p) => ({
        name: p.name,
        publicKey: p.publicKey,
      })),
    );
  }
}
