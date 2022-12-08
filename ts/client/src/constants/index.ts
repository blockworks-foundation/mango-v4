import { PublicKey } from '@solana/web3.js';

export const OPENBOOK_PROGRAM_ID = {
  devnet: new PublicKey('EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj'),
  'mainnet-beta': new PublicKey('srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX'),
};

export const MANGO_V4_ID = {
  testnet: new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'),
  devnet: new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'),
  'mainnet-beta': new PublicKey('4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg'),
};
