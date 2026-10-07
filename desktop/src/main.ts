import { mount } from 'svelte';

import './app.css';
import App from './App.svelte';
import { init } from './lib/store.svelte';

init().then(() => mount(App, { target: document.getElementById('app')! }));
