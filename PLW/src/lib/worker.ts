let ws: WebSocket | undefined = undefined;
let last_addr: string | undefined = undefined;

onmessage = (event) => {
  switch (event.data.type) {
    case "NEW":
      createWs(event.data.payload);
      break;
    case "MSG":
      // Send JSON command to server
      tryResetWs();
      if (ws != undefined && ws.readyState === WebSocket.OPEN) {
        let msg = JSON.stringify(event.data.payload);
        ws.send(msg);
      }
    case "RST":
      tryResetWs();
  }
};

function tryResetWs() {
  if (
    (ws === undefined || ws.readyState != WebSocket.OPEN) &&
    last_addr != undefined
  ) {
    console.log("RS");
    createWs(last_addr);
  }
}

function handleMessage(event: MessageEvent) {
  if (event.data instanceof ArrayBuffer) {
    // ignore tsc during runtime an overloaded postMessage method is called
    // @ts-ignore: No overload matches this call
    postMessage({ type: "bin", payload: event.data }, [event.data]);
  } else if (event.data.indexOf("{") === 0) {
    handleJSON(JSON.parse(event.data));
  }
}

function handleJSON(wsCmd: any) {
  // check for server working directories
  if (wsCmd.dirs != undefined && wsCmd.vps != undefined) {
    let obj = { dirs: wsCmd.dirs, vps: wsCmd.vps };
    self.postMessage({ type: "config", payload: obj });
    return;
  }

  // check for layout
  if (
    wsCmd.modules != undefined &&
    wsCmd.start_addrs != undefined &&
    wsCmd.end_addrs != undefined
  ) {
    let obj = {
      modules: wsCmd.modules,
      start_addrs: wsCmd.start_addrs,
      end_addrs: wsCmd.end_addrs,
    };
    self.postMessage({ type: "layout", payload: obj });
    return;
  }

  // check status message
  if (wsCmd.command === "Status") {
    let hasVP = 0;
    let hasGDB = 0;
    if (wsCmd.value === "true") {
      hasVP = 1;
    } else if (wsCmd.value.indexOf("Connected") > -1) {
      hasGDB = 1;
    } else if (wsCmd.value === "false") {
      hasVP = -1;
    } else if (wsCmd.value.indexOf("NotConnected") > -1) {
      hasGDB = -1;
    }
    let obj = { vp: hasVP, gdb: hasGDB };
    self.postMessage({ type: "status", payload: obj });
    return;
  }

  // check VP start
  if (wsCmd.command === "Start") {
    self.postMessage({ type: "start", payload: wsCmd.value });
    return;
  }

  if (wsCmd.command === "Options") {
    self.postMessage({ type: "options", payload: wsCmd.value });
  }
}

function createWs(addr: string) {
  if (ws != undefined && ws.readyState === WebSocket.OPEN) {
    return;
  }

  ws = new WebSocket("ws://" + addr + "/ws");
  ws.binaryType = "arraybuffer";

  ws.addEventListener("message", handleMessage);
  ws.addEventListener("open", () => {
    self.postMessage({ type: "open", payload: addr });
    last_addr = addr;
  });
  ws.addEventListener("close", () => {
    self.postMessage({ type: "close" });
  });
  ws.addEventListener("error", () => {
    self.postMessage({ type: "error" });
  });
}
