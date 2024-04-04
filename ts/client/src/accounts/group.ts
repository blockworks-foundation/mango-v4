import { BorshAccountsCoder } from '@coral-xyz/anchor';
import { Market, Orderbook } from '@project-serum/serum';
import { parsePriceData } from '@pythnetwork/client';
import { TOKEN_PROGRAM_ID, unpackAccount } from '@solana/spl-token';
import {
  AccountInfo,
  AddressLookupTableAccount,
  PublicKey,
} from '@solana/web3.js';
import BN from 'bn.js';
import merge from 'lodash/merge';
import { MangoClient } from '../client';
import { OPENBOOK_PROGRAM_ID } from '../constants';
import { Id } from '../ids';
import { I80F48 } from '../numbers/I80F48';
import { PriceImpact, computePriceImpactOnJup } from '../risk';
import {
  buildFetch,
  deepClone,
  toNative,
  toNativeI80F48,
  toUiDecimals,
} from '../utils';
import { Bank, MintInfo, TokenIndex } from './bank';
import {
  OracleProvider,
  isPythOracle,
  isSwitchboardOracle,
  parseSwitchboardOracle,
} from './oracle';
import { BookSide, PerpMarket, PerpMarketIndex } from './perp';
import { MarketIndex, Serum3Market } from './serum3';

export class Group {
  static from(
    publicKey: PublicKey,
    obj: {
      creator: PublicKey;
      groupNum: number;
      admin: PublicKey;
      fastListingAdmin: PublicKey;
      mngoTokenIndex: number;
      insuranceMint: PublicKey;
      insuranceVault: PublicKey;
      testing: number;
      version: number;
      buybackFees: number;
      buybackFeesMngoBonusFactor: number;
      addressLookupTables: PublicKey[];
      securityAdmin: PublicKey;
      depositLimitQuote: BN;
      ixGate: BN;
      buybackFeesSwapMangoAccount: PublicKey;
      buybackFeesExpiryInterval: BN;
      fastListingIntervalStart: BN;
      fastListingsInInterval: number;
      allowedFastListingsPerInterval: number;
      collateralFeeInterval: BN;
    },
  ): Group {
    return new Group(
      publicKey,
      obj.creator,
      obj.groupNum,
      obj.admin,
      obj.fastListingAdmin,
      obj.mngoTokenIndex as TokenIndex,
      obj.insuranceMint,
      obj.insuranceVault,
      obj.testing,
      obj.version,
      obj.buybackFees == 1,
      obj.buybackFeesMngoBonusFactor,
      obj.addressLookupTables,
      obj.securityAdmin,
      obj.depositLimitQuote,
      obj.ixGate,
      obj.buybackFeesSwapMangoAccount,
      obj.buybackFeesExpiryInterval,
      obj.fastListingIntervalStart,
      obj.fastListingsInInterval,
      obj.allowedFastListingsPerInterval,
      obj.collateralFeeInterval,
      [], // addressLookupTablesList
      new Map(), // banksMapByName
      new Map(), // banksMapByMint
      new Map(), // banksMapByTokenIndex
      new Map(), // serum3MarketsMapByExternal
      new Map(), // serum3MarketsMapByMarketIndex
      new Map(), // serum3MarketExternalsMap
      new Map(), // perpMarketsMapByOracle
      new Map(), // perpMarketsMapByMarketIndex
      new Map(), // perpMarketsMapByName
      new Map(), // mintInfosMapByTokenIndex
      new Map(), // mintInfosMapByMint
      new Map(), // vaultAmountsMap
      [],
    );
  }

  constructor(
    public publicKey: PublicKey,
    public creator: PublicKey,
    public groupNum: number,
    public admin: PublicKey,
    public fastListingAdmin: PublicKey,
    public mngoTokenIndex: TokenIndex,
    public insuranceMint: PublicKey,
    public insuranceVault: PublicKey,
    public testing: number,
    public version: number,
    public buybackFees: boolean,
    public buybackFeesMngoBonusFactor: number,
    public addressLookupTables: PublicKey[],
    public securityAdmin: PublicKey,
    public depositLimitQuote,
    public ixGate: BN,
    public buybackFeesSwapMangoAccount: PublicKey,
    public buybackFeesExpiryInterval: BN,
    public fastListingIntervalStart: BN,
    public fastListingsInInterval: number,
    public allowedFastListingsPerInterval: number,
    public collateralFeeInterval: BN,
    public addressLookupTablesList: AddressLookupTableAccount[],
    public banksMapByName: Map<string, Bank[]>,
    public banksMapByMint: Map<string, Bank[]>,
    public banksMapByTokenIndex: Map<TokenIndex, Bank[]>,
    public serum3MarketsMapByExternal: Map<string, Serum3Market>,
    public serum3MarketsMapByMarketIndex: Map<MarketIndex, Serum3Market>,
    public serum3ExternalMarketsMap: Map<string, Market>,
    public perpMarketsMapByOracle: Map<string, PerpMarket>,
    public perpMarketsMapByMarketIndex: Map<PerpMarketIndex, PerpMarket>,
    public perpMarketsMapByName: Map<string, PerpMarket>,
    public mintInfosMapByTokenIndex: Map<TokenIndex, MintInfo>,
    public mintInfosMapByMint: Map<string, MintInfo>,
    public vaultAmountsMap: Map<string, BN>,
    public pis: PriceImpact[],
  ) {}

  public async reloadAll(client: MangoClient): Promise<void> {
    const ids: Id | undefined = await client.getIds(this.publicKey);

    // console.time('group.reload');
    await Promise.all([
      this.reloadPriceImpactData(),
      this.reloadAlts(client),
      this.reloadBanks(client, ids).then(() =>
        Promise.all([
          this.reloadBankOraclePrices(client),
          this.reloadVaults(client),
          this.reloadPerpMarkets(client, ids).then(() =>
            this.reloadPerpMarketOraclePrices(client),
          ),
        ]),
      ),
      this.reloadMintInfos(client, ids),
      this.reloadSerum3Markets(client, ids).then(() =>
        this.reloadSerum3ExternalMarkets(client, ids),
      ),
    ]);
    // console.timeEnd('group.reload');
  }

  public async reloadPriceImpactData(): Promise<void> {
    try {
      this.pis = await (
        await (
          await buildFetch()
        )(
          `https://api.mngo.cloud/data/v4/risk/listed-tokens-one-week-price-impacts`,
          {
            mode: 'cors',
            headers: {
              'Content-Type': 'application/json',
              'Access-Control-Allow-Origin': '*',
            },
          },
        )
      ).json();
    } catch (error) {
      console.log(`Error while loading price impact: ${error}`);
    }
  }

  public async reloadAlts(client: MangoClient): Promise<void> {
    const alts = await Promise.all(
      this.addressLookupTables
        .filter((alt) => !alt.equals(PublicKey.default))
        .map((alt) =>
          client.program.provider.connection.getAddressLookupTable(alt),
        ),
    );
    this.addressLookupTablesList = alts.map((res, i) => {
      if (!res || !res.value) {
        throw new Error(`Undefined ALT ${this.addressLookupTables[i]}!`);
      }
      return res.value;
    });
  }

  public async reloadBanks(client: MangoClient, ids?: Id): Promise<void> {
    let banks: Bank[];

    if (ids && ids.getBanks().length) {
      banks = (
        await client.program.account.bank.fetchMultiple(ids.getBanks())
      ).map((account, index) =>
        Bank.from(ids.getBanks()[index], account as any),
      );
    } else {
      banks = await client.getBanksForGroup(this);
    }

    const oldbanksMapByTokenIndex = deepClone(this.banksMapByTokenIndex);

    this.banksMapByName = new Map();
    this.banksMapByMint = new Map();
    this.banksMapByTokenIndex = new Map();
    for (const bank of banks) {
      // ensure that freshly fetched banks have valid price until we fetch oracles again
      const oldBanks = oldbanksMapByTokenIndex.get(bank.tokenIndex);
      if (oldBanks && oldBanks.length > 0) {
        merge(bank, oldBanks[0]);
      }

      const mintId = bank.mint.toString();
      if (this.banksMapByMint.has(mintId)) {
        this.banksMapByMint.get(mintId)?.push(bank);
        this.banksMapByName.get(bank.name)?.push(bank);
        this.banksMapByTokenIndex.get(bank.tokenIndex)?.push(bank);
      } else {
        this.banksMapByMint.set(mintId, [bank]);
        this.banksMapByName.set(bank.name, [bank]);
        this.banksMapByTokenIndex.set(bank.tokenIndex, [bank]);
      }
    }
  }

  public async reloadMintInfos(client: MangoClient, ids?: Id): Promise<void> {
    let mintInfos: MintInfo[];
    if (ids && ids.getMintInfos().length) {
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

  public async reloadSerum3Markets(
    client: MangoClient,
    ids?: Id,
  ): Promise<void> {
    let serum3Markets: Serum3Market[];
    if (ids && ids.getSerum3Markets().length) {
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

    this.serum3MarketsMapByExternal = new Map(
      serum3Markets.map((serum3Market) => [
        serum3Market.serumMarketExternal.toBase58(),
        serum3Market,
      ]),
    );
    this.serum3MarketsMapByMarketIndex = new Map(
      serum3Markets.map((serum3Market) => [
        serum3Market.marketIndex,
        serum3Market,
      ]),
    );
  }

  public async reloadSerum3ExternalMarkets(
    client: MangoClient,
    ids?: Id,
  ): Promise<void> {
    let markets: Market[] = [];
    const externalMarketIds = ids?.getSerum3ExternalMarkets();

    if (ids && externalMarketIds && externalMarketIds.length) {
      markets = await Promise.all(
        (
          await client.program.provider.connection.getMultipleAccountsInfo(
            externalMarketIds,
          )
        ).map(
          (account, index) =>
            new Market(
              Market.getLayout(OPENBOOK_PROGRAM_ID[client.cluster]).decode(
                account?.data,
              ),
              ids.banks.find(
                (b) =>
                  b.tokenIndex ===
                  this.serum3MarketsMapByExternal.get(
                    externalMarketIds[index].toString(),
                  )?.baseTokenIndex,
              )?.decimals || 6,
              ids.banks.find(
                (b) =>
                  b.tokenIndex ===
                  this.serum3MarketsMapByExternal.get(
                    externalMarketIds[index].toString(),
                  )?.quoteTokenIndex,
              )?.decimals || 6,
              { commitment: client.program.provider.connection.commitment },
              OPENBOOK_PROGRAM_ID[client.cluster],
            ),
        ),
      );
    } else {
      markets = await Promise.all(
        Array.from(this.serum3MarketsMapByExternal.values()).map(
          (serum3Market) =>
            Market.load(
              client.program.provider.connection,
              serum3Market.serumMarketExternal,
              { commitment: client.program.provider.connection.commitment },
              OPENBOOK_PROGRAM_ID[client.cluster],
            ),
        ),
      );
    }

    this.serum3ExternalMarketsMap = new Map(
      Array.from(this.serum3MarketsMapByExternal.values()).map(
        (serum3Market, index) => [
          serum3Market.serumMarketExternal.toBase58(),
          markets[index],
        ],
      ),
    );
  }

  public async reloadPerpMarkets(client: MangoClient, ids?: Id): Promise<void> {
    let perpMarkets: PerpMarket[];
    if (ids && ids.getPerpMarkets().length) {
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

    // ensure that freshly fetched perp markets have valid price until we fetch oracles again
    const oldPerpMarketByMarketIndex = deepClone(
      this.perpMarketsMapByMarketIndex,
    );
    for (const perpMarket of perpMarkets) {
      const oldPerpMarket = oldPerpMarketByMarketIndex.get(
        perpMarket.perpMarketIndex,
      );
      if (oldPerpMarket) {
        merge(perpMarket, oldPerpMarket);
      }
    }

    this.perpMarketsMapByName = new Map(
      perpMarkets.map((perpMarket) => [perpMarket.name, perpMarket]),
    );
    this.perpMarketsMapByOracle = new Map(
      perpMarkets.map((perpMarket) => [
        perpMarket.oracle.toBase58(),
        perpMarket,
      ]),
    );
    this.perpMarketsMapByMarketIndex = new Map(
      perpMarkets.map((perpMarket) => [perpMarket.perpMarketIndex, perpMarket]),
    );
  }

  public async reloadBankOraclePrices(client: MangoClient): Promise<void> {
    const banks: Bank[][] = Array.from(
      this.banksMapByMint,
      ([, value]) => value,
    );
    const oracles = banks.map((b) => b[0].oracle);
    const ais =
      await client.program.provider.connection.getMultipleAccountsInfo(oracles);

    const coder = new BorshAccountsCoder(client.program.idl);
    for (const [index, ai] of ais.entries()) {
      for (const bank of banks[index]) {
        if (!ai)
          throw new Error(
            `Undefined accountInfo object in reloadBankOraclePrices for ${bank.oracle}!`,
          );
        const { price, uiPrice, lastUpdatedSlot, provider, deviation } =
          await this.decodePriceFromOracleAi(
            coder,
            bank.oracle,
            ai,
            this.getMintDecimals(bank.mint),
            client,
          );
        bank._price = price;
        bank._uiPrice = uiPrice;
        bank._oracleLastUpdatedSlot = lastUpdatedSlot;
        bank._oracleProvider = provider;
        bank._oracleLastKnownDeviation = deviation;
      }
    }
  }

  public async reloadPerpMarketOraclePrices(
    client: MangoClient,
  ): Promise<void> {
    const perpMarkets: PerpMarket[] = Array.from(
      this.perpMarketsMapByName.values(),
    );
    const oracles = perpMarkets.map((b) => b.oracle);
    const ais =
      await client.program.provider.connection.getMultipleAccountsInfo(oracles);

    const coder = new BorshAccountsCoder(client.program.idl);
    await Promise.all(
      Array.from(ais.entries()).map(async ([i, ai]) => {
        const perpMarket = perpMarkets[i];
        if (!ai)
          throw new Error(
            `Undefined ai object in reloadPerpMarketOraclePrices for ${perpMarket.oracle}!`,
          );

        const { price, uiPrice, lastUpdatedSlot, provider, deviation } =
          await this.decodePriceFromOracleAi(
            coder,
            perpMarket.oracle,
            ai,
            perpMarket.baseDecimals,
            client,
          );
        perpMarket._price = price;
        perpMarket._uiPrice = uiPrice;
        perpMarket._oracleLastUpdatedSlot = lastUpdatedSlot;
        perpMarket._oracleProvider = provider;
        perpMarket._oracleLastKnownDeviation = deviation;
      }),
    );
  }

  public async decodePriceFromOracleAi(
    coder: BorshAccountsCoder<string>,
    oracle: PublicKey,
    ai: AccountInfo<Buffer>,
    baseDecimals: number,
    client: MangoClient,
  ): Promise<{
    price: I80F48;
    uiPrice: number;
    lastUpdatedSlot: number;
    provider: OracleProvider;
    deviation: I80F48;
  }> {
    let price, uiPrice, lastUpdatedSlot, provider, deviation;
    if (
      !BorshAccountsCoder.accountDiscriminator('stubOracle').compare(
        ai.data.slice(0, 8),
      )
    ) {
      const stubOracle = coder.decode('stubOracle', ai.data);
      price = new I80F48(stubOracle.price.val);
      uiPrice = this.toUiPrice(price, baseDecimals);
      lastUpdatedSlot = stubOracle.lastUpdateSlot.toNumber();
      provider = OracleProvider.Stub;
      deviation = stubOracle.deviation;
    } else if (isPythOracle(ai)) {
      const priceData = parsePriceData(ai.data);
      uiPrice = priceData.previousPrice;
      price = this.toNativePrice(uiPrice, baseDecimals);
      lastUpdatedSlot = parseInt(priceData.lastSlot.toString());
      deviation =
        priceData.previousConfidence !== undefined
          ? this.toNativePrice(priceData.previousConfidence, baseDecimals)
          : undefined;

      provider = OracleProvider.Pyth;
    } else if (isSwitchboardOracle(ai)) {
      const priceData = await parseSwitchboardOracle(
        oracle,
        ai,
        client.program.provider.connection,
      );
      uiPrice = priceData.price;
      price = this.toNativePrice(uiPrice, baseDecimals);
      lastUpdatedSlot = priceData.lastUpdatedSlot;
      deviation = this.toNativePrice(priceData.uiDeviation, baseDecimals);
      provider = OracleProvider.Switchboard;
    } else {
      throw new Error(
        `Unknown oracle provider (parsing not implemented) for oracle ${oracle}, with owner ${ai.owner}!`,
      );
    }
    return { price, uiPrice, lastUpdatedSlot, provider, deviation };
  }

  public async reloadVaults(client: MangoClient): Promise<void> {
    const vaultPks = Array.from(this.banksMapByMint.values())
      .flat()
      .map((bank) => bank.vault);
    const vaultAccounts =
      await client.program.provider.connection.getMultipleAccountsInfo(
        vaultPks,
      );
    this.vaultAmountsMap = new Map(
      vaultAccounts.map((vaultAi, i) => {
        if (!vaultAi) {
          throw new Error(`Undefined vaultAi for ${vaultPks[i]}`!);
        }
        const vaultAmount = unpackAccount(
          vaultPks[i],
          vaultAi,
          TOKEN_PROGRAM_ID,
        ).amount;
        return [vaultPks[i].toBase58(), new BN(vaultAmount.toString())];
      }),
    );
  }

  public getMintDecimals(mintPk: PublicKey): number {
    const bank = this.getFirstBankByMint(mintPk);
    return bank.mintDecimals;
  }

  public getMintDecimalsByTokenIndex(tokenIndex: TokenIndex): number {
    const bank = this.getFirstBankByTokenIndex(tokenIndex);
    return bank.mintDecimals;
  }

  public getInsuranceMintDecimals(): number {
    return this.getMintDecimals(this.insuranceMint);
  }

  public getFirstBankByMint(mintPk: PublicKey): Bank {
    const banks = this.banksMapByMint.get(mintPk.toString());
    if (!banks) throw new Error(`No bank found for mint ${mintPk}!`);
    return banks[0];
  }

  public getFirstBankByTokenIndex(tokenIndex: TokenIndex): Bank {
    const banks = this.banksMapByTokenIndex.get(tokenIndex);
    if (!banks) throw new Error(`No bank found for tokenIndex ${tokenIndex}!`);
    return banks[0];
  }

  /**
   * Returns a price impact in percentage, between 0 to 100 for a token,
   * returns -1 if data is bad
   */
  public getPriceImpactByTokenIndex(
    tokenIndex: TokenIndex,
    usdcAmountUi: number,
  ): number {
    const bank = this.getFirstBankByTokenIndex(tokenIndex);
    const pisBps = computePriceImpactOnJup(this.pis, usdcAmountUi, bank.name);
    return (pisBps * 100) / 10000;
  }

  public getFirstBankForMngo(): Bank {
    return this.getFirstBankByTokenIndex(this.mngoTokenIndex);
  }

  public getFirstBankForPerpSettlement(): Bank {
    return this.getFirstBankByTokenIndex(0 as TokenIndex);
  }

  public getTokenVaultBalanceByMint(mintPk: PublicKey): BN {
    const banks = this.banksMapByMint.get(mintPk.toBase58());
    if (!banks) {
      throw new Error(`No bank found for mint ${mintPk}!`);
    }
    const totalAmount = new BN(0);
    for (const bank of banks) {
      const amount = this.vaultAmountsMap.get(bank.vault.toBase58());
      if (!amount) {
        throw new Error(
          `Vault balance not found for bank ${bank.name} ${bank.bankNum}!`,
        );
      }
      totalAmount.iadd(amount);
    }

    return totalAmount;
  }

  /**
   *
   * @param mintPk
   * @returns sum of ui balances of vaults for all banks for a token
   */
  public getTokenVaultBalanceByMintUi(mintPk: PublicKey): number {
    return toUiDecimals(
      this.getTokenVaultBalanceByMint(mintPk),
      this.getMintDecimals(mintPk),
    );
  }

  public getSerum3MarketByMarketIndex(marketIndex: MarketIndex): Serum3Market {
    const serum3Market = this.serum3MarketsMapByMarketIndex.get(marketIndex);
    if (!serum3Market) {
      throw new Error(`No serum3Market found for marketIndex ${marketIndex}!`);
    }
    return serum3Market;
  }

  public getSerum3MarketByName(name: string): Serum3Market {
    const serum3Market = Array.from(
      this.serum3MarketsMapByExternal.values(),
    ).find((serum3Market) => serum3Market.name === name);
    if (!serum3Market) {
      throw new Error(`No serum3Market found by name ${name}!`);
    }
    return serum3Market;
  }

  public getSerum3MarketByExternalMarket(
    externalMarketPk: PublicKey,
  ): Serum3Market {
    const serum3Market = Array.from(
      this.serum3MarketsMapByExternal.values(),
    ).find((serum3Market) =>
      serum3Market.serumMarketExternal.equals(externalMarketPk),
    );
    if (!serum3Market) {
      throw new Error(
        `No serum3Market found for external serum3 market ${externalMarketPk.toString()}!`,
      );
    }
    return serum3Market;
  }

  public getSerum3ExternalMarket(externalMarketPk: PublicKey): Market {
    const market = this.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    );
    if (!market) {
      throw new Error(
        `No external market found for pk ${externalMarketPk.toString()}!`,
      );
    }
    return market;
  }

  public async loadSerum3BidsForMarket(
    client: MangoClient,
    externalMarketPk: PublicKey,
  ): Promise<Orderbook> {
    const serum3Market = this.getSerum3MarketByExternalMarket(externalMarketPk);
    return await serum3Market.loadBids(client, this);
  }

  public async loadSerum3AsksForMarket(
    client: MangoClient,
    externalMarketPk: PublicKey,
  ): Promise<Orderbook> {
    const serum3Market = this.getSerum3MarketByExternalMarket(externalMarketPk);
    return await serum3Market.loadAsks(client, this);
  }

  public findPerpMarket(marketIndex: PerpMarketIndex): PerpMarket {
    const perpMarket = Array.from(this.perpMarketsMapByName.values()).find(
      (perpMarket) => perpMarket.perpMarketIndex === marketIndex,
    );
    if (!perpMarket) {
      throw new Error(
        `No perpMarket found for perpMarketIndex ${marketIndex}!`,
      );
    }
    return perpMarket;
  }

  public getPerpMarketByOracle(oracle: PublicKey): PerpMarket {
    const perpMarket = this.perpMarketsMapByOracle.get(oracle.toBase58());
    if (!perpMarket) {
      throw new Error(`No PerpMarket found for oracle ${oracle}!`);
    }
    return perpMarket;
  }

  public getPerpMarketByMarketIndex(marketIndex: PerpMarketIndex): PerpMarket {
    const perpMarket = this.perpMarketsMapByMarketIndex.get(marketIndex);
    if (!perpMarket) {
      throw new Error(`No PerpMarket found with marketIndex ${marketIndex}!`);
    }
    return perpMarket;
  }

  public getPerpMarketByName(perpMarketName: string): PerpMarket {
    const perpMarket = Array.from(
      this.perpMarketsMapByMarketIndex.values(),
    ).find((perpMarket) => perpMarket.name === perpMarketName);
    if (!perpMarket) {
      throw new Error(`No PerpMarket found by name ${perpMarketName}!`);
    }
    return perpMarket;
  }

  public async loadPerpBidsForMarket(
    client: MangoClient,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<BookSide> {
    const perpMarket = this.getPerpMarketByMarketIndex(perpMarketIndex);
    return await perpMarket.loadBids(client);
  }

  public async loadPerpAsksForMarket(
    client: MangoClient,
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<BookSide> {
    const perpMarket = this.getPerpMarketByMarketIndex(perpMarketIndex);
    return await perpMarket.loadAsks(client);
  }

  public consoleLogBanks(): void {
    for (const mintBanks of this.banksMapByMint.values()) {
      for (const bank of mintBanks) {
        console.log(bank.toString());
      }
    }
  }

  public toUiPrice(price: I80F48 | number, baseDecimals: number): number {
    return toUiDecimals(price, this.getInsuranceMintDecimals() - baseDecimals);
  }

  public toNativePrice(uiPrice: number, baseDecimals: number): I80F48 {
    return toNativeI80F48(
      uiPrice,
      // note: our oracles are quoted in USD and our insurance mint is USD
      // please update when these assumptions change
      this.getInsuranceMintDecimals() - baseDecimals,
    );
  }

  public toNativeDecimals(uiAmount: number, mintPk: PublicKey): BN {
    const decimals = this.getMintDecimals(mintPk);
    return toNative(uiAmount, decimals);
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

    const banks: Bank[] = [];
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
