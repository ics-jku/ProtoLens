import type { Ref } from "vue";

export interface Transaction {
  sim_time: bigint;
  action: number;
  initiator: string;
  target: string;
  address: bigint;
  data_length: number;
  data: bigint;
  trans_cnt: bigint;
}

export interface AppState {
  modules: Array<string>;
  start_addrs: Array<string>;
  end_addrs: Array<string>;
  workingDirs: Array<string>;
  workingVps: Array<string>;
  hasSocket: Ref<boolean>;
  hasVP: Ref<boolean>;
  hasGdb: Ref<boolean>;
}
