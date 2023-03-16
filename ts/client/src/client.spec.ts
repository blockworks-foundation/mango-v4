import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { expect } from 'chai';
import { extractReturnValuesForSolanaTxLogs } from './utils';

describe('Solana', () => {
  it('Parse logs for a tx', () => {
    const txLogs = [
      'Program GDDMwNyyx8uB6zrqwBFHjLLG3TBYk2F8Az4yrQC5RzMp invoke [1]',
      'Program log: Sequence in order | sequence_num=1678963799926 | last_known=1678963798389',
      'Program GDDMwNyyx8uB6zrqwBFHjLLG3TBYk2F8Az4yrQC5RzMp consumed 3504 of 1200000 compute units',
      'Program GDDMwNyyx8uB6zrqwBFHjLLG3TBYk2F8Az4yrQC5RzMp success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: HealthRegionBegin',
      'Program log: pre_init_health: 368109920.181575993492142',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 49143 of 1196496 compute units',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: PerpCancelAllOrders',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 14159 of 1147353 compute units',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: PerpPlaceOrder',
      'Program data: vx89QqxinBZbF8fIam5zn68XUYGDY+lPkIvzcATObT+88Ze90vUfHAAA0MOjkawXApT3/////////9DDo5GsFwKU9/////////9gBhJSaUsAYgAAAAAAAAAAABSvbu7G/mEAAAAAAAAAAABQId06jcv1AAkAAAAAAAD4BwAAAAAAAPV2Tw/B//////////////8=',
      'Program log: bid on book order_id=4627218176937451340867441 quantity=553 price=250841',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 58306 of 1133194 compute units',
      'Program return: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg AXGnqv//////2dMDAAAAAAA=',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: PerpPlaceOrder',
      'Program log: ask on book order_id=4644871711015991392950415 quantity=553 price=251799',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 34403 of 1074888 compute units',
      'Program return: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg AY9YVQAAAAAAl9cDAAAAAAA=',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: HealthRegionEnd',
      'Program log: post_init_health: 368361099.946146068890076',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 39991 of 1040485 compute units',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
    ];

    const ret = extractReturnValuesForSolanaTxLogs<BN>(
      new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'),
      'perpPlaceOrder',
      txLogs,
    );
    expect(ret[0].toString()).equals('4627218176937451340867441');
    expect(ret[1].toString()).equals('4644871711015991392950415');
  });
});
