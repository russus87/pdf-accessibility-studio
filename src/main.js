import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { temaIniziale } from "./lib/tema.js";

temaIniziale();

const app = mount(App, { target: document.getElementById("app") });

export default app;
