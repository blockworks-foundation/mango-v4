import Big from 'big.js';
import BN from 'bn.js';
export class I80F48Dto {
    val;
    constructor(val) {
        this.val = val;
    }
}
// TODO - this whole class is inefficient; consider optimizing
export class I80F48 {
    /**
    This is represented by a 128 bit signed integer underneath
    The first 80 bits are treated as an integer and last 48 bits are treated as fractional part after binary point
    It's possible to think of an I80F48 as an i128 divided by 2 ^ 40
  
    Read up on how fixed point math works: https://inst.eecs.berkeley.edu/~cs61c/sp06/handout/fixedpt.html
    Read up on how 2s complement works: https://en.wikipedia.org/wiki/Two%27s_complement
     */
    static MAX_SIZE = 128;
    static FRACTIONS = 48;
    static MULTIPLIER_BIG = new Big(2).pow(I80F48.FRACTIONS);
    static MULTIPLIER_BN = new BN(2).pow(new BN(I80F48.FRACTIONS));
    static MULTIPLIER_NUMBER = Math.pow(2, I80F48.FRACTIONS);
    static MAX_BN = new BN(2)
        .pow(new BN(I80F48.MAX_SIZE))
        .div(new BN(2))
        .sub(new BN(1));
    static MIN_BN = new BN(2)
        .pow(new BN(I80F48.MAX_SIZE))
        .div(new BN(2))
        .neg();
    data; // This is i128 => array of 16 bytes
    static from(dto) {
        return new I80F48(dto.val);
    }
    constructor(data) {
        if (data.lt(I80F48.MIN_BN) || data.gt(I80F48.MAX_BN)) {
            throw new Error('Number out of range');
        }
        this.data = data;
    }
    static fromNumber(x) {
        const int_part = Math.trunc(x);
        const v = new BN(int_part.toFixed(0)).iushln(48);
        v.iadd(new BN((x - int_part) * I80F48.MULTIPLIER_NUMBER));
        return new I80F48(v);
    }
    static fromNumberOrUndef(x) {
        return x === undefined ? undefined : I80F48.fromNumber(x);
    }
    static fromOptionalString(x) {
        return x ? I80F48.fromString(x) : undefined;
    }
    static fromString(x) {
        const initialValue = new Big(x).times(I80F48.MULTIPLIER_BIG);
        const fixedPointValue = new BN(initialValue.round().toFixed());
        return new I80F48(fixedPointValue);
    }
    static fromI64(x) {
        return new I80F48(x.ushln(48));
    }
    static fromU64(x) {
        return new I80F48(x.ushln(48));
    }
    toTwos() {
        return this.data.toTwos(I80F48.MAX_SIZE);
    }
    toString() {
        return this.toBig().toFixed();
    }
    /**
     * The number will be rounded first for UI sensibilities, then toFixed
     */
    toFixed(decimals) {
        return this.toBig().round(14).toFixed(decimals);
    }
    toLocaleString(locales, options) {
        return this.toNumber().toLocaleString(locales, options);
    }
    toBig() {
        return new Big(this.data.toString()).div(I80F48.MULTIPLIER_BIG);
    }
    static fromBig(x) {
        return new I80F48(new BN(x.mul(I80F48.MULTIPLIER_BIG).round().toFixed()));
    }
    toNumber() {
        return this.toBig().toNumber();
    }
    static fromArray(src) {
        if (src.length !== 16) {
            throw new Error('Uint8Array must be of length 16');
        }
        return new I80F48(new BN(src, 'le').fromTwos(I80F48.MAX_SIZE));
    }
    toArray() {
        return new Uint8Array(this.data.toTwos(I80F48.MAX_SIZE).toArray('le', 16));
    }
    toArrayLike(ArrayType, endian, length) {
        return this.data
            .toTwos(I80F48.MAX_SIZE)
            .toArrayLike(ArrayType, endian, length);
    }
    getData() {
        return this.data;
    }
    getBinaryLayout() {
        return this.data
            .toTwos(I80F48.MAX_SIZE)
            .toString(2, I80F48.MAX_SIZE)
            .replace(/-/g, '');
    }
    add(x) {
        return new I80F48(this.data.add(x.getData()));
    }
    sub(x) {
        return new I80F48(this.data.sub(x.getData()));
    }
    iadd(x) {
        this.data.iadd(x.getData());
        return this;
    }
    isub(x) {
        this.data.isub(x.getData());
        return this;
    }
    floor() {
        // Low IQ method
        return I80F48.fromBig(this.toBig().round(undefined, 0));
        // return new I80F48(this.data.shrn(I80F48.FRACTIONS).shln(I80F48.FRACTIONS));
    }
    ceil() {
        // Low IQ method, 3 -> round up
        return I80F48.fromBig(this.toBig().round(undefined, 3));
        // const frac = this.data.maskn(I80F48.FRACTIONS);
        // if (frac.eq(ZERO_BN)) {
        //   return this;
        // } else {
        //   return this.floor().add(ONE_I80F48);
        // }
    }
    frac() {
        // TODO verify this works for negative numbers
        return new I80F48(this.data.maskn(I80F48.FRACTIONS));
    }
    /**
     * Multiply the two and shift
     */
    mul(x) {
        return new I80F48(this.data.mul(x.data).iushrn(I80F48.FRACTIONS));
    }
    imul(x) {
        this.data.imul(x.getData()).iushrn(I80F48.FRACTIONS);
        return this;
    }
    div(x) {
        return new I80F48(this.data.ushln(I80F48.FRACTIONS).div(x.data));
    }
    idiv(x) {
        this.data = this.data.iushln(I80F48.FRACTIONS).div(x.data);
        return this;
    }
    gt(x) {
        return this.data.gt(x.getData());
    }
    lt(x) {
        return this.data.lt(x.getData());
    }
    gte(x) {
        return this.data.gte(x.getData());
    }
    lte(x) {
        return this.data.lte(x.getData());
    }
    eq(x) {
        // TODO make sure this works when they're diff signs or 0
        return this.data.eq(x.getData());
    }
    cmp(x) {
        // TODO make sure this works when they're diff signs or 0
        return this.data.cmp(x.getData());
    }
    neg() {
        return this.mul(_NEG_ONE_I80F48);
    }
    isPos() {
        return this.gt(_ZERO_I80F48);
    }
    isNeg() {
        return this.data.isNeg();
    }
    isZero() {
        return this.eq(_ZERO_I80F48);
    }
    min(x) {
        return this.lte(x) ? this : x;
    }
    max(x) {
        return this.gte(x) ? this : x;
    }
    abs() {
        if (this.isNeg()) {
            return this.neg();
        }
        else {
            return this;
        }
    }
}
/** @internal */
const _ZERO_I80F48 = I80F48.fromNumber(0);
/** @internal */
const _NEG_ONE_I80F48 = I80F48.fromNumber(-1);
export function ONE_I80F48() {
    return I80F48.fromNumber(1);
}
export function ZERO_I80F48() {
    return I80F48.fromNumber(0);
}
export function HUNDRED_I80F48() {
    return I80F48.fromNumber(100);
}
export function MAX_I80F48() {
    return new I80F48(I80F48.MAX_BN);
}
