import { render } from "preact";
import "../style.css";
import { AppShell } from "./components/shell";
import { ConsoleErrorBoundary } from "./components/error-boundary";
import { startController } from "./controller";

const root = document.getElementById("app");
if (root === null) throw new Error("Management Center root is missing");
render(<ConsoleErrorBoundary><AppShell /></ConsoleErrorBoundary>, root);
startController();
