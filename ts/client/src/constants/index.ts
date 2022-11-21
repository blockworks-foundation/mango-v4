import { PublicKey } from '@solana/web3.js';

export const SERUM3_PROGRAM_ID = {
  testnet: new PublicKey('DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY'),
  devnet: new PublicKey('DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY'),
  'mainnet-beta': new PublicKey('9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin'),
};

export const MANGO_V4_ID = {
  testnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  devnet: new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
  'mainnet-beta': new PublicKey('m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD'),
};
