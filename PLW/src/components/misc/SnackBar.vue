<script setup lang="ts">
import { ref, inject, onMounted } from "vue";

const worker: Worker | undefined = inject("worker");
const showRef = ref("");
const colorRef = ref("is-primary");
const msgText = ref("");

const colors = [
  "is-primary",
  "is-info",
  "is-link",
  "is-success",
  "is-warning",
  "is-danger",
];
const showTime = 3000;
let currentTimeout: number = -1;

onMounted(() => {
  worker?.addEventListener("message", (msg) => {
    switch (msg.data.type) {
      case "open":
        show("Connected to " + msg.data.payload, 3);
        break;
      case "close":
        show("Server connection closed", 5);
        break;
      case "error":
        show("Server connection error", 5);
        break;
    }
  });
});

function show(text: string, type: number) {
  showRef.value = "show";
  msgText.value = text;
  if (type != undefined && type > 0 && type < colors.length) {
    colorRef.value = colors[type];
  }
  clearTimeout(currentTimeout);
  currentTimeout = setTimeout(() => {
    showRef.value = "";
    msgText.value = "";
  }, showTime);
}
</script>

<template>
  <div id="snackbar" class="notification" :class="[showRef, colorRef]">
    {{ msgText }}
  </div>
</template>

<style scoped>
#snackbar {
  width: fit-content;
  visibility: hidden;
  min-width: 250px;
  margin-left: -125px;
  text-align: center;
  border-radius: 2px;
  padding: 16px;
  position: fixed;
  z-index: 1;
  left: 50%;
  bottom: 30px;
}

#snackbar.show {
  visibility: visible;
  /* Add animation: Take 0.5 seconds to fade in and out the snackbar.
  However, delay the fade out process for 2.5 seconds */
  -webkit-animation:
    fadein 0.5s,
    fadeout 0.5s 2.5s;
  animation:
    fadein 0.5s,
    fadeout 0.5s 2.5s;
}

@-webkit-keyframes fadein {
  from {
    bottom: 0;
    opacity: 0;
  }

  to {
    bottom: 30px;
    opacity: 1;
  }
}

@keyframes fadein {
  from {
    bottom: 0;
    opacity: 0;
  }

  to {
    bottom: 30px;
    opacity: 1;
  }
}

@-webkit-keyframes fadeout {
  from {
    bottom: 30px;
    opacity: 1;
  }

  to {
    bottom: 0;
    opacity: 0;
  }
}

@keyframes fadeout {
  from {
    bottom: 30px;
    opacity: 1;
  }

  to {
    bottom: 0;
    opacity: 0;
  }
}
</style>
