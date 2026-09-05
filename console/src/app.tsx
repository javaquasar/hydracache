import { render } from "preact";
import "../style.css";
import { AppShell } from "./components/shell";
import { startController } from "./controller";

const root = document.getElementById("app");
if (root === null) throw new Error("Management Center root is missing");
render(<AppShell />, root);
startController();
