import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { AppProviders } from "./app/providers";
import { App } from "./app/App";
import { applyCachedThemeMarker } from "./plugins/appearance-preference";
import { installClientLogging } from "./shared/logging/client-logger";
import "./styles.css";

applyCachedThemeMarker();
installClientLogging();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppProviders>
      <App />
    </AppProviders>
  </StrictMode>,
);
