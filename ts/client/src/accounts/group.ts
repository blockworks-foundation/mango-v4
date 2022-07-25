import { Market } from '@project-serum/serum';
import { parsePriceData, PriceData } from '@pythnetwork/client';
import { PublicKey } from '@solana/web3.js';
import { MangoClient } from '../client';
import { SERUM3_PROGRAM_ID } from '../constants';
import { Id } from '../ids';
import { Bank, MintInfo } from './bank';
import { I80F48, ONE_I80F48 } from './I80F48';
import { PerpMarket } from './perp';
import { Serum3Market } from './serum3';
import { BorshAccountsCoder } from '@project-serum/anchor';

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
    public serum3MarketExternalsMap: Map<string, Market>,
    public perpMarketsMap: Map<string, PerpMarket>,
    public mintInfosMap: Map<number, MintInfo>,
    public oraclesMap: Map<string, PriceData>,
  ) {}

  public findBank(tokenIndex: number): Bank | undefined {
    return Array.from(this.banksMap.values()).find(
      (bank) => bank.tokenIndex === tokenIndex,
    );
  }

  public findSerum3Market(marketIndex: number): Serum3Market | undefined {
    return Array.from(this.serum3MarketsMap.values()).find(
      (serum3Market) => serum3Market.marketIndex === marketIndex,
    );
  }

  public async reloadAll(client: MangoClient) {
    let ids: Id | undefined = undefined;

    if (client.groupName) {
      ids = Id.fromIds(client.groupName);
    }

    // console.time('group.reload');
    await Promise.all([
      this.reloadBanks(client, ids),
      this.reloadMintInfos(client, ids),
      this.reloadSerum3Markets(client, ids),
      this.reloadPerpMarkets(client, ids),
    ]);

    await Promise.all([
      // requires reloadBanks to have finished loading
      this.reloadBankPrices(client, ids),
      // requires reloadSerum3Markets to have finished loading
      this.reloadSerum3ExternalMarkets(client, ids),
    ]);
    // console.timeEnd('group.reload');
  }

  public async reloadBanks(client: MangoClient, ids?: Id) {
    let banks: Bank[];

    if (ids) {
      banks = (
        await client.program.account.bank.fetchMultiple(ids.getBanks())
      ).map((account, index) =>
        Bank.from(ids.getBanks()[index], account as any),
      );
    } else {
      banks = await client.getBanksForGroup(this);
    }

    this.banksMap = new Map(banks.map((bank) => [bank.name, bank]));
  }

  public async reloadMintInfos(client: MangoClient, ids?: Id) {
    let mintInfos: MintInfo[];
    if (ids) {
      mintInfos = (
        await client.program.account.mintInfo.fetchMultiple(ids.getMintInfos())
      ).map((account, index) =>
        MintInfo.from(ids.getMintInfos()[index], account as any),
      );
    } else {
      mintInfos = await client.getMintInfosForGroup(this);
    }

    this.mintInfosMap = new Map(
      mintInfos.map((mintInfo) => {
        return [mintInfo.tokenIndex, mintInfo];
      }),
    );
  }

  public async reloadSerum3Markets(client: MangoClient, ids?: Id) {
    let serum3Markets: Serum3Market[];
    if (ids) {
      serum3Markets = (
        await client.program.account.serum3Market.fetchMultiple(
          ids.getSerum3Markets(),
        )
      ).map((account, index) =>
        Serum3Market.from(ids.getSerum3Markets()[index], account as any),
      );
    } else {
      serum3Markets = await client.serum3GetMarkets(this);
    }

    this.serum3MarketsMap = new Map(
      serum3Markets.map((serum3Market) => [serum3Market.name, serum3Market]),
    );
  }

  public async reloadSerum3ExternalMarkets(client: MangoClient, ids?: Id) {
    const externalMarkets = await Promise.all(
      Array.from(this.serum3MarketsMap.values()).map((serum3Market) =>
        Market.load(
          client.program.provider.connection,
          serum3Market.serumMarketExternal,
          { commitment: client.program.provider.connection.commitment },
          SERUM3_PROGRAM_ID[client.cluster],
        ),
      ),
    );

    this.serum3MarketExternalsMap = new Map(
      Array.from(this.serum3MarketsMap.values()).map((serum3Market, index) => [
        serum3Market.name,
        externalMarkets[index],
      ]),
    );
  }

  public async reloadPerpMarkets(client: MangoClient, ids?: Id) {
    let perpMarkets: PerpMarket[];
    if (ids) {
      perpMarkets = (
        await client.program.account.perpMarket.fetchMultiple(
          ids.getPerpMarkets(),
        )
      ).map((account, index) =>
        PerpMarket.from(ids.getPerpMarkets()[index], account as any),
      );
    } else {
      perpMarkets = await client.perpGetMarkets(this);
    }

    this.perpMarketsMap = new Map(
      perpMarkets.map((perpMarket) => [perpMarket.name, perpMarket]),
    );
  }

  public async reloadBankPrices(client: MangoClient, ids?: Id): Promise<void> {
    const banks = Array.from(this?.banksMap, ([, value]) => value);
    const oracles = banks.map((b) => b.oracle);
    const prices =
      await client.program.provider.connection.getMultipleAccountsInfo(oracles);

    const coder = new BorshAccountsCoder(client.program.idl);
    for (const [index, price] of prices.entries()) {
      if (banks[index].name === 'USDC') {
        banks[index].price = ONE_I80F48;
      } else {
        // Try and deserialize to stubOracle - if it fails then assume oracle type is pyth
        // TODO: Implement switchboard oracle type
        try {
          let stubOracle = coder.decode('stubOracle', price.data);
          banks[index].price = new I80F48(stubOracle.price.val);
        } catch (e: any)  {
          console.log(e);
          banks[index].price = I80F48.fromNumber(
            parsePriceData(price.data).previousPrice,
          );
        }
      }
    }
  }

  toString(): string {
    let res = 'Group\n';
    res = res + ' pk: ' + this.publicKey.toString();

    res =
      res +
      '\n mintInfos:' +
      Array.from(this.mintInfosMap.entries())
        .map(
          (mintInfoTuple) =>
            '  \n' + mintInfoTuple[0] + ') ' + mintInfoTuple[1].toString(),
        )
        .join(', ');

    return res;
  }
}
