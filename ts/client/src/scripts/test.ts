import {WasmAccount, WasmAccounts, compute_health_wasm, Pubkey} from "mango-v4";

function main() {
  console.log("hello");

  let accounts = new WasmAccounts;

  for (let x = 0; x < 10; ++x) {
    const test_data = new WasmAccount;
    test_data.key = new Pubkey("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");
    test_data.owner = new Pubkey("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");
    test_data.data = new Buffer("fooobaaa");
    accounts.push(test_data);
  }

  const test_data = new WasmAccount;
  test_data.key = new Pubkey("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");
  test_data.owner = new Pubkey("m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD");
  test_data.data = new Buffer("fooobaaa");
  console.log(compute_health_wasm(test_data, accounts));
  process.exit();
}

main();
