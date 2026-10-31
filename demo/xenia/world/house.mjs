// One house, declared once (demo/xenia/houses/<name>.json), stood up for xenia:
// the shared terminal and homes, the seed, then the run's line — the cast, the
// screens, and the scenarios as scenes (xenia DESIGN §5, "A run"). The house
// file's paths are relative to demo/xenia; its people are its own.
import { symlinkSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { terminal } from './terminal.mjs';

const HERE = import.meta.dirname;
const DEMO = resolve(HERE, '..');
const BIN = process.env.CANON_BIN ?? resolve(DEMO, '../../target/debug/canon');

// A brief is a chat line, "<from>: <text>" (demo/xenia/README.md, "The houses").
const brief = (text, to) => {
  const m = /^([^:]+):\s*([\s\S]+)$/.exec(text.trim());
  if (!m) throw new Error(`a brief is "<from>: <text>": ${JSON.stringify(text)}`);
  return [{ from: m[1], to, text: m[2] }];
};

export async function runHouse(file) {
  const house = JSON.parse(await readFile(file, 'utf8'));
  if (house.surface?.kind !== 'terminal') throw new Error(`house ${house.house}: the surface is a terminal, and this one is ${JSON.stringify(house.surface?.kind)}`);
  const root = await mkdtemp(join(tmpdir(), 'canon-house-'));
  const names = Object.keys(house.people);
  const homes = Object.fromEntries(names.map((as) => [as, join(root, (house.surface.homes ?? 'home/{as}').split('{as}').join(as))]));
  for (const home of Object.values(homes)) await mkdir(join(home, '.local', 'bin'), { recursive: true });
  const cwd = join(root, house.surface.cwd ?? 'house');
  await mkdir(cwd, { recursive: true });
  for (const seed of house.seed) {
    for (const as of seed.path.includes('{as}') ? names : [null]) {
      const path = join(root, as ? seed.path.split('{as}').join(as) : seed.path);
      await mkdir(dirname(path), { recursive: true });
      await writeFile(path, as ? seed.text.split('{as}').join(as) : seed.text);
    }
  }
  if (house.surface.installed) for (const home of Object.values(homes)) symlinkSync(BIN, join(home, '.local', 'bin', 'canon'));
  const t = await terminal(names.map((as) => ({ as, home: homes[as], cwd })), house.surface.env ?? {});
  const line = {
    begins: `${names.join(', ')} at their own terminals in the house they share; canon ${house.surface.installed ? 'installed' : 'not yet installed'}`,
    cast: names.map((as) => ({ as, person: `../people/${basename(house.people[as].file)}`, device: 'computer', url: t.url(as) })),
    screens: house.screens.map(({ name, match }) => ({ name, expr: `((document.body.dataset.last ?? '').match(new RegExp(${JSON.stringify(match)})) ?? null) !== null` })),
    displays: [],
    channels: [{ name: house.chat.name, members: house.chat.members, opening: [] }],
    faults: [{ expr: "document.body.innerText.toLowerCase().includes('panicked at')" }],
    bounds: [],
    reach: [t.origin],
    scenes: [...house.scenarios, { id: house.debrief.id ?? 'debrief', ...house.debrief }].map(({ id, brief: text, feedback, reach, minutes, talk }) => ({ id, brief: brief(text, house.chat.members), feedback, ...(reach ? { reach } : {}), ...(minutes ? { minutes } : {}), ...(talk ? { talk } : {}) })),
  };
  console.log(JSON.stringify(line));
  await new Promise((done) => {
    process.stdin.on('end', done);
    process.stdin.on('close', done);
    process.stdin.resume();
  });
  await t.close();
}
