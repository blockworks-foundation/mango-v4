import { BN } from '@coral-xyz/anchor';
import { OpenOrders } from '@project-serum/serum';
import { expect } from 'chai';
import range from 'lodash/range';

import { PublicKey } from '@solana/web3.js';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { BankForHealth, StablePriceModel, TokenIndex } from './bank';
import { HealthCache, PerpInfo, Serum3Info, TokenInfo } from './healthCache';
import { HealthType, PerpPosition, Serum3Orders } from './mangoAccount';
import { PerpMarket, PerpOrderSide } from './perp';
import { MarketIndex } from './serum3';
import { deepClone } from '../utils';

function mockBankAndOracle(
  tokenIndex: TokenIndex,
  maintWeight: number,
  initWeight: number,
  price: number,
  stablePrice: number,
  deposits = 0,
  borrows = 0,
  borrowWeightScaleStartQuote = Number.MAX_SAFE_INTEGER,
  depositWeightScaleStartQuote = Number.MAX_SAFE_INTEGER,
): BankForHealth {
  return {
    tokenIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price: I80F48.fromNumber(price),
    stablePriceModel: { stablePrice: stablePrice } as StablePriceModel,
    scaledInitAssetWeight: (price: I80F48): I80F48 => {
      const depositsQuote = I80F48.fromNumber(deposits).mul(price);
      if (
        depositWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
        depositsQuote.lte(I80F48.fromNumber(depositWeightScaleStartQuote))
      ) {
        return I80F48.fromNumber(1 - initWeight);
      }
      return I80F48.fromNumber(1 - initWeight).mul(
        I80F48.fromNumber(depositWeightScaleStartQuote).div(depositsQuote),
      );
    },
    scaledInitLiabWeight: (price: I80F48): I80F48 => {
      const borrowsQuote = I80F48.fromNumber(borrows).mul(price);
      if (
        borrowWeightScaleStartQuote >= Number.MAX_SAFE_INTEGER ||
        borrowsQuote.lte(I80F48.fromNumber(borrowWeightScaleStartQuote))
      ) {
        return I80F48.fromNumber(1 + initWeight);
      }
      return I80F48.fromNumber(1 + initWeight).mul(
        borrowsQuote.div(I80F48.fromNumber(borrowWeightScaleStartQuote)),
      );
    },
    nativeDeposits: () => I80F48.fromNumber(deposits),
    nativeBorrows: () => I80F48.fromNumber(borrows),
    maintWeights: () => [
      I80F48.fromNumber(1 - maintWeight),
      I80F48.fromNumber(1 + maintWeight),
    ],
    borrowWeightScaleStartQuote: borrowWeightScaleStartQuote,
    depositWeightScaleStartQuote: depositWeightScaleStartQuote,
  };
}

function mockPerpMarket(
  perpMarketIndex: number,
  settleTokenIndex: number,
  maintBaseWeight: number,
  initBaseWeight: number,
  baseLotSize: number,
  price: number,
): PerpMarket {
  return {
    perpMarketIndex,
    settleTokenIndex: settleTokenIndex as TokenIndex,
    maintBaseAssetWeight: I80F48.fromNumber(1 - maintBaseWeight),
    initBaseAssetWeight: I80F48.fromNumber(1 - initBaseWeight),
    maintBaseLiabWeight: I80F48.fromNumber(1 + maintBaseWeight),
    initBaseLiabWeight: I80F48.fromNumber(1 + initBaseWeight),
    maintOverallAssetWeight: I80F48.fromNumber(1 - 0.02),
    initOverallAssetWeight: I80F48.fromNumber(1 - 0.05),
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
      0 as TokenIndex,
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
      new Serum3Orders(
        PublicKey.default,
        2 as MarketIndex,
        4 as TokenIndex,
        0 as TokenIndex,
        0,
        0,
      ),
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

    const pM = mockPerpMarket(9, 0, 0.1, 0.2, 10, targetBank.price.toNumber());
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
      ZERO_I80F48(),
      new BN(0),
      ZERO_I80F48(),
    );
    const pi1 = PerpInfo.fromPerpPosition(pM, pp);

    const hc = new HealthCache([ti1, ti2], [si1], [pi1]);

    // for bank1/oracle1
    // including open orders (scenario: bids execute)
    const serum1 = 1.0 + (20.0 + 15.0 * 5.0);
    // and perp (scenario: bids execute)
    const perp1 =
      (3.0 + 7.0 + 1.0) * 10.0 * 5.0 * 0.8 +
      (-310.0 + 2.0 * 100.0 - 7.0 * 10.0 * 5.0);
    const health1 = (100.0 + serum1 + perp1) * 0.8;
    // for bank2/oracle2
    const health2 = (-10.0 + 3.0) * 5.0 * 1.5;

    const health = hc.health(HealthType.init).toNumber();
    console.log(
      ` - health ${health
        .toFixed(3)
        .padStart(
          10,
        )}, case "test that includes all the side values (like referrer_rebates_accrued)"`,
    );

    expect(health - (health1 + health2)).lessThan(0.0000001);
  });

  it('test_health1', (done) => {
    function testFixture(fixture: {
      name: string;
      token1: number;
      token2: number;
      token3: number;
      bs1: [number, number];
      bs2: [number, number];
      bs3: [number, number];
      oo12: [number, number];
      oo13: [number, number];
      sa12: [number, number];
      sa13: [number, number];
      perp1: [number, number, number, number];
      expectedHealth: number;
    }): void {
      const bank1: BankForHealth = mockBankAndOracle(
        0 as TokenIndex,
        0.1,
        0.2,
        1,
        1,
        fixture.bs1[0],
        fixture.bs1[0],
        fixture.bs1[1],
        fixture.bs1[1],
      );

      const bank2: BankForHealth = mockBankAndOracle(
        4 as TokenIndex,
        0.3,
        0.5,
        5,
        5,
        fixture.bs2[0],
        fixture.bs2[0],
        fixture.bs2[1],
        fixture.bs2[1],
      );
      const bank3: BankForHealth = mockBankAndOracle(
        5 as TokenIndex,
        0.3,
        0.5,
        10,
        10,
        fixture.bs3[0],
        fixture.bs3[0],
        fixture.bs3[1],
        fixture.bs3[1],
      );

      const ti1 = TokenInfo.fromBank(bank1, I80F48.fromNumber(fixture.token1));
      const ti2 = TokenInfo.fromBank(bank2, I80F48.fromNumber(fixture.token2));
      const ti3 = TokenInfo.fromBank(bank3, I80F48.fromNumber(fixture.token3));

      const si1 = Serum3Info.fromOoModifyingTokenInfos(
        new Serum3Orders(
          PublicKey.default,
          2 as MarketIndex,
          4 as TokenIndex,
          0 as TokenIndex,
          fixture.sa12[0],
          fixture.sa12[1],
        ),
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
        new Serum3Orders(
          PublicKey.default,
          3 as MarketIndex,
          5 as TokenIndex,
          0 as TokenIndex,
          fixture.sa13[0],
          fixture.sa13[1],
        ),
        2,
        ti3,
        0,
        ti1,
        3 as MarketIndex,
        {
          quoteTokenTotal: new BN(fixture.oo13[0]),
          baseTokenTotal: new BN(fixture.oo13[1]),
          quoteTokenFree: new BN(0),
          baseTokenFree: new BN(0),
          referrerRebatesAccrued: new BN(0),
        } as any as OpenOrders,
      );

      const pM = mockPerpMarket(9, 0, 0.1, 0.2, 10, bank2.price.toNumber());
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
        ZERO_I80F48(),
        new BN(0),
        ZERO_I80F48(),
      );
      const pi1 = PerpInfo.fromPerpPosition(pM, pp);

      const hc = new HealthCache([ti1, ti2, ti3], [si1, si2], [pi1]);
      const health = hc.health(HealthType.init).toNumber();
      console.log(
        ` - case "${fixture.name}" health ${health
          .toFixed(3)
          .padStart(10)}, expected ${fixture.expectedHealth}`,
      );
      expect(Math.abs(health - fixture.expectedHealth)).lessThan(0.0000001);
    }

    const basePrice = 5;
    const baseLotsToQuote = 10.0 * basePrice;

    testFixture({
      name: '0',
      token1: 100,
      token2: -10,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [20, 15],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [3, -131, 7, 11],
      expectedHealth:
        // for token1
        0.8 *
          (100.0 +
            // including open orders (scenario: bids execute)
            (20.0 + 15.0 * basePrice) +
            // including perp (scenario: bids execute)
            (3.0 + 7.0) * baseLotsToQuote * 0.8 +
            (-131.0 - 7.0 * baseLotsToQuote)) -
        // for token2
        10.0 * basePrice * 1.5,
    });

    testFixture({
      name: '1',
      token1: -100,
      token2: 10,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [20, 15],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [-10, -131, 7, 11],
      expectedHealth:
        // for token1
        1.2 *
          (-100.0 +
            // for perp (scenario: asks execute)
            (-10.0 - 11.0) * baseLotsToQuote * 1.2 +
            (-131.0 + 11.0 * baseLotsToQuote)) +
        // for token2, including open orders (scenario: asks execute)
        (10.0 * basePrice + (20.0 + 15.0 * basePrice)) * 0.5,
    });

    testFixture({
      name: '2: weighted positive perp pnl',
      token1: 0,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [-1, 100, 0, 0],
      expectedHealth: 0.8 * 0.95 * (100.0 - 1.2 * 1.0 * baseLotsToQuote),
    });

    testFixture({
      name: '3: negative perp pnl is not weighted',
      token1: 0,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [1, -100, 0, 0],
      expectedHealth: 1.2 * (-100.0 + 0.8 * 1.0 * baseLotsToQuote),
    });

    testFixture({
      name: '4: perp health',
      token1: 0,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [10, 100, 0, 0],
      expectedHealth: 0.8 * 0.95 * (100.0 + 0.8 * 10.0 * baseLotsToQuote),
    });

    testFixture({
      name: '5: perp health',
      token1: 0,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [30, -100, 0, 0],
      expectedHealth: 0.8 * 0.95 * (-100.0 + 0.8 * 30.0 * baseLotsToQuote),
    });

    testFixture({
      name: '6, reserved oo funds',
      token1: -100,
      token2: -10,
      token3: -10,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [1, 1],
      oo13: [1, 1],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [0, 0, 0, 0],
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
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [1, 1],
      oo13: [1, 1],
      sa12: [0, 0],
      sa13: [0, 0],
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
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [10, 1],
      sa12: [0, 0],
      sa13: [0, 0],
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
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [100, 0],
      oo13: [10, 1],
      sa12: [0, 0],
      sa13: [0, 0],
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

    testFixture({
      name: '10, checking collateral limit',
      token1: 100,
      token2: 100,
      token3: 100,
      bs1: [100, 1000],
      bs2: [1500, 5000],
      bs3: [10000, 10000],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // token1
        0.8 * 100.0 +
        // token2
        0.5 * 100.0 * 5.0 * (5000.0 / (1500.0 * 5.0)) +
        // token3
        0.5 * 100.0 * 10.0 * (10000.0 / (10000.0 * 10.0)),
    });

    testFixture({
      name: '11, checking borrow limit',
      token1: -100,
      token2: -100,
      token3: -100,
      bs1: [100, 1000],
      bs2: [1500, 5000],
      bs3: [10000, 10000],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // token1
        -1.2 * 100.0 -
        // token2
        1.5 * 100.0 * 5.0 * ((1500.0 * 5.0) / 5000.0) -
        // token3
        1.5 * 100.0 * 10.0 * ((10000.0 * 10.0) / 10000.0),
    });

    testFixture({
      name: '12, positive perp health offsets token borrow',
      token1: -100,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [1, 100, 0, 0],
      expectedHealth:
        0.8 * (-100.0 + 0.95 * (100.0 + 0.8 * 1.0 * baseLotsToQuote)),
    });

    testFixture({
      name: '13, negative perp health offsets token deposit',
      token1: 100,
      token2: 0,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [0, 0],
      oo13: [0, 0],
      sa12: [0, 0],
      sa13: [0, 0],
      perp1: [-1, -100, 0, 0],
      expectedHealth: 1.2 * (100.0 - 100.0 - 1.2 * 1.0 * baseLotsToQuote),
    });

    testFixture({
      name: '14, reserved oo funds with max bid/min ask',
      token1: -100,
      token2: -10,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [1, 1],
      oo13: [11, 1],
      sa12: [0, 3],
      sa13: [1.0 / 12.0, 0],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // tokens
        -100.0 * 1.2 -
        10.0 * 5.0 * 1.5 +
        // oo_1_2 (-> token1)
        (1.0 + 3.0) * 1.2 +
        // oo_1_3 (-> token3)
        (11.0 / 12.0 + 1.0) * 10.0 * 0.5,
    });

    testFixture({
      name: '15, reserved oo funds with max bid/min ask not crossing oracle',
      token1: -100,
      token2: -10,
      token3: 0,
      bs1: [0, Number.MAX_SAFE_INTEGER],
      bs2: [0, Number.MAX_SAFE_INTEGER],
      bs3: [0, Number.MAX_SAFE_INTEGER],
      oo12: [1, 1],
      oo13: [11, 1],
      sa12: [0, 6],
      sa13: [1.0 / 9.0, 0],
      perp1: [0, 0, 0, 0],
      expectedHealth:
        // tokens
        -100.0 * 1.2 -
        10.0 * 5.0 * 1.5 +
        // oo_1_2 (-> token1)
        (1.0 + 5.0) * 1.2 +
        // oo_1_3 (-> token3)
        (11.0 / 10.0 + 1.0) * 10.0 * 0.5,
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
        .getMaxSwapSourceForHealthRatio(
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
      minValue: number,
      priceFactor: number,
      maxSwapFn: (HealthCache) => I80F48,
    ): number[] {
      const clonedHc: HealthCache = deepClone<HealthCache>(hc);

      const sourcePrice = clonedHc.tokenInfos[source].prices;
      const targetPrice = clonedHc.tokenInfos[target].prices;
      const swapPrice = I80F48.fromNumber(priceFactor)
        .mul(sourcePrice.oracle)
        .div(targetPrice.oracle);
      const sourceAmount = clonedHc.getMaxSwapSourceForHealthFn(
        banks[source],
        banks[target],
        swapPrice,
        I80F48.fromNumber(minValue),
        maxSwapFn,
      );

      function valueForAmount(amount: I80F48): I80F48 {
        // adjust token balance
        const clonedHcClone: HealthCache = deepClone<HealthCache>(clonedHc);
        clonedHc.tokenInfos[source].balanceSpot.isub(amount);
        clonedHc.tokenInfos[target].balanceSpot.iadd(amount.mul(swapPrice));
        return maxSwapFn(clonedHcClone);
      }

      return [
        sourceAmount.toNumber(),
        valueForAmount(sourceAmount).toNumber(),
        valueForAmount(sourceAmount.sub(ONE_I80F48())).toNumber(),
        valueForAmount(sourceAmount.add(ONE_I80F48())).toNumber(),
      ];
    }

    function checkMaxSwapResult(
      hc: HealthCache,
      source: TokenIndex,
      target: TokenIndex,
      minValue: number,
      priceFactor: number,
      maxSwapFn: (HealthCache) => I80F48,
    ): void {
      const [sourceAmount, actualValue, minusValue, plusValue] =
        findMaxSwapActual(hc, source, target, minValue, priceFactor, maxSwapFn);
      console.log(
        ` -- checking ${source} to ${target} for priceFactor: ${priceFactor}, target: ${minValue} actual: ${minusValue}/${actualValue}/${plusValue}, amount: ${sourceAmount}`,
      );
      if (actualValue < minValue) {
        // check that swapping more would decrease the ratio!
        expect(plusValue < actualValue);
      } else {
        expect(actualValue >= minValue);
        // either we're within tolerance of the target, or swapping 1 more would
        // bring us below the target
        expect(actualValue < minValue + 1 || plusValue < minValue);
      }
    }

    function maxSwapFnRatio(hc: HealthCache): I80F48 {
      return hc.healthRatio(HealthType.init);
    }

    function maxSwapFn(hc: HealthCache): I80F48 {
      return hc.health(HealthType.init);
    }

    for (const fn of [maxSwapFn, maxSwapFnRatio]) {
      {
        console.log(' - test 0');
        // adjust by usdc
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
        );

        for (const priceFactor of [0.1, 0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              1 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              2 as TokenIndex,
              target,
              priceFactor,
              fn,
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
            fn,
          );
        }).to.throw('Number out of range');
      }

      {
        console.log(' - test 1');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
        );

        for (const priceFactor of [0.1, 0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              1 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              2 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              2 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
          }
        }
      }

      {
        console.log(' - test 2');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-50).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        // possible even though the init ratio is <100
        checkMaxSwapResult(
          clonedHc,
          1 as TokenIndex,
          0 as TokenIndex,
          100,
          1,

          maxSwapFn,
        );
      }

      {
        console.log(' - test 3');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-30).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(100).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        clonedHc.tokenInfos[2].balanceSpot.iadd(
          I80F48.fromNumber(-30).div(clonedHc.tokenInfos[2].prices.oracle),
        );

        // swapping with a high ratio advises paying back all liabs
        // and then swapping even more because increasing assets in 0 has better asset weight
        const initRatio = clonedHc.healthRatio(HealthType.init).toNumber();
        const [amount, actualRatio] = findMaxSwapActual(
          clonedHc,
          1 as TokenIndex,
          0 as TokenIndex,
          100,
          1,
          maxSwapFn,
        );
        expect(actualRatio / 2.0 > initRatio);
        expect(amount - 100 / 3).lessThan(1);
      }

      {
        console.log(' - test 4');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(100).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(-2).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        clonedHc.tokenInfos[2].balanceSpot.iadd(
          I80F48.fromNumber(-65).div(clonedHc.tokenInfos[2].prices.oracle),
        );

        const initRatio = clonedHc.healthRatio(HealthType.init);
        expect(initRatio.toNumber()).greaterThan(3);
        expect(initRatio.toNumber()).lessThan(4);

        checkMaxSwapResult(
          clonedHc,
          0 as TokenIndex,
          1 as TokenIndex,
          1,
          1,
          maxSwapFn,
        );
        checkMaxSwapResult(
          clonedHc,
          0 as TokenIndex,
          1 as TokenIndex,
          3,
          1,
          maxSwapFn,
        );
        checkMaxSwapResult(
          clonedHc,
          0 as TokenIndex,
          1 as TokenIndex,
          4,
          1,
          maxSwapFn,
        );
      }

      // TODO test 5

      {
        console.log(' - test 6');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);
        clonedHc.serum3Infos = [
          new Serum3Info(
            I80F48.fromNumber(30 / 3),
            I80F48.fromNumber(30 / 2),
            ZERO_I80F48(),
            ZERO_I80F48(),
            1,
            0,
            0 as MarketIndex,
          ),
        ];

        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(-40).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        clonedHc.tokenInfos[2].balanceSpot.iadd(
          I80F48.fromNumber(120).div(clonedHc.tokenInfos[2].prices.oracle),
        );

        for (const priceFactor of [0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              1 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              2 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              2 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              2 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
            checkMaxSwapResult(
              clonedHc,
              2 as TokenIndex,
              1 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
          }
        }
      }

      {
        // check starting with negative health but swapping can make it positive
        console.log(' - test 7');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);

        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(20).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        expect(clonedHc.health(HealthType.init).toNumber() < 0);

        for (const priceFactor of [0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
          }
        }
      }

      {
        // check starting with negative health but swapping can't make it positive
        console.log(' - test 8');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);

        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(-20).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].balanceSpot.iadd(
          I80F48.fromNumber(10).div(clonedHc.tokenInfos[1].prices.oracle),
        );
        expect(clonedHc.health(HealthType.init).toNumber() < 0);

        for (const priceFactor of [0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              1 as TokenIndex,
              0 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
          }
        }
      }

      {
        // swap some assets into a zero-asset-weight token
        console.log(' - test 9');
        const clonedHc: HealthCache = deepClone<HealthCache>(hc);

        // adjust by usdc
        clonedHc.tokenInfos[0].balanceSpot.iadd(
          I80F48.fromNumber(10).div(clonedHc.tokenInfos[0].prices.oracle),
        );
        clonedHc.tokenInfos[1].initAssetWeight = ZERO_I80F48();
        expect(
          findMaxSwapActual(
            clonedHc,
            0 as TokenIndex,
            1 as TokenIndex,
            1,
            1,
            maxSwapFn,
          )[0] > 0,
        );

        for (const priceFactor of [0.9, 1.1]) {
          for (const target of range(1, 100, 1)) {
            checkMaxSwapResult(
              clonedHc,
              0 as TokenIndex,
              1 as TokenIndex,
              target,
              priceFactor,
              fn,
            );
          }
        }
      }
    }
    done();
  });

  it('test_max_perp', (done) => {
    const baseLotSize = 100;
    const b0 = mockBankAndOracle(0 as TokenIndex, 0.0, 0.0, 1, 1);
    const b1 = mockBankAndOracle(1 as TokenIndex, 0.2, 0.2, 1.5, 1.5);
    const p0 = mockPerpMarket(0, 1, 0.3, 0.3, baseLotSize, 2);
    const hc = new HealthCache(
      [
        TokenInfo.fromBank(b0, I80F48.fromNumber(0)),
        TokenInfo.fromBank(b1, I80F48.fromNumber(0)),
      ],
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
      const prices = hc.perpInfos[0].basePrices;
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
      let hcClone: HealthCache = deepClone<HealthCache>(hc);
      hcClone.perpInfos[0].baseLots.iadd(new BN(baseLots1));
      hcClone.perpInfos[0].quote.isub(baseNative.mul(tradePrice));
      const actualRatio = hcClone.healthRatio(HealthType.init);

      // the ratio for trading just one base lot extra
      const baseLots2 = direction * (baseLots0 + 1);
      baseNative = I80F48.fromNumber(baseLots2 * baseLotSize);
      hcClone = deepClone<HealthCache>(hc);
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
      expect(ratio).lessThanOrEqual(actualRatio);
      expect(plusRatio - 0.1).lessThanOrEqual(ratio);
    }

    // adjust token
    hc.tokenInfos[0].balanceSpot.iadd(I80F48.fromNumber(3000));
    for (const existingSettle of [-500, 0, 300]) {
      const hcClone1: HealthCache = deepClone<HealthCache>(hc);
      hcClone1.tokenInfos[1].balanceSpot.iadd(
        I80F48.fromNumber(existingSettle),
      );
      for (const existing of [-5, 0, 3]) {
        const hcClone2: HealthCache = deepClone<HealthCache>(hcClone1);
        hcClone2.perpInfos[0].baseLots.iadd(new BN(existing));
        hcClone2.perpInfos[0].quote.isub(
          I80F48.fromNumber(existing * baseLotSize * 2),
        );
        for (const side of [PerpOrderSide.bid, PerpOrderSide.ask]) {
          console.log(
            `lots ${existing}, settle ${existingSettle}, side ${
              side === PerpOrderSide.bid ? 'bid' : 'ask'
            }`,
          );
          for (const priceFactor of [0.8, 1.0, 1.1]) {
            for (const ratio of range(1, 101, 1)) {
              checkMaxTrade(hcClone2, side, ratio, priceFactor);
            }
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

  it('Modify copy of health object leaving original object unchanged', (done) => {
    const baseLotSize = 100;
    const b0 = mockBankAndOracle(0 as TokenIndex, 0.1, 0.1, 2, 2);
    const b1 = mockBankAndOracle(1 as TokenIndex, 0.2, 0.2, 3, 3);
    const b2 = mockBankAndOracle(2 as TokenIndex, 0.3, 0.3, 4, 4);
    const p0 = mockPerpMarket(0, 1, 0.3, 0.3, baseLotSize, 2);
    const hc = new HealthCache(
      [
        TokenInfo.fromBank(b0, I80F48.fromNumber(0)),
        TokenInfo.fromBank(b1, I80F48.fromNumber(0)),
        TokenInfo.fromBank(b2, I80F48.fromNumber(0)),
      ],
      [],
      [PerpInfo.emptyFromPerpMarket(p0)],
    );
    const clonedHc: HealthCache = deepClone(hc);
    // mess up props
    clonedHc.tokenInfos[0].balanceSpot.iadd(
      I80F48.fromNumber(100).div(clonedHc.tokenInfos[0].prices.oracle),
    );
    clonedHc.tokenInfos[1].balanceSpot.iadd(
      I80F48.fromNumber(-2).div(clonedHc.tokenInfos[1].prices.oracle),
    );
    clonedHc.tokenInfos[1].prices.oracle.iadd(new I80F48(new BN(333333)));
    expect(hc.tokenInfos[0].balanceSpot.eq(clonedHc.tokenInfos[0].balanceSpot))
      .to.be.false;
    expect(hc.tokenInfos[1].balanceSpot.eq(clonedHc.tokenInfos[1].balanceSpot))
      .to.be.false;
    expect(
      hc.tokenInfos[1].prices.oracle.eq(clonedHc.tokenInfos[1].prices.oracle),
    ).to.be.false;
    //one is unchanged
    expect(hc.tokenInfos[2].balanceSpot.eq(clonedHc.tokenInfos[2].balanceSpot))
      .to.be.true;

    //do something stupid
    clonedHc['addProp'] = '123';
    expect(hc['addProp'] === undefined && clonedHc['addProp'] === '123').to.be
      .true;

    clonedHc.adjustPerpInfo(
      0,
      new I80F48(new BN(100000)),
      PerpOrderSide.ask,
      new BN(100000),
    );
    //original object is unaffected by running method from inside the clonedObj
    expect(
      clonedHc.perpInfos[0].baseLots.eq(hc.perpInfos[0].baseLots) &&
        clonedHc.perpInfos[0].quote.eq(hc.perpInfos[0].quote),
    ).to.be.false;

    expect(
      clonedHc.healthRatio(HealthType.init).eq(hc.healthRatio(HealthType.init)),
    ).to.be.false;

    done();
  });

  // TODO: test_assets_and_borrows
});
