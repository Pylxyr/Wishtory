import '@fontsource-variable/google-sans/wght.css';
import '@fontsource-variable/jetbrains-mono/wght.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { initTheme } from './lib/state.svelte';

initTheme(); // before mount, so a light-theme user never sees a dark flash
mount(App, { target: document.getElementById('app')! });
