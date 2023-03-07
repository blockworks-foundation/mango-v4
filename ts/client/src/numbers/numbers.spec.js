import BN from 'bn.js';
import { expect } from 'chai';
import { U64_MAX_BN } from '../utils';
import { I80F48 } from './I80F48';
describe('Math', () => {
    it('js number to BN and I80F48', () => {
        // BN can be only be created from js numbers which are <=2^53
        expect(function () {
            new BN(0x1fffffffffffff);
        }).to.not.throw('Assertion failed');
        expect(function () {
            new BN(0x20000000000000);
        }).to.throw('Assertion failed');
        // max BN cant be converted to a number
        expect(function () {
            U64_MAX_BN.toNumber();
        }).to.throw('Number can only safely store up to 53 bits');
        // max I80F48 can be converted to a number
        // though, the number is represented in scientific notation
        // anything above ^20 gets represented with scientific notation
        expect(I80F48.fromString('604462909807314587353087.999999999999996')
            .toNumber()
            .toString()).equals('6.044629098073146e+23');
        // I80F48 constructor takes a BN, but it doesnt do what one might think it does
        expect(new I80F48(new BN(10)).toNumber()).not.equals(10);
        expect(I80F48.fromI64(new BN(10)).toNumber()).equals(10);
        // BN treats input as whole integer
        expect(new BN(1.5).toNumber()).equals(1);
    });
});
