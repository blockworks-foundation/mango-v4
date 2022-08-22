import { BorshAccountsCoder } from '@project-serum/anchor';
import { coder } from '@project-serum/anchor/dist/cjs/spl/token';
import { Market } from '@project-serum/serum';
import { parsePriceData, PriceData } from '@pythnetwork/client';
import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { MangoClient } from '../client';
import { SERUM3_PROGRAM_ID } from '../constants';
import { Id } from '../ids';
import { toNativeDecimals, toUiDecimals } from '../utils';
import { Bank, MintInfo } from './bank';
import { I80F48, ONE_I80F48 } from './I80F48';
import { PerpMarket } from './perp';
import { Serum3Market } from './serum3';

export class Group {
  static from(
    publicKey: PublicKey,
    obj: {
      creator: PublicKey;
      groupNum: number;
      admin: PublicKey;
      fastListingAdmin: PublicKey;
      insuranceMint: PublicKey;
      insuranceVault: PublicKey;
      testing: number;
      version: number;
    },
  ): Group {
    return new Group(
      publicKey,
      obj.creator,
      obj.groupNum,
      obj.admin,
      obj.fastListingAdmin,
      obj.insuranceMint,
      obj.insuranceVault,
      obj.testing,
      obj.version,
      new Map(), // banksMapByName
      new Map(), // banksMapByMint
      new Map(), // banksMapByTokenIndex
      new Map(), // serum3MarketsMap
      new Map(), // serum3MarketExternalsMap
      new Map(), // perpMarketsMap
      new Map(), // mintInfosMapByTokenIndex
      new Map(), // mintInfosMapByMint
      new Map(), // oraclesMap
      new Map(), // vaultAmountsMap
    );
  }

  constructor(
    public publicKey: PublicKey,
    public creator: PublicKey,
    public groupNum: number,
    public admin: PublicKey,
    public fastListingAdmin: PublicKey,
    public insuranceMint: PublicKey,
    public insuranceVault: PublicKey,
    public testing: number,
    public version: number,
    public banksMapByName: Map<string, Bank[]>,
    public banksMapByMint: Map<string, Bank[]>,
    public banksMapByTokenIndex: Map<number, Bank[]>,
    public serum3MarketsMap: Map<string, Serum3Market>,
    public serum3MarketExternalsMap: Map<string, Market>,
    public perpMarketsMap: Map<string, PerpMarket>,
    public mintInfosMapByTokenIndex: Map<number, MintInfo>,
    public mintInfosMapByMint: Map<string, MintInfo>,
    private oraclesMap: Map<string, PriceData>, // UNUSED
    public vaultAmountsMap: Map<string, number>,
  ) {}

  public findSerum3Market(marketIndex: number): Serum3Market | undefined {
    return Array.from(this.serum3MarketsMap.values()).find(
      (serum3Market) => serum3Market.marketIndex === marketIndex,
    );
  }

  public async reloadAll(client: MangoClient) {
    let ids: Id | undefined = undefined;

    if (client.idsSource === 'api') {
      ids = await Id.fromApi(this.publicKey);
    } else if (client.idsSource === 'static') {
      ids = Id.fromIdsByPk(this.publicKey);
    } else {
      ids = null;
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
      // requires reloadBanks to have finished loading
      this.reloadVaults(client, ids),
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

    this.banksMapByName = new Map();
    this.banksMapByMint = new Map();
    this.banksMapByTokenIndex = new Map();
    for (const bank of banks) {
      const mintId = bank.mint.toString();
      if (this.banksMapByMint.has(mintId)) {
        this.banksMapByMint.get(mintId).push(bank);
        this.banksMapByName.get(bank.name).push(bank);
        this.banksMapByTokenIndex.get(bank.tokenIndex).push(bank);
      } else {
        this.banksMapByMint.set(mintId, [bank]);
        this.banksMapByName.set(bank.name, [bank]);
        this.banksMapByTokenIndex.set(bank.tokenIndex, [bank]);
      }
    }
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

    this.mintInfosMapByTokenIndex = new Map(
      mintInfos.map((mintInfo) => {
        return [mintInfo.tokenIndex, mintInfo];
      }),
    );

    this.mintInfosMapByMint = new Map(
      mintInfos.map((mintInfo) => {
        return [mintInfo.mint.toString(), mintInfo];
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
    const banks = Array.from(this?.banksMapByMint, ([, value]) => value);
    const oracles = banks.map((b) => b[0].oracle);
    const prices =
      await client.program.provider.connection.getMultipleAccountsInfo(oracles);

    const coder = new BorshAccountsCoder(client.program.idl);
    for (const [index, price] of prices.entries()) {
      for (const bank of banks[index]) {
        if (bank.name === 'USDC') {
          bank.price = ONE_I80F48;
          bank.uiPrice = 1;
        } else {
          // TODO: Implement switchboard oracle type
          if (
            !BorshAccountsCoder.accountDiscriminator('stubOracle').compare(
              price.data.slice(0, 8),
            )
          ) {
            const stubOracle = coder.decode('stubOracle', price.data);
            bank.price = new I80F48(stubOracle.price.val);
            bank.uiPrice = this?.toUiPrice(
              bank.price,
              bank.mint,
              this?.insuranceMint,
            );
          } else {
            bank.uiPrice = parsePriceData(price.data).previousPrice;
            bank.price = this?.toNativePrice(
              bank.uiPrice,
              bank.mint,
              this?.insuranceMint,
            );
          }
        }
      }
    }
  }

  public async reloadVaults(client: MangoClient, ids?: Id): Promise<void> {
    const vaultPks = Array.from(this.banksMapByMint.values())
      .flat()
      .map((bank) => bank.vault);
    this.vaultAmountsMap = new Map(
      (
        await client.program.provider.connection.getMultipleAccountsInfo(
          vaultPks,
        )
      ).map((vaultAi, i) => [
        vaultPks[i].toBase58(),
        coder().accounts.decode('token', vaultAi.data).amount.toNumber(),
      ]),
    );
  }

  public getMintDecimals(mintPk: PublicKey) {
    return this.banksMapByMint.get(mintPk.toString())[0].mintDecimals;
  }

  public getFirstBankByMint(mintPk: PublicKey) {
    return this.banksMapByMint.get(mintPk.toString())![0];
  }

  public getFirstBankByTokenIndex(tokenIndex: number) {
    return this.banksMapByTokenIndex.get(tokenIndex)[0];
  }

  /**
   *
   * @param client
   * @param mintPk
   * @returns sum of native balances of vaults for all banks for a token (fetched from vaultAmountsMap cache)
   */
  public async getTokenVaultBalanceByMint(
    client: MangoClient,
    mintPk: PublicKey,
  ): Promise<I80F48> {
    const banks = this.banksMapByMint.get(mintPk.toString());
    let amount = 0;
    for (const bank of banks) {
      amount += this.vaultAmountsMap.get(bank.vault.toBase58());
    }
    return I80F48.fromNumber(amount);
  }

  /**
   *
   * @param client
   * @param mintPk
   * @returns sum of ui balances of vaults for all banks for a token
   */
  public async getTokenVaultBalanceByMintUi(
    client: MangoClient,
    mintPk: PublicKey,
  ): Promise<number> {
    return toUiDecimals(
      await this.getTokenVaultBalanceByMint(client, mintPk),
      this.getMintDecimals(mintPk),
    );
  }

  public consoleLogBanks() {
    for (const mintBanks of this.banksMapByMint.values()) {
      for (const bank of mintBanks) {
        console.log(bank.toString());
      }
    }
  }

  public toUiPrice(
    price: I80F48,
    tokenMintPk: PublicKey,
    quoteMintPk: PublicKey,
  ): number {
    const tokenDecimals = this.getMintDecimals(tokenMintPk);
    const quoteDecimals = this.getMintDecimals(quoteMintPk);
    return price
      .mul(I80F48.fromNumber(Math.pow(10, tokenDecimals - quoteDecimals)))
      .toNumber();
  }

  public toNativePrice(
    uiPrice: number,
    tokenMintPk: PublicKey,
    quoteMintPk: PublicKey,
  ): I80F48 {
    const tokenDecimals = this.getMintDecimals(tokenMintPk);
    const quoteDecimals = this.getMintDecimals(quoteMintPk);
    return I80F48.fromNumber(uiPrice).mul(
      I80F48.fromNumber(Math.pow(10, quoteDecimals - tokenDecimals)),
    );
  }

  public toNativeDecimals(uiAmount: number, mintPk: PublicKey): BN {
    const decimals = this.getMintDecimals(mintPk);
    return toNativeDecimals(uiAmount, decimals);
  }

  toString(): string {
    let res = 'Group\n';
    res = res + ' pk: ' + this.publicKey.toString();

    res =
      res +
      '\n mintInfos:' +
      Array.from(this.mintInfosMapByTokenIndex.entries())
        .map(
          (mintInfoTuple) =>
            '  \n' + mintInfoTuple[0] + ') ' + mintInfoTuple[1].toString(),
        )
        .join(', ');

    const banks = [];
    for (const tokenBanks of this.banksMapByMint.values()) {
      for (const bank of tokenBanks) {
        banks.push(bank);
      }
    }

    res =
      res +
      '\n banks:' +
      Array.from(banks)
        .map((bank) => '  \n' + bank.name + ') ' + bank.toString())
        .join(', ');

    return res;
  }
}
