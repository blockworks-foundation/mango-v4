import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { Bank, MintInfo } from './bank';
import { PerpMarket } from './perp';
import { Serum3Market } from './serum3';

export class Group {
  static from(
    publicKey: PublicKey,
    obj: { admin: PublicKey; groupNum: number },
  ): Group {
    return new Group(
      publicKey,
      obj.admin,
      obj.groupNum,
      new Map(),
      new Map(),
      new Map(),
      new Map(),
    );
  }

  constructor(
    public publicKey: PublicKey,
    public admin: PublicKey,
    public groupNum: number,
    public banksMap: Map<string, Bank>,
    public serum3MarketsMap: Map<string, Serum3Market>,
    public perpMarketsMap: Map<string, PerpMarket>,
    public mintInfosMap: Map<string, MintInfo>,
  ) {}

  public findBank(tokenIndex: number): Bank | undefined {
    return Array.from(this.banksMap.values()).find(
      (bank) => bank.tokenIndex === tokenIndex,
    );
  }

  public async reload(client: MangoClient) {
    await this.reloadBanks(client);
    await this.reloadMintInfos(client);
    await this.reloadSerum3Markets(client);
    await this.reloadPerpMarkets(client);
  }

  public async reloadBanks(client: MangoClient) {
    const banks = await client.getBanksForGroup(this);
    this.banksMap = new Map(banks.map((bank) => [bank.name, bank]));
  }

  public async reloadMintInfos(client: MangoClient) {
    const mintInfos = await client.getMintInfosForGroup(this);
    this.mintInfosMap = new Map(
      mintInfos.map((mintInfo) => {
        // console.log(
        //   Array.from(this.banksMap.values()).find(
        //     (bank) => bank.mint.toBase58() === mintInfo.mint.toBase58(),
        //   ),
        // );
        return [
          Array.from(this.banksMap.values()).find(
            (bank) => bank.mint.toBase58() === mintInfo.mint.toBase58(),
          )?.name!,
          mintInfo,
        ];
      }),
    );

    // console.log(this.banksMap);
    // console.log(this.mintInfosMap);
  }

  public async reloadSerum3Markets(client: MangoClient) {
    const serum3Markets = await client.serum3GetMarket(this);
    this.serum3MarketsMap = new Map(
      serum3Markets.map((serum3Market) => [serum3Market.name, serum3Market]),
    );
  }

  public async reloadPerpMarkets(client: MangoClient) {
    const perpMarkets = await client.perpGetMarket(this);
    this.perpMarketsMap = new Map(
      perpMarkets.map((perpMarket) => [perpMarket.name, perpMarket]),
    );
  }
}
