import { BN } from '@project-serum/anchor';
import { utf8 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { PublicKey } from '@solana/web3.js';
import { I80F48, I80F48Dto } from './I80F48';

export class PerpMarket {
  public name: string;
  public quoteLotSize: number;
  public baseLotSize: number;
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
      reserved: number[];
      perpMarketIndex: number;
      baseTokenIndex: number;
      quoteTokenIndex: number;
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
      obj.reserved,
      obj.perpMarketIndex,
      obj.baseTokenIndex,
      obj.quoteTokenIndex,
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
    quoteLotSize: BN,
    baseLotSize: BN,
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
    reserved: number[],
    public perpMarketIndex: number,
    public baseTokenIndex: number,
    public quoteTokenIndex: number,
  ) {
    this.name = utf8.decode(new Uint8Array(name)).split('\x00')[0];
    this.quoteLotSize = quoteLotSize.toNumber();
    this.baseLotSize = baseLotSize.toNumber();
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
