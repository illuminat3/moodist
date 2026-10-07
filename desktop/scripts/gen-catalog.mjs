// Generates src/lib/catalog.json from the web app's sound definitions.
// Icons are pre-rendered to SVG strings so the desktop UI ships no icon library.
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const webData = join(root, '..', 'src', 'data');

// Same order as src/data/sounds.ts, plus binaural which the web app exposes separately.
const categoryFiles = [
  'nature',
  'rain',
  'animals',
  'urban',
  'places',
  'transport',
  'things',
  'noise',
  'binaural',
];

const iconPacks = {};
function loadIcon(pack, name) {
  iconPacks[pack] ??= require(`react-icons/${pack}/index.js`);
  const Icon = iconPacks[pack][name];
  if (!Icon) throw new Error(`Missing icon ${pack}/${name}`);
  return renderToStaticMarkup(React.createElement(Icon))
    .replace(/ (height|width)="1em"/g, '')
    .replace(/ style="[^"]*"/g, '');
}

const icons = {};
function iconRef(pack, name) {
  const key = `${pack}/${name}`;
  icons[key] ??= loadIcon(pack, name);
  return key;
}

const categories = categoryFiles.map(file => {
  const source = readFileSync(join(webData, 'sounds', `${file}.tsx`), 'utf8');

  const imports = {};
  for (const match of source.matchAll(
    /import\s*\{([^}]+)\}\s*from\s*'react-icons\/([\w-]+)\/index'/g,
  )) {
    for (const name of match[1].split(',').map(s => s.trim()).filter(Boolean)) {
      imports[name] = match[2];
    }
  }
  const icon = name => iconRef(imports[name], name);

  const sounds = [];
  for (const match of source.matchAll(
    /icon:\s*<(\w+)\s*\/>,\s*id:\s*'([^']+)',\s*label:\s*'([^']+)',\s*src:\s*getAssetPath\('\/sounds\/([^']+)'\)/g,
  )) {
    sounds.push({ icon: icon(match[1]), id: match[2], label: match[3], path: match[4] });
  }

  const header = source.match(
    /export const \w+: Category = \{\s*icon:\s*<(\w+)\s*\/>,\s*id:\s*'([^']+)'/,
  );
  const title = source.match(/title:\s*'([^']+)'/);
  if (!header || !title || sounds.length === 0) {
    throw new Error(`Could not parse ${file}.tsx`);
  }

  return { icon: icon(header[1]), id: header[2], sounds, title: title[1] };
});

// UI icons used by the desktop shell (not part of any sound).
const ui = {
  alarm: ['md', 'MdOutlineAlarm'],
  check: ['bi', 'BiCheck'],
  clock: ['bi', 'BiTimeFive'],
  close: ['io5', 'IoClose'],
  delete: ['bi', 'BiTrash'],
  down: ['bi', 'BiChevronDown'],
  edit: ['bi', 'BiPencil'],
  grip: ['bi', 'BiMenuAltLeft'],
  heart: ['bi', 'BiSolidHeart'],
  heartOutline: ['bi', 'BiHeart'],
  hourglass: ['bi', 'BiHourglass'],
  loop: ['tb', 'TbRepeat'],
  menu: ['bi', 'BiMenu'],
  mixes: ['bi', 'BiAlbum'],
  moon: ['bi', 'BiMoon'],
  next: ['bi', 'BiSkipNext'],
  pause: ['bi', 'BiPause'],
  play: ['bi', 'BiPlay'],
  playlist: ['bi', 'BiListUl'],
  plus: ['bi', 'BiPlus'],
  prev: ['bi', 'BiSkipPrevious'],
  save: ['bi', 'BiSave'],
  search: ['bi', 'BiSearch'],
  settings: ['bi', 'BiCog'],
  shuffle: ['bi', 'BiShuffle'],
  sounds: ['bi', 'BiGridAlt'],
  stop: ['bi', 'BiStop'],
  swell: ['tb', 'TbWaveSine'],
  timer: ['bi', 'BiTimer'],
  up: ['bi', 'BiChevronUp'],
  volume: ['bi', 'BiVolumeFull'],
  mute: ['bi', 'BiVolumeMute'],
  reset: ['bi', 'BiReset'],
};
const uiIcons = Object.fromEntries(
  Object.entries(ui).map(([key, [pack, name]]) => [key, loadIcon(pack, name)]),
);

const catalog = { categories, icons, ui: uiIcons };
writeFileSync(join(root, 'src', 'lib', 'catalog.json'), JSON.stringify(catalog));

// The Rust side only needs ids, labels and paths.
const rustCatalog = categories.flatMap(c =>
  c.sounds.map(s => ({ id: s.id, label: s.label, path: s.path })),
);
writeFileSync(
  join(root, 'daemon', 'catalog.json'),
  JSON.stringify(rustCatalog, null, 1),
);

const total = categories.reduce((n, c) => n + c.sounds.length, 0);
console.log(`catalog: ${categories.length} categories, ${total} sounds, ${Object.keys(icons).length} icons`);
