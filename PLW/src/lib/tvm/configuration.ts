import type { Ref } from "vue";

export interface Configuration {
  vp: string;
  proj: string;
  args: string;
  flags: Ref<Array<Flag>>;
  quantum: number;
  arch: string;
}

export interface Flag {
  active: boolean;
  value: string;
  name: string;
}
