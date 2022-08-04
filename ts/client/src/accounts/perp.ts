import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { QUOTE_DECIMALS } from './bank';
import { I80F48, I80F48Dto } from './I80F48';

export class PerpMarket {
  public name: string;
  public maintAssetWeight: I80F48;
  public initAssetWeight: I80F48;
  public maintLiabWeight: I80F48;
  public initLiabWeight: I80F48;
  public liquidationFee: I80F48;
  public makerFee: I80F48;
  public takerFee: I80F48;
  public openInterest: number;
  public seqNum: number;
  public feesAccrued: I80F48;

  static from(
    publicKey: PublicKey,
    obj: {
      name: number[];
      group: PublicKey;
      oracle: PublicKey;
      bids: PublicKey;
      asks: PublicKey;
      eventQueue: PublicKey;
      quoteLotSize: BN;
      baseLotSize: BN;
      maintAssetWeight: I80F48Dto;
      initAssetWeight: I80F48Dto;
      maintLiabWeight: I80F48Dto;
      initLiabWeight: I80F48Dto;
      liquidationFee: I80F48Dto;
      makerFee: I80F48Dto;
      takerFee: I80F48Dto;
      openInterest: BN;
      seqNum: any; // TODO: ts complains that this is unknown for whatever reason
      feesAccrued: I80F48Dto;
      bump: number;
      baseTokenDecimals: number;
      perpMarketIndex: number;
      baseTokenIndex: number;
      quoteTokenIndex: number;
      registrationTime: BN;
    },
  ): PerpMarket {
    return new PerpMarket(
      publicKey,
      obj.name,
      obj.group,
      obj.oracle,
      obj.bids,
      obj.asks,
      obj.eventQueue,
      obj.quoteLotSize,
      obj.baseLotSize,
      obj.maintAssetWeight,
      obj.initAssetWeight,
      obj.maintLiabWeight,
      obj.initLiabWeight,
      obj.liquidationFee,
      obj.makerFee,
      obj.takerFee,
      obj.openInterest,
      obj.seqNum,
      obj.feesAccrued,
      obj.bump,
      obj.baseTokenDecimals,
      obj.perpMarketIndex,
      obj.baseTokenIndex,
      obj.quoteTokenIndex,
      obj.registrationTime,
    );
  }

  constructor(
    public publicKey: PublicKey,
    name: number[],
    public group: PublicKey,
    public oracle: PublicKey,
    public bids: PublicKey,
    public asks: PublicKey,
    public eventQueue: PublicKey,
    public quoteLotSize: BN,
    public baseLotSize: BN,
    maintAssetWeight: I80F48Dto,
    initAssetWeight: I80F48Dto,
    maintLiabWeight: I80F48Dto,
    initLiabWeight: I80F48Dto,
    liquidationFee: I80F48Dto,
    makerFee: I80F48Dto,
    takerFee: I80F48Dto,
    openInterest: BN,
    seqNum: BN,
    feesAccrued: I80F48Dto,
    bump: number,
    public baseTokenDecimals: number,
    public perpMarketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
    public registrationTime: BN,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.maintAssetWeight = I80F48.from(maintAssetWeight);
    this.initAssetWeight = I80F48.from(initAssetWeight);
    this.maintLiabWeight = I80F48.from(maintLiabWeight);
    this.initLiabWeight = I80F48.from(initLiabWeight);
    this.liquidationFee = I80F48.from(liquidationFee);
    this.makerFee = I80F48.from(makerFee);
    this.takerFee = I80F48.from(takerFee);
    this.openInterest = openInterest.toNumber();
    this.seqNum = seqNum.toNumber();
    this.feesAccrued = I80F48.from(feesAccrued);
  }

  uiToNativePriceQuantity(price: number, quantity: number): [BN, BN] {
    const baseUnit = Math.pow(10, this.baseTokenDecimals);
    const quoteUnit = Math.pow(10, QUOTE_DECIMALS);
    const nativePrice = new BN(price * quoteUnit)
      .mul(this.baseLotSize)
      .div(this.quoteLotSize.mul(new BN(baseUnit)));
    const nativeQuantity = new BN(quantity * baseUnit).div(this.baseLotSize);
    return [nativePrice, nativeQuantity];
  }

  uiQuoteToLots(uiQuote: number): BN {
    const quoteUnit = Math.pow(10, QUOTE_DECIMALS);
    return new BN(uiQuote * quoteUnit).div(this.quoteLotSize);
  }

  toString(): string {
    return (
      'PerpMarket ' +
      '\n perpMarketIndex -' +
      this.perpMarketIndex +
      '\n maintAssetWeight -' +
      this.maintAssetWeight.toNumber() +
      '\n initAssetWeight -' +
      this.initAssetWeight.toNumber() +
      '\n maintLiabWeight -' +
      this.maintLiabWeight.toNumber() +
      '\n initLiabWeight -' +
      this.initLiabWeight.toNumber() +
      '\n liquidationFee -' +
      this.liquidationFee.toNumber() +
      '\n makerFee -' +
      this.makerFee.toNumber() +
      '\n takerFee -' +
      this.takerFee.toNumber()
    );
  }
}

export class Side {
  static bid = { bid: {} };
  static ask = { ask: {} };
}

export class OrderType {
  static limit = { limit: {} };
  static immediateOrCancel = { immediateorcancel: {} };
  static postOnly = { postonly: {} };
  static market = { market: {} };
  static postOnlySlide = { postonlyslide: {} };
}
