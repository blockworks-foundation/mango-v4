import { BN } from '@project-serum/anchor';
import { OpenOrders } from '@project-serum/serum';
import { expect } from 'chai';
import { I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { toUiDecimalsForQuote } from '../utils';
import { BankForHealth, TokenIndex } from './bank';
import { HealthCache, PerpInfo, Serum3Info, TokenInfo } from './healthCache';
import { HealthType, PerpPosition } from './mangoAccount';
import { PerpMarket } from './perp';
import { MarketIndex } from './serum3';

function mockBankAndOracle(
  tokenIndex: TokenIndex,
  maintWeight: number,
  initWeight: number,
  price: number,
): BankForHealth {
  return {
    tokenIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price: I80F48.fromNumber(price),
  };
}

function mockPerpMarket(
  perpMarketIndex: number,
  maintWeight: number,
  initWeight: number,
  price: I80F48,
): PerpMarket {
  return {
    perpMarketIndex,
    maintAssetWeight: I80F48.fromNumber(1 - maintWeight),
    initAssetWeight: I80F48.fromNumber(1 - initWeight),
    maintLiabWeight: I80F48.fromNumber(1 + maintWeight),
    initLiabWeight: I80F48.fromNumber(1 + initWeight),
    price,
    quoteLotSize: new BN(100),
    baseLotSize: new BN(10),
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
    );
    const targetBank: BankForHealth = mockBankAndOracle(
      4 as TokenIndex,
      0.3,
      0.5,
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

    const pM = mockPerpMarket(9, 0.1, 0.2, targetBank.price);
    const pp = new PerpPosition(
      pM.perpMarketIndex,
      new BN(3),
      I80F48.fromNumber(-310),
      new BN(7),
      new BN(11),
      new BN(1),
      new BN(2),
      I80F48.fromNumber(0),
      I80F48.fromNumber(0),
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
      `health ${health
        .toFixed(3)
        .padStart(
          10,
        )}, case "test that includes all the side values (like referrer_rebates_accrued)"`,
    );

    expect(health - (health1 + health2 + health3)).lessThan(0.0000001);
  });

  it('test_health1', () => {
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
      );
      const bank2: BankForHealth = mockBankAndOracle(
        4 as TokenIndex,
        0.3,
        0.5,
        5,
      );
      const bank3: BankForHealth = mockBankAndOracle(
        5 as TokenIndex,
        0.3,
        0.5,
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

      const pM = mockPerpMarket(9, 0.1, 0.2, bank2.price);
      const pp = new PerpPosition(
        pM.perpMarketIndex,
        new BN(fixture.perp1[0]),
        I80F48.fromNumber(fixture.perp1[1]),
        new BN(fixture.perp1[2]),
        new BN(fixture.perp1[3]),
        new BN(0),
        new BN(0),
        I80F48.fromNumber(0),
        I80F48.fromNumber(0),
      );
      const pi1 = PerpInfo.fromPerpPosition(pM, pp);

      const hc = new HealthCache([ti1, ti2, ti3], [si1, si2], [pi1]);
      const health = hc.health(HealthType.init).toNumber();
      console.log(
        `health ${health.toFixed(3).padStart(10)}, case "${fixture.name}"`,
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
  });

  it('max swap tokens for min ratio', () => {
    // USDC like
    const sourceBank: BankForHealth = {
      tokenIndex: 0 as TokenIndex,
      maintAssetWeight: I80F48.fromNumber(1),
      initAssetWeight: I80F48.fromNumber(1),
      maintLiabWeight: I80F48.fromNumber(1),
      initLiabWeight: I80F48.fromNumber(1),
      price: I80F48.fromNumber(1),
    };
    // BTC like
    const targetBank: BankForHealth = {
      tokenIndex: 1 as TokenIndex,
      maintAssetWeight: I80F48.fromNumber(0.9),
      initAssetWeight: I80F48.fromNumber(0.8),
      maintLiabWeight: I80F48.fromNumber(1.1),
      initLiabWeight: I80F48.fromNumber(1.2),
      price: I80F48.fromNumber(20000),
    };

    const hc = new HealthCache(
      [
        new TokenInfo(
          0 as TokenIndex,
          sourceBank.maintAssetWeight,
          sourceBank.initAssetWeight,
          sourceBank.maintLiabWeight,
          sourceBank.initLiabWeight,
          sourceBank.price!,
          I80F48.fromNumber(-18 * Math.pow(10, 6)),
          ZERO_I80F48(),
        ),

        new TokenInfo(
          1 as TokenIndex,
          targetBank.maintAssetWeight,
          targetBank.initAssetWeight,
          targetBank.maintLiabWeight,
          targetBank.initLiabWeight,
          targetBank.price!,
          I80F48.fromNumber(51 * Math.pow(10, 6)),
          ZERO_I80F48(),
        ),
      ],
      [],
      [],
    );

    expect(
      toUiDecimalsForQuote(
        hc.getMaxSourceForTokenSwap(
          targetBank,
          sourceBank,
          I80F48.fromNumber(1),
          I80F48.fromNumber(0.95),
        ),
      ).toFixed(3),
    ).equals('0.008');

    expect(
      toUiDecimalsForQuote(
        hc.getMaxSourceForTokenSwap(
          sourceBank,
          targetBank,
          I80F48.fromNumber(1),
          I80F48.fromNumber(0.95),
        ),
      ).toFixed(3),
    ).equals('90.477');
  });
});
