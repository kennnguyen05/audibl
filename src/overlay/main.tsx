import React from "react";
import ReactDOM from "react-dom/client";
import { initLanguage } from "@/i18n";
import { Overlay } from "./Overlay";
import "@/index.css";

initLanguage();

// The overlay window is transparent; only the pill is drawn.
document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Overlay />
  </React.StrictMode>,
);
