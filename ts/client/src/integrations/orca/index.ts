// import {
//   OrcaPoolToken,
//   ORCA_TOKEN_SWAP_ID_DEVNET,
//   PoolTokenCount,
// } from '@orca-so/sdk';
// import { orcaDevnetPoolConfigs } from '@orca-so/sdk/dist/constants/devnet/pools';
// import { OrcaPoolParams } from '@orca-so/sdk/dist/model/orca/pool/pool-types';
// import { OrcaPoolConfig as OrcaDevnetPoolConfig } from '@orca-so/sdk/dist/public/devnet/pools/config';
// import { BN } from '@project-serum/anchor';
// import {
//   AccountInfo,
//   AccountLayout,
//   TOKEN_PROGRAM_ID,
//   u64,
// } from '@solana/spl-token';
// import { TokenSwap } from '@solana/spl-token-swap';
// import { Connection, PublicKey } from '@solana/web3.js';

// import { Bank } from '../../accounts/bank';
// import { toNativeDecimals, toUiDecimals } from '../../utils';
// import * as Tokens from './tokens';

// export { ORCA_TOKEN_SWAP_ID_DEVNET };

// /*
//   Orca ix references:
//     swap fn: https://github.com/orca-so/typescript-sdk/blob/main/src/model/orca/pool/orca-pool.ts#L162
//     swap ix: https://github.com/orca-so/typescript-sdk/blob/main/src/public/utils/web3/instructions/pool-instructions.ts#L41
// */
// export const buildOrcaInstruction = async (
//   orcaTokenSwapId: PublicKey,
//   inputBank: Bank,
//   outputBank: Bank,
//   amountInU64: BN,
//   minimumAmountOutU64: BN,
// ) => {
//   // TODO: select the correct pool params based on passed in banks
//   const poolParams = orcaDevnetPoolConfigs[OrcaDevnetPoolConfig.ORCA_SOL];

//   const [authorityForPoolAddress] = await PublicKey.findProgramAddress(
//     [poolParams.address.toBuffer()],
//     orcaTokenSwapId,
//   );

//   const instruction = TokenSwap.swapInstruction(
//     poolParams.address,
//     authorityForPoolAddress,
//     inputBank.publicKey, // userTransferAuthority
//     inputBank.vault, // inputTokenUserAddress
//     poolParams.tokens[inputBank.mint.toString()].addr, // inputToken.addr
//     poolParams.tokens[outputBank.mint.toString()].addr, // outputToken.addr
//     outputBank.vault, // outputTokenUserAddress
//     poolParams.poolTokenMint,
//     poolParams.feeAccount,
//     null, // hostFeeAccount
//     orcaTokenSwapId,
//     TOKEN_PROGRAM_ID,
//     amountInU64,
//     minimumAmountOutU64,
//   );

//   instruction.keys[2].isSigner = false;
//   instruction.keys[2].isWritable = true;

//   return { instruction, signers: [] };
// };

// export const getOrcaOutputAmount = async (
//   connection: Connection,
//   inputToken: string,
//   outputToken: string,
//   amountIn: number,
// ): Promise<number> => {
//   // TODO: select the correct pool params based on passed in banks
//   const inputMint = Tokens.solToken;
//   const poolParams = getOrcaPoolParams(inputToken, outputToken);

//   const { inputPoolToken, outputPoolToken } = getTokens(
//     poolParams,
//     inputMint.mint.toString(),
//   );

//   const { inputTokenCount, outputTokenCount } = await getTokenCount(
//     connection,
//     poolParams,
//     inputPoolToken,
//     outputPoolToken,
//   );

//   const [poolInputAmount, poolOutputAmount] = [
//     inputTokenCount,
//     outputTokenCount,
//   ];

//   const invariant = poolInputAmount.mul(poolOutputAmount);
//   const nativeAmountIn = toNativeDecimals(amountIn, 9);

//   const [newPoolOutputAmount] = ceilingDivision(
//     invariant,
//     poolInputAmount.add(nativeAmountIn),
//   );

//   const outputAmount = poolOutputAmount.sub(newPoolOutputAmount);

//   return toUiDecimals(outputAmount.toNumber(), 6);
// };

// function getTokens(poolParams: OrcaPoolParams, inputTokenId: string) {
//   if (poolParams.tokens[inputTokenId] == undefined) {
//     throw new Error('Input token not part of pool');
//   }

//   const tokenAId = poolParams.tokenIds[0];
//   const tokenBId = poolParams.tokenIds[1];

//   const forward = tokenAId == inputTokenId;

//   const inputOrcaToken = forward
//     ? poolParams.tokens[tokenAId]
//     : poolParams.tokens[tokenBId];
//   const outputOrcaToken = forward
//     ? poolParams.tokens[tokenBId]
//     : poolParams.tokens[tokenAId];
//   return { inputPoolToken: inputOrcaToken, outputPoolToken: outputOrcaToken };
// }

// const getOrcaPoolParams = (inputToken: string, outputToken: string) => {
//   return orcaDevnetPoolConfigs[OrcaDevnetPoolConfig.ORCA_SOL];
// };

// async function getTokenCount(
//   connection: Connection,
//   poolParams: OrcaPoolParams,
//   inputPoolToken: OrcaPoolToken,
//   outputPoolToken: OrcaPoolToken,
// ): Promise<PoolTokenCount> {
//   if (poolParams.tokens[inputPoolToken.mint.toString()] == undefined) {
//     throw new Error('Input token not part of pool');
//   }

//   if (poolParams.tokens[outputPoolToken.mint.toString()] == undefined) {
//     throw new Error('Output token not part of pool');
//   }

//   const accountInfos = await connection.getMultipleAccountsInfo([
//     inputPoolToken.addr,
//     outputPoolToken.addr,
//   ]);

//   const tokens = accountInfos.map((info) =>
//     info != undefined ? deserializeAccount(info.data) : undefined,
//   );
//   const inputTokenAccount = tokens[0],
//     outputTokenAccount = tokens[1];

//   if (inputTokenAccount === undefined || outputTokenAccount === undefined) {
//     throw new Error('Unable to fetch accounts for specified tokens.');
//   }

//   return {
//     inputTokenCount: inputTokenAccount.amount,
//     outputTokenCount: outputTokenAccount.amount,
//   };
// }

// const deserializeAccount = (
//   data: Buffer | undefined,
// ): AccountInfo | undefined => {
//   if (data == undefined || data.length == 0) {
//     return undefined;
//   }

//   const accountInfo = AccountLayout.decode(data);
//   accountInfo.mint = new PublicKey(accountInfo.mint);
//   accountInfo.owner = new PublicKey(accountInfo.owner);
//   accountInfo.amount = u64.fromBuffer(accountInfo.amount);

//   return accountInfo;
// };

// const ZERO = new BN(0);
// const ONE = new BN(1);
// const ceilingDivision = (dividend: u64, divisor: u64): [u64, u64] => {
//   let quotient = dividend.div(divisor);
//   if (quotient.eq(ZERO)) {
//     return [ZERO, divisor];
//   }

//   let remainder = dividend.mod(divisor);
//   if (remainder.gt(ZERO)) {
//     quotient = quotient.add(ONE);
//     divisor = dividend.div(quotient);
//     remainder = dividend.mod(quotient);
//     if (remainder.gt(ZERO)) {
//       divisor = divisor.add(ONE);
//     }
//   }

//   return [quotient, divisor];
// };
