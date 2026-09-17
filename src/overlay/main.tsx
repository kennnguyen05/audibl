import React from "react";
import ReactDOM from "react-dom/client";
import { initLanguage } from "@/i18n";

initLanguage();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <div />
  </React.StrictMode>,
);
