import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { expect } from 'chai';
import { parseProgramLogs } from './program-logs';
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
    const res1 = parseProgramLogs(txLogs);
    // console.log(res1);

    expect(ret[0].toString()).equals('4627218176937451340867441');
    expect(ret[1].toString()).equals('4644871711015991392950415');

    const nestedTxLogs = [
      'Program ComputeBudget111111111111111111111111111111 invoke [1]',
      'Program ComputeBudget111111111111111111111111111111 success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: FlashLoanBegin',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 989071 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 28574 of 1000000 compute units',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
      'Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL invoke [1]',
      'Program log: CreateIdempotent',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: GetAccountDataSize',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1622 of 966021 compute units',
      'Program return: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA pQAAAAAAAAA=',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program 11111111111111111111111111111111 invoke [2]',
      'Program 11111111111111111111111111111111 success',
      'Program log: Initialize the associated token account',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: InitializeImmutableOwner',
      'Program log: Please upgrade to SPL Token 2022 for immutable owner support',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1405 of 959531 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: InitializeAccount3',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4241 of 955649 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL consumed 20301 of 971426 compute units',
      'Program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL success',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]',
      'Program log: Instruction: Swap',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 917494 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 909936 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc consumed 49616 of 951125 compute units',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc success',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]',
      'Program log: Instruction: Swap',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 860586 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 853032 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc consumed 56904 of 901509 compute units',
      'Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc success',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg invoke [1]',
      'Program log: Instruction: FlashLoanEnd',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]',
      'Program log: Instruction: Transfer',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 827419 compute units',
      'Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success',
      'Program log: pre_init_health: 618684342.155799507737285',
      'Program data: F5qwwQsqqPRbF8fIam5zn68XUYGDY+lPkIvzcATObT+88Ze90vUfHEmZ3/W5eWM4IT0NhCC+gPzUAqilakGhRtVC4kARsYtJAADkKn1ThTfSAAAAAAAAAAAAWm72Cnid+UAPAAAAAAAAAIHZUOPW0QNXDwAAAAAAAAA=',
      'Program data: F5qwwQsqqPRbF8fIam5zn68XUYGDY+lPkIvzcATObT+88Ze90vUfHEmZ3/W5eWM4IT0NhCC+gPzUAqilakGhRtVC4kARsYtJAQBQqdgSH5IVAAAAAAAAAAAArASF6CzOy0MPAAAAAAAAAFdRFkhMRkJJDwAAAAAAAAA=',
      'Program data: HZ5i63hl5i5bF8fIam5zn68XUYGDY+lPkIvzcATObT+88Ze90vUfHEmZ3/W5eWM4IT0NhCC+gPzUAqilakGhRtVC4kARsYtJAgAAAAAAAAAAAAAAwL3w/////////wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWm72Cnid+UAPAAAAAAAAAIHZUOPW0QNXDwAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAQAAAAAAAAB4NQ8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACsBIXoLM7LQw8AAAAAAAAAV1EWSExGQkkPAAAAAAAAAPpSUX7DAAEAAAAAAAAAAAAB',
      'Program log: post_init_health: 618584054.026412020465358',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg consumed 101131 of 844605 compute units',
      'Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg success',
    ];

    /// TODO
    const res = parseProgramLogs(nestedTxLogs);
    console.log(res[1]);
    console.log(res[2]);
  });
});
