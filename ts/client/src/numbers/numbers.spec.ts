import { expect } from 'chai';
import { U64_MAX_BN } from '../utils';

describe('Math', () => {
  it('do not convert BN toNumber', () => {
    console.log(1e7);
    expect(U64_MAX_BN.toNumber());
  });
});
