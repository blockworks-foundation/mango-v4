import { Cluster, PublicKey } from '@solana/web3.js';
import ids from '../ids.json';

export class Id {
  constructor(
    public cluster: Cluster,
    public name: string,
    public publicKey: string,
    public serum3ProgramId: string,
    public mangoProgramId: string,
    public banks: { name: string; publicKey: string }[],
    public stubOracles: { name: string; publicKey: string }[],
    public mintInfos: { name: string; publicKey: string }[],
    public serum3Markets: {
      name: string;
      publicKey: string;
      marketExternal: string;
    }[],
    public perpMarkets: { name: string; publicKey: string }[],
  ) {}

  public getBanks(): PublicKey[] {
    return Array.from(this.banks.map((bank) => new PublicKey(bank.publicKey)));
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
      this.serum3Markets.map(
        (serum3Market) => new PublicKey(serum3Market.publicKey),
      ),
    );
  }

  public getPerpMarkets(): PublicKey[] {
    return Array.from(
      this.perpMarkets.map((perpMarket) => new PublicKey(perpMarket.publicKey)),
    );
  }

  static fromIds(name: string): Id {
    const groupConfig = ids.groups.find((id) => id['name'] === name);
    return new Id(
      groupConfig.cluster as Cluster,
      name,
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
}
