import { mount } from "svelte";

import App from "./App.svelte";
import "./styles.css";

const target = document.querySelector<HTMLElement>("#app");
if (!target) throw new Error("#app is missing from index.html");

mount(App, { target });