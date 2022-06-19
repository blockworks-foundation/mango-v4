import {Test, compute_health_wasm, Pubkey} from "mango-v4";

async function main() {
  console.log("hello");
  const test_data = new Test;
  test_data.key = new Pubkey("");
  test_data.owner = new Pubkey("");
  test_data.data = new Buffer("");
  compute_health_wasm(test_data);
  process.exit();
}

main();
