import { expect } from 'chai';
import { toUiDecimalsForQuote } from '../utils';
import { BankForHealth } from './bank';
import { HealthCache, TokenInfo } from './healthCache';
import { I80F48, ZERO_I80F48 } from './I80F48';

describe('Health Cache', () => {
  it('max swap tokens for min ratio', () => {
    // USDC like
    const sourceBank: BankForHealth = {
      tokenIndex: 0,
      maintAssetWeight: I80F48.fromNumber(1),
      initAssetWeight: I80F48.fromNumber(1),
      maintLiabWeight: I80F48.fromNumber(1),
      initLiabWeight: I80F48.fromNumber(1),
      price: I80F48.fromNumber(1),
    };
    // BTC like
    const targetBank: BankForHealth = {
      tokenIndex: 1,
      maintAssetWeight: I80F48.fromNumber(0.9),
      initAssetWeight: I80F48.fromNumber(0.8),
      maintLiabWeight: I80F48.fromNumber(1.1),
      initLiabWeight: I80F48.fromNumber(1.2),
      price: I80F48.fromNumber(20000),
    };

    const hc = new HealthCache(
      [
        new TokenInfo(
          0,
          sourceBank.maintAssetWeight,
          sourceBank.initAssetWeight,
          sourceBank.maintLiabWeight,
          sourceBank.initLiabWeight,
          sourceBank.price!,
          I80F48.fromNumber(-18 * Math.pow(10, 6)),
          ZERO_I80F48(),
        ),

        new TokenInfo(
          1,
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
