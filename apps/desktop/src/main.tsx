import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { tauriClient } from "./tauri";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App client={tauriClient} />
  </React.StrictMode>,
);
