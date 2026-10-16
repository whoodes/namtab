"use strict";
const { contextBridge, ipcRenderer } = require("electron");
contextBridge.exposeInMainWorld(
  "mk1Desktop",
  Object.freeze({
    dispatch(command) {
      return ipcRenderer.invoke("mk1:command", command);
    },
    subscribe(listener) {
      if (typeof listener !== "function")
        throw new TypeError("State listener required");
      const handler = (_event, state) => listener(state);
      ipcRenderer.on("mk1:state", handler);
      return () => ipcRenderer.removeListener("mk1:state", handler);
    },
  }),
);
