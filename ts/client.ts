import { Program, Provider } from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { IDL, MangoV4 } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

  static async connect(
    provider: Provider,
    devnet?: boolean,
  ): Promise<MangoClient> {
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    // TODO: remove
    // Temporarily patch missing types, so we can do new Program(...) below.
    MangoClient.addDummyType(idl, 'usize');
    MangoClient.addDummyType(idl, 'AnyNode');
    MangoClient.addDummyType(idl, 'EventQueueHeader');
    MangoClient.addDummyType(idl, 'AnyEvent');
    MangoClient.addDummyType(idl, 'instructions::NewOrderInstructionData');
    MangoClient.addDummyType(idl, 'instructions::CancelOrderInstructionData');
    MangoClient.addDummyType(idl, 'H');
    MangoClient.addDummyType(idl, 'H::Item');
    MangoClient.addDummyType(idl, 'NodeHandle');

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, MANGO_V4_ID, provider),
      devnet,
    );
  }

  private static addDummyType(idl: MangoV4, typeName: string) {
    (idl.types as any).push({
      name: typeName,
      type: {
        kind: 'struct',
        fields: [
          {
            name: 'val',
            type: 'u64',
          },
        ],
      },
    });
  }
}
