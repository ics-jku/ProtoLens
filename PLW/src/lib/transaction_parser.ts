import type { Transaction } from "@/types";

const PKT_LENGTHB = 28;
const PREAMBLE = 8;
const one = BigInt(1);

function isTransactionPacket(byteLength: number) {
  return byteLength % PKT_LENGTHB === PREAMBLE;
}

export default function parseBinary(
  buffer: ArrayBuffer,
  modules: Array<string>,
): Array<Transaction> {
  let transactions = new Array<Transaction>();
  if (!isTransactionPacket(buffer.byteLength)) {
    console.log("Received an incorrect amount of bytes");
    return transactions;
  }

  let data = new DataView(buffer);
  // PREAMBLE contains transaction counter
  let t_count = data.getBigUint64(0, true);
  for (let i = PREAMBLE; i < data.byteLength; i += PKT_LENGTHB) {
    t_count += one;
    let trans: Transaction = {
      sim_time: data.getBigUint64(i, true),
      action: data.getUint8(i + 8),
      initiator: "Core-" + String.fromCharCode(data.getUint8(i + 9)),
      target: modules[data.getUint8(i + 10)],
      address: data.getBigUint64(i + 11, true),
      data_length: data.getUint8(i + 19),
      data: data.getBigUint64(i + 20, true),
      trans_cnt: t_count,
    };
    if (trans.target === undefined) {
      console.log("Transaction contains a unknown target");
      continue;
    }
    transactions.push(trans);
  }
  return transactions;
}
