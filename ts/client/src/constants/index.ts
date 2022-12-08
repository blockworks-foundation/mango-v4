import { PublicKey } from '@solana/web3.js';

export const OPENBOOK_PROGRAM_ID = {
  devnet: new PublicKey('EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj'),
  'mainnet-beta': new PublicKey('srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX'),
};

export const MANGO_V4_ID = {
  testnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  devnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  'mainnet-beta': new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
};
