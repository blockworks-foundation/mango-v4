import { Cluster, PublicKey } from '@solana/web3.js';
import ids from '../ids.json';
import { MANGO_V4_ID } from './constants';

export class Id {
  constructor(
    public banks: Map<String, PublicKey>,
    public stubOracles: Map<String, PublicKey>,
    public mintInfos: Map<String, PublicKey>,
    public serum3Markets: Map<String, PublicKey>,
    public serum3MarketExternals: Map<String, PublicKey>,
    public perpMarkets: Map<String, PublicKey>,
  ) {}

  public getBanks(): PublicKey[] {
    return Array.from(this.banks.values());
  }

  public getStubOracles(): PublicKey[] {
    return Array.from(this.stubOracles.values());
  }

  public getMintInfos(): PublicKey[] {
    return Array.from(this.mintInfos.values());
  }

  public getSerum3Markets(): PublicKey[] {
    return Array.from(this.serum3Markets.values());
  }

  public getPerpMarkets(): PublicKey[] {
    return Array.from(this.perpMarkets.values());
  }

  static fromIds(cluster: Cluster, programId: PublicKey, group: PublicKey): Id {
    let groupConfig =
      ids['devnet'][MANGO_V4_ID['devnet'].toBase58()][group.toString()];
    return new Id(
      new Map(Object.entries(groupConfig['banks'])),
      new Map(Object.entries(groupConfig['stubOracles'])),
      new Map(Object.entries(groupConfig['mintInfos'])),
      new Map(Object.entries(groupConfig['serum3Markets'])),
      new Map(Object.entries(groupConfig['serum3MarketExternals'])),
      new Map(Object.entries(groupConfig['perpMarkets'])),
    );
  }
}
