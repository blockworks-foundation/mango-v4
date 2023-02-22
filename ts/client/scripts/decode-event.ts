import { BN, BorshCoder } from '@coral-xyz/anchor';
import { IDL } from '../src/mango_v4';

async function main() {
  const coder = new BorshCoder(IDL);

  const event = coder.events.decode(process.argv[2]);
  console.log(
    JSON.stringify(
      event,
      function (key, value) {
        const orig_value = this[key]; // value is already processed
        if (orig_value instanceof BN) {
          return orig_value.toString();
        }
        return value;
      },
      '    ',
    ),
  );

  process.exit();
}

main();
