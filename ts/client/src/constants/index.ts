import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';

export const RUST_U64_MAX = (): BN => {
  return new BN('18446744073709551615');
};
export const RUST_I64_MAX = (): BN => {
  return new BN('9223372036854775807');
};
export const RUST_I64_MIN = (): BN => {
  return new BN('-9223372036854775807');
};

export const OPENBOOK_PROGRAM_ID = {
  devnet: new PublicKey('EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj'),
  'mainnet-beta': new PublicKey('srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX'),
};

export const MANGO_V4_ID = {
  testnet: new PublicKey('4MTevvuuqC2sZtckP6RPeLZcqV3JyqoF3N5fswzc3NmT'),
  devnet: new PublicKey('4MTevvuuqC2sZtckP6RPeLZcqV3JyqoF3N5fswzc3NmT'),
  'mainnet-beta': new PublicKey('4MTevvuuqC2sZtckP6RPeLZcqV3JyqoF3N5fswzc3NmT'),
};

export const USDC_MINT = new PublicKey(
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
);
export const MAX_RECENT_PRIORITY_FEE_ACCOUNTS = 128;
