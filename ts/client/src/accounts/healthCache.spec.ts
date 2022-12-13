import { BN } from '@project-serum/anchor';
import { OpenOrders } from '@project-serum/serum';
import { expect } from 'chai';
import _ from 'lodash';
import { I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { BankForHealth, StablePriceModel, TokenIndex } from './bank';
import { HealthCache, PerpInfo, Serum3Info, TokenInfo } from './healthCache';
import { HealthType, PerpPosition } from './mangoAccount';
import { PerpMarket, PerpOrderSide } from './perp';
import { MarketIndex } from './serum3';

function mockBankAndOracle(
  tokenIndex: TokenIndex,
  maintWeight: number,
  initWeight: number,
  price: number,
  stablePrice: number,
): BankForHealth {
  return {
    tokenIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price: I80F48.fromNumber(price),
    stablePriceModel: { stablePrice: stablePrice } as StablePriceModel,
    scaledInitAssetWeight: () => I80F48.fromNumber(1 - initWeight),
    scaledInitLiabWeight: () => I80F48.fromNumber(1 + initWeight),
  };
}

function mockPerpMarket(
  perpMarketIndex: number,
  maintWeight: number,
  initWeight: number,
  baseLotSize: number,
  price: number,
): PerpMarket {
  return {
    perpMarketIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price: I80F48.fromNumber(price),
    stablePriceModel: { stablePrice: price } as StablePriceModel,
    quoteLotSize: new BN(100),
    baseLotSize: new BN(baseLotSize),
    longFunding: ZERO_I80F48(),
    shortFunding: ZERO_I80F48(),
  } as unknown as PerpMarket;
}

describe('Health Cache', () => {
  it('test_health0', () => {
    const sourceBank: BankForHealth = mockBankAndOracle(
      1 as TokenIndex,
      0.1,
      0.2,
      1,
      1,
    );
    const targetBank: BankForHealth = mockBankAndOracle(
      4 as TokenIndex,
      0.3,
      0.5,
      5,
      5,
    );

    const ti1 = TokenInfo.fromBank(sourceBank, I80F48.fromNumber(100));
    const ti2 = TokenInfo.fromBank(targetBank, I80F48.fromNumber(-10));

    const si1 = Serum3Info.fromOoModifyingTokenInfos(
      1,
      ti2,
      0,
      ti1,
      2 as MarketIndex,
      {
        quoteTokenTotal: new BN(21),
        baseTokenTotal: new BN(18),
        quoteTokenFree: new BN(1),
        baseTokenFree: new BN(3),
        referrerRebatesAccrued: new BN(2),
      } as any as OpenOrders,
    );

    const pM = mockPerpMarket(9, 0.1, 0.2, 10, targetBank.price.toNumber());
    const pp = new PerpPosition(
      pM.perpMarketIndex,
      0,
      new BN(0),
      new BN(3),
      I80F48.fromNumber(-310),
      new BN(0),
      ZERO_I80F48(),
      ZERO_I80F48(),
      new BN(7),
      new BN(11),
      new BN(1),
      new BN(2),
      0,
      0,
      new BN(0),
      new BN(0),
      new BN(0),
      0,
      ZERO_I80F48(),
    );
    const pi1 = PerpInfo.fromPerpPosition(pM, pp);

    const hc = new HealthCache([ti1, ti2], [si1], [pi1]);

    // for bank1/oracle1, including open orders (scenario: bids execute)
    const health1 = (100.0 + 1.0 + 2.0 + (20.0 + 15.0 * 5.0)) * 0.8;
    // for bank2/oracle2
    const health2 = (-10.0 + 3.0) * 5.0 * 1.5;
    // for perp (scenario: bids execute)
    const health3 =
      (3.0 + 7.0 + 1.0) * 10.0 * 5.0 * 0.8 +
      (-310.0 + 2.0 * 100.0 - 7.0 * 10.0 * 5.0);

    const health = hc.health(HealthType.init).toNumber();
    console.log(
      ` - health ${health
        .toFixed(3)
        .padStart(
          10,
        )}, case "test that includes all the side values (like referrer_rebates_accrued)"`,
    );

    expect(health - (health1 + health2 + health3)).lessThan(0.0000001);
  });

  it('test_health1', (done) => {
    function testFixture(fixture: {
      name: string;
      token1: number;
      token2: number;
      token3: number;
      oo12: [number, number];
      oo13: [number, number];
      perp1: [number, number, number, number];
      expectedHealth: number;
    }): void {
      const bank1: BankForHealth = mockBankAndOracle(
        1 as TokenIndex,
        0.1,
        0.2,
        1,
        1,
      );
      const bank2: BankForHealth = mockBankAndOracle(
        4 as TokenIndex,
        0.3,
        0.5,
        5,
        5,
      );
      const bank3: BankForHealth = mockBankAndOracle(
        5 as TokenIndex,
        0.3,
        0.5,
        10,
        10,
      );

      const ti1 = TokenInfo.fromBank(bank1, I80F48.fromNumber(fixture.token1));
      const ti2 = TokenInfo.fromBank(bank2, I80F48.fromNumber(fixture.token2));
      const ti3 = TokenInfo.fromBank(bank3, I80F48.fromNumber(fixture.token3));

      const si1 = Serum3Info.fromOoModifyingTokenInfos(
        1,
        ti2,
        0,
        ti1,
        2 as MarketIndex,
        {
          quoteTokenTotal: new BN(fixture.oo12[0]),
          baseTokenTotal: new BN(fixture.oo12[1]),
          quoteTokenFree: new BN(0),
          baseTokenFree: new BN(0),
          referrerRebatesAccrued: new BN(0),
        } as any as OpenOrders,
      );

      const si2 = Serum3Info.fromOoModifyingTokenInfos(
        2,
        ti3,
        0,
        ti1,
        2 as MarketIndex,
        {
          quoteTokenTotal: new BN(fixture.oo13[0]),
          baseTokenTotal: new BN(fixture.oo13[1]),
          quoteTokenFree: new BN(0),
          baseTokenFree: new BN(0),
          referrerRebatesAccrued: new BN(0),
        } as any as OpenOrders,
      );

      const pM = mockPerpMarket(9, 0.1, 0.2, 10, bank2.price.toNumber());
      const pp = new PerpPosition(
        pM.perpMarketIndex,
        0,
        new BN(0),
        new BN(fixture.perp1[0]),
        I80F48.fromNumber(fixture.perp1[1]),
        new BN(0),
        ZERO_I80F48(),
        ZERO_I80F48(),
        new BN(fixture.perp1[2]),
        new BN(fixture.perp1[3]),
        new BN(0),
        new BN(0),
        0,
        0,
        new BN(0),
        new BN(0),
        new BN(0),
        0,
        ZERO_I80F48(),
      );
      const pi1 = PerpInfo.fromPerpPosition(pM, pp);

      const hc = new HealthCache([ti1, ti2, ti3], [si1, si2], [pi1]);
      const health = hc.health(HealthType.init).toNumber();
      console.log(
        ` - case "${fixture.name}" health ${health.toFixed(3).padStart(10)}`,
      );
      expect(health - fixture.expectedHealth).lessThan(0.0000001);
    }

    const basePrice = 5;
    const baseLotsToQuote = 10.0 * basePrice;

    testFixture({
      name: '0',
      token1: 100,
      token2: -10,
      token3: 0,
      oo12: [20, 15],
      oo13: [0, 0],
      perp1: [3, -131, 7, 11],
      expectedHealth:
        // for token1, including open orders (scenario: bids execute)
        (100.0 + (20.0 + 15.0 * basePrice)) * 0.8 -
        // for token2
        10.0 * basePrice * 1.5 +
        // for perp (scenario: bids execute)
        (3.0 + 7.0) * baseLotsToQuote * 0.8 +
        (-131.0 - 7.0 * baseLotsToQuote),
    });

    testFixture({
      name: '1',
      token1: -100,
      token2: 10,
      token3: 0,
      oo12: [20, 15],
      oo13: [0, 0],
      perp1: [-10, -131, 7, 11],
      expectedHealth:
        // for token1
        -100.0 * 1.2 +
        // for token2, including open orders (scenario: asks execute)
        (10.0 * basePrice + (20.0 + 15.0 * basePrice)) * 0.5 +
        // for perp (scenario: asks execute)
        (-10.0 - 11.0) * baseLotsToQuote * 1.2 +
        (-131.0 + 11.0 * baseLotsToQuote),
    });

    testFixture({
      name: '2',
      token1: 0,
      token2: 0,
      token3: 0,
      oo12: [0, 0],
      oo13: [0, 0],
      perp1: [-10, 100, 0, 0],
      expectedHealth: 0,
    });

    testFixture({
      name: '3',
      token1: 0,
      token2: 0,
      token3: 0,
      oo12: [0, 0],
      oo13: [0, 0],
      perp1: [1, -100, 0, 0],
      expectedHealth: -100.0 + 0.8 * 1.0 * baseLotsToQuote,
    });

    testFixture({
      name: '4',
      token1: 0,
      token2: 0,
      token3: 0,
      oo12: [0, 0],
      oo13: [0, 0],
      perp1: [10, 100, 0, 0],
      expectedHealth: 0,
    });

    testFixture({
      name: '5',
      token1: 0,
      token2: 0,
      token3: 0,
      oo12: [0, 0],
      oo13: [0, 0],
      perp1: [30, -100, 0, 0],
      expectedHealth: 0,
    });

    testFixture({
      name: '6, reserved oo funds',
      token1: -100,
      token2: -10,
      token3: -10,
      oo12: [1, 1],
      oo13: [1, 1],
      perp1: [30, -100, 0, 0],
      expectedHealth:
        // tokens
        -100.0 * 1.2 -
        10.0 * 5.0 * 1.5 -
        10.0 * 10.0 * 1.5 +
        // oo_1_2 (-> token1)
        (1.0 + 5.0) * 1.2 +
        // oo_1_3 (-> token1)
        (1.0 + 10.0) * 1.2,
    });

    testFixture({
      name: '7, reserved oo funds cross the zero balance level',
      token1: -14,
      token2: -10,
      token3: -10,
      oo12: [1, 1],
      oo13: [1, 1],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        -14.0 * 1.2 -
        10.0 * 5.0 * 1.5 -
        10.0 * 10.0 * 1.5 +
        // oo_1_2 (-> token1)
        3.0 * 1.2 +
        3.0 * 0.8 +
        // oo_1_3 (-> token1)
        8.0 * 1.2 +
        3.0 * 0.8,
    });

    testFixture({
      name: '8, reserved oo funds in a non-quote currency',
      token1: -100,
      token2: -100,
      token3: -1,
      oo12: [0, 0],
      oo13: [10, 1],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // tokens
        -100.0 * 1.2 -
        100.0 * 5.0 * 1.5 -
        10.0 * 1.5 +
        // oo_1_3 (-> token3)
        10.0 * 1.5 +
        10.0 * 0.5,
    });

    testFixture({
      name: '9, like 8 but oo_1_2 flips the oo_1_3 target',
      token1: -100,
      token2: -100,
      token3: -1,
      oo12: [100, 0],
      oo13: [10, 1],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // tokens
        -100.0 * 1.2 -
        100.0 * 5.0 * 1.5 -
        10.0 * 1.5 +
        // oo_1_2 (-> token1)
        80.0 * 1.2 +
        20.0 * 0.8 +
        // oo_1_3 (-> token1)
        20.0 * 0.8,
    });

    done();
  });

  it('test_max_swap', (done) => {
    const b0 = mockBankAndOracle(0 as TokenIndex, 0.1, 0.1, 2, 2);
    const b1 = mockBankAndOracle(1 as TokenIndex, 0.2, 0.2, 3, 3);
    const b2 = mockBankAndOracle(2 as TokenIndex, 0.3, 0.3, 4, 4);
    const banks = [b0, b1, b2];
    const hc = new HealthCache(
      [
        TokenInfo.fromBank(b0, I80F48.fromNumber(0)),
        TokenInfo.fromBank(b1, I80F48.fromNumber(0)),
        TokenInfo.fromBank(b2, I80F48.fromNumber(0)),
      ],
      [],
      [],
    );

    expect(
      hc
        .getMaxSourceForTokenSwap(
          b0,
          b1,
          I80F48.fromNumber(2 / 3),
          I80F48.fromNumber(50),
        )
        .toNumber(),
    ).lessThan(0.0000001);

    function findMaxSwapActual(
      hc: HealthCache,
      source: TokenIndex,
      target: TokenIndex,
      ratio: number,
      priceFactor: number,
    ): I80F48[] {
      const clonedHc: HealthCache = _.cloneDeep(hc);

      const sourcePrice = clonedHc.tokenInfos[source].prices;
      const targetPrice = clonedHc.tokenInfos[target].prices;
      const swapPrice = I80F48.fromNumber(priceFactor)
        .mul(sourcePrice.oracle)
        .div(targetPrice.oracle);
      const sourceAmount = clonedHc.getMaxSourceForTokenSwap(
        banks[source],
        banks[target],
        swapPrice,
        I80F48.fromNumber(ratio),
      );

      // adjust token balance
      clonedHc.tokenInfos[source].balanceNative.isub(sourceAmount);
      clonedHc.tokenInfos[target].balanceNative.iadd(
        sourceAmount.mul(swapPrice),
      );

      return [sourceAmount, clonedHc.healthRatio(HealthType.init)];
    }

    function checkMaxSwapResult(
      hc: HealthCache,
      source: TokenIndex,
      target: TokenIndex,
      ratio: number,
      priceFactor: number,
    ): void {
      const [sourceAmount, actualRatio] = findMaxSwapActual(
        hc,
        source,
        target,
        ratio,
        priceFactor,
      );
      console.log(
        ` -- checking ${source} to ${target} for priceFactor: ${priceFactor}, target ratio ${ratio}: actual ratio: ${actualRatio}, amount: ${sourceAmount}`,
      );
      expect(Math.abs(actualRatio.toNumber() - ratio)).lessThan(1);
    }

    {
      console.log(' - test 0');
      // adjust by usdc
      const clonedHc = _.cloneDeep(hc);
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
      );

      for (const priceFactor of [0.1, 0.9, 1.1]) {
        for (const target of _.range(1, 100, 1)) {
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            1 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            1 as TokenIndex,
            0 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            2 as TokenIndex,
            target,
            priceFactor,
          );
        }
      }

      // At this unlikely price it's healthy to swap infinitely
      expect(function () {
        findMaxSwapActual(
          clonedHc,
          0 as TokenIndex,
          1 as TokenIndex,
          50.0,
          1.5,
        );
      }).to.throw('Number out of range');
    }

    {
      console.log(' - test 1');
      const clonedHc = _.cloneDeep(hc);
      // adjust by usdc
      clonedHc.tokenInfos[0].balanceNative.iadd(
        I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
      );
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
      );

      for (const priceFactor of [0.1, 0.9, 1.1]) {
        for (const target of _.range(1, 100, 1)) {
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            1 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            1 as TokenIndex,
            0 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            2 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            2 as TokenIndex,
            0 as TokenIndex,
            target,
            priceFactor,
          );
        }
      }
    }

    {
      console.log(' - test 2');
      const clonedHc = _.cloneDeep(hc);
      // adjust by usdc
      clonedHc.tokenInfos[0].balanceNative.iadd(
        I80F48.fromNumber(-50).div(clonedHc.tokenInfos[0].prices.oracle),
      );
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
      );
      // possible even though the init ratio is <100
      checkMaxSwapResult(clonedHc, 1 as TokenIndex, 0 as TokenIndex, 100, 1);
    }

    {
      console.log(' - test 3');
      const clonedHc = _.cloneDeep(hc);
      // adjust by usdc
      clonedHc.tokenInfos[0].balanceNative.iadd(
        I80F48.fromNumber(-30).div(clonedHc.tokenInfos[0].prices.oracle),
      );
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
      );
      clonedHc.tokenInfos[2].balanceNative.iadd(
        I80F48.fromNumber(-30).div(clonedHc.tokenInfos[2].prices.oracle),
      );

      // swapping with a high ratio advises paying back all liabs
      // and then swapping even more because increasing assets in 0 has better asset weight
      const initRatio = clonedHc.healthRatio(HealthType.init);
      const [amount, actualRatio] = findMaxSwapActual(
        clonedHc,
        1 as TokenIndex,
        0 as TokenIndex,
        100,
        1,
      );
      expect(actualRatio.div(I80F48.fromNumber(2)).toNumber()).greaterThan(
        initRatio.toNumber(),
      );
      expect(amount.toNumber() - 100 / 3).lessThan(1);
    }

    {
      console.log(' - test 4');
      const clonedHc = _.cloneDeep(hc);
      // adjust by usdc
      clonedHc.tokenInfos[0].balanceNative.iadd(
        I80F48.fromNumber(100).div(clonedHc.tokenInfos[0].prices.oracle),
      );
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(-2).div(clonedHc.tokenInfos[1].prices.oracle),
      );
      clonedHc.tokenInfos[2].balanceNative.iadd(
        I80F48.fromNumber(-65).div(clonedHc.tokenInfos[2].prices.oracle),
      );

      const initRatio = clonedHc.healthRatio(HealthType.init);
      expect(initRatio.toNumber()).greaterThan(3);
      expect(initRatio.toNumber()).lessThan(4);

      checkMaxSwapResult(clonedHc, 0 as TokenIndex, 1 as TokenIndex, 1, 1);
      checkMaxSwapResult(clonedHc, 0 as TokenIndex, 1 as TokenIndex, 3, 1);
      checkMaxSwapResult(clonedHc, 0 as TokenIndex, 1 as TokenIndex, 4, 1);
    }

    // TODO test 5

    {
      console.log(' - test 6');
      const clonedHc = _.cloneDeep(hc);
      clonedHc.serum3Infos = [
        new Serum3Info(
          I80F48.fromNumber(30 / 3),
          I80F48.fromNumber(30 / 2),
          1,
          0,
          0 as MarketIndex,
        ),
      ];

      // adjust by usdc
      clonedHc.tokenInfos[0].balanceNative.iadd(
        I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
      );
      clonedHc.tokenInfos[1].balanceNative.iadd(
        I80F48.fromNumber(-40).div(clonedHc.tokenInfos[1].prices.oracle),
      );
      clonedHc.tokenInfos[2].balanceNative.iadd(
        I80F48.fromNumber(120).div(clonedHc.tokenInfos[2].prices.oracle),
      );

      for (const priceFactor of [
        // 0.9,

        1.1,
      ]) {
        for (const target of _.range(1, 100, 1)) {
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            1 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            1 as TokenIndex,
            0 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            0 as TokenIndex,
            2 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            1 as TokenIndex,
            2 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            2 as TokenIndex,
            0 as TokenIndex,
            target,
            priceFactor,
          );
          checkMaxSwapResult(
            clonedHc,
            2 as TokenIndex,
            1 as TokenIndex,
            target,
            priceFactor,
          );
        }
      }
    }

    done();
  });

  it('test_max_perp', (done) => {
    const baseLotSize = 100;
    const b0 = mockBankAndOracle(0 as TokenIndex, 0.0, 0.0, 1, 1);
    const p0 = mockPerpMarket(0, 0.3, 0.3, baseLotSize, 2, 2);
    const hc = new HealthCache(
      [TokenInfo.fromBank(b0, I80F48.fromNumber(0))],
      [],
      [PerpInfo.emptyFromPerpMarket(p0)],
    );

    expect(hc.health(HealthType.init).toNumber()).equals(0);

    expect(
      hc
        .getMaxPerpForHealthRatio(
          p0,
          I80F48.fromNumber(2),
          PerpOrderSide.bid,
          I80F48.fromNumber(50),
        )
        .toNumber(),
    ).equals(0);

    function findMaxTrade(
      hc: HealthCache,
      side: PerpOrderSide,
      ratio: number,
      priceFactor: number,
    ): number[] {
      const prices = hc.perpInfos[0].prices;
      const tradePrice = I80F48.fromNumber(priceFactor).mul(prices.oracle);
      const baseLots0 = hc
        .getMaxPerpForHealthRatio(
          p0,
          tradePrice,
          side,
          I80F48.fromNumber(ratio),
        )
        .toNumber();

      const direction = side == PerpOrderSide.bid ? 1 : -1;

      // compute the health ratio we'd get when executing the trade
      const baseLots1 = direction * baseLots0;
      let baseNative = I80F48.fromNumber(baseLots1).mul(
        I80F48.fromNumber(baseLotSize),
      );
      let hcClone: HealthCache = _.cloneDeep(hc);
      hcClone.perpInfos[0].baseLots.iadd(new BN(baseLots1));
      hcClone.perpInfos[0].quote.isub(baseNative.mul(tradePrice));
      const actualRatio = hcClone.healthRatio(HealthType.init);

      // the ratio for trading just one base lot extra
      const baseLots2 = direction * (baseLots0 + 1);
      baseNative = I80F48.fromNumber(baseLots2 * baseLotSize);
      hcClone = _.cloneDeep(hc);
      hcClone.perpInfos[0].baseLots.iadd(new BN(baseLots2));
      hcClone.perpInfos[0].quote.isub(baseNative.mul(tradePrice));
      const plusRatio = hcClone.healthRatio(HealthType.init);

      return [baseLots0, actualRatio.toNumber(), plusRatio.toNumber()];
    }

    function checkMaxTrade(
      hc: HealthCache,
      side: PerpOrderSide,
      ratio: number,
      priceFactor: number,
    ): void {
      const [baseLots, actualRatio, plusRatio] = findMaxTrade(
        hc,
        side,
        ratio,
        priceFactor,
      );
      console.log(
        `checking for price_factor: ${priceFactor}, target ratio ${ratio}: actual ratio: ${actualRatio}, plus ratio: ${plusRatio}, base_lots: ${baseLots}`,
      );
      expect(ratio).lessThan(actualRatio);
      expect(plusRatio - 0.1).lessThanOrEqual(ratio);
    }

    // adjust token
    hc.tokenInfos[0].balanceNative.iadd(I80F48.fromNumber(3000));
    for (const existing of [-5, 0, 3]) {
      const hcClone: HealthCache = _.cloneDeep(hc);
      hcClone.perpInfos[0].baseLots.iadd(new BN(existing));
      hcClone.perpInfos[0].quote.isub(
        I80F48.fromNumber(existing * baseLotSize * 2),
      );
      for (const side of [PerpOrderSide.bid, PerpOrderSide.ask]) {
        console.log(
          `existing ${existing} ${side === PerpOrderSide.bid ? 'bid' : 'ask'}`,
        );
        for (const priceFactor of [0.8, 1.0, 1.1]) {
          for (const ratio of _.range(1, 101, 1)) {
            checkMaxTrade(hcClone, side, ratio, priceFactor);
          }
        }
      }
    }

    // check some extremely bad prices
    checkMaxTrade(hc, PerpOrderSide.bid, 50, 2);
    checkMaxTrade(hc, PerpOrderSide.ask, 50, 0.1);

    // and extremely good prices
    expect(function () {
      findMaxTrade(hc, PerpOrderSide.bid, 50, 0.1);
    }).to.throw();
    expect(function () {
      findMaxTrade(hc, PerpOrderSide.ask, 50, 1.5);
    }).to.throw();

    done();
  });
});
