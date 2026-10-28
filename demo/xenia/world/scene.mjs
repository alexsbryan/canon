// One canon story, stood up for one person. The scene command prints xenia's
// scene line and keeps the world up until xenia closes its stdin (DESIGN §5,
// "Scene"). Each story is a folder a real person could be in; the openings are
// what a housemate told them, and nothing else is said about canon.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { mkdir, mkdtemp, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { terminal } from './terminal.mjs';

const HERE = import.meta.dirname;
const BIN = process.env.CANON_BIN ?? resolve(HERE, '../../../target/debug/canon');
const PATH = '/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin';

const PEOPLE = {
  mira: { name: 'Mira', email: 'mira@house.local', from: 'Sam' },
  sam: { name: 'Sam', email: 'sam@house.local', from: 'Mira' },
  june: { name: 'June', email: 'june@house.local', from: 'Sam' },
};

// The last completed output line is the terminal's current screen. A check on
// it is the closest a scrolling transcript has to "what screen is this".
const last = (test) => ({ expr: `(document.body.dataset.last ?? '')${test}` });
const ruleLive = last(".match(/rules? live/)");

const STORIES = {
  'first-hour': {
    begins: 'on a computer, at a terminal, in an empty house folder, with canon not yet installed',
    install: true,
    seed: null,
    opening:
      'I have been meaning to get our house\'s rules out of the group chat. Could you set up canon here this afternoon? Install it the way the README says:\n\n' +
      'curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh | sh\n\n' +
      'Then start it with the house profile, add "Quiet hours are 10pm to 7am.", and leave the list showing. Thanks.',
    goal: { text: '1 rule live' },
    milestones: [{ text: 'GETTING_STARTED' }, { text: 'canon initialised' }, { text: 'in force' }],
    screens: [
      { name: 'the rules', ...ruleLive },
      { name: 'a rule written', ...last(" === 'in force'") },
      { name: 'a new canon', ...last(".includes('drawing lots')") },
      { name: 'the short help', ...last(".includes('agent surface')") },
      { name: 'the version', ...last(" === 'canon 0.5.0'") },
      { name: 'the installer', ...last(".includes('GETTING_STARTED')") },
      { name: 'the terminal', text: 'Command line' },
    ],
    bounds: [{ from: { text: 'Command line' }, to: { text: '1 rule live' }, seconds: 600 }],
  },
  join: {
    begins: 'on a computer, at a terminal, in an empty folder, with canon installed and a housemate\'s share block in ~/share.txt',
    install: false,
    seed: 'share',
    opening: 'Here is our house\'s canon block. I saved it in ~/share.txt. Could you start your own copy here and read it in? Then I can show the others.',
    goal: { text: '1 rule live' },
    milestones: [{ text: 'canon initialised' }, { text: 'later generations' }],
    screens: [
      { name: 'the rules', ...ruleLive },
      { name: 'the adopted canon', ...last(".includes('later generations')") },
      { name: 'a new canon', ...last(".includes('drawing lots')") },
      { name: 'an error', ...last(".includes('no canon here')") },
      { name: 'the terminal', text: 'Command line' },
    ],
    bounds: [{ from: { text: 'Command line' }, to: { text: '1 rule live' }, seconds: 300 }],
  },
  draft: {
    begins: 'on a computer, at a terminal, in an empty house folder, with canon installed and the house\'s notes in ~/house-notes',
    install: false,
    seed: 'notes',
    opening:
      'The house\'s notes are in ~/house-notes. There is a model on this machine at http://localhost:9741/v1 that answers to primary. ' +
      'Could you use canon to read the notes and write down what it finds? Finish with the list.',
    goal: { text: '1 rule live' },
    milestones: [{ text: 'model = primary' }, { text: 'Candidate 1 of' }, { text: 'rule accepted' }],
    screens: [
      { name: 'the rules', ...ruleLive },
      { name: 'a rule accepted', ...last(".includes('rule accepted')") },
      { name: 'a proposal', text: 'Candidate 1 of' },
      { name: 'the model', ...last(".includes('model =')") },
      { name: 'a new canon', ...last(".includes('drawing lots')") },
      { name: 'an error', ...last(".includes('no canon here')") },
      { name: 'the terminal', text: 'Command line' },
    ],
    bounds: [{ from: { text: 'Command line' }, to: { text: '1 rule live' }, seconds: 600 }],
  },
  explore: {
    begins: 'on a computer, at a terminal, in the house\'s canon folder, with canon installed',
    install: false,
    seed: 'explore',
    opening: 'I keep our house\'s rules in canon now. Have a play and tell me what you think it is for.',
    goal: null,
    milestones: [],
    explore: { purpose: 'A place for the rules a group already keeps: what is in force, what changed and why, and who may decide.' },
    screens: [
      { name: 'the rules', ...ruleLive },
      { name: 'a rule written or changed', ...last(" === 'in force'") },
      { name: 'the questions', ...last(" === '1 open'") },
      { name: 'the open canon', ...last(".includes('widest scopes')") },
      { name: 'the full help', ...last(".includes('force-resolved')") },
      { name: 'the short help', ...last(".includes('agent surface')") },
      { name: 'the raw acts', ...last('.match(/\\d+ acts?$/)') },
      { name: 'the terminal', text: 'Command line' },
    ],
    bounds: [],
  },
};

// The seed's acts are a housemate's, not the person's. CANON_ACTOR is stripped
// from the ambient environment, so the audit cannot inherit one by accident.
const seedEnv = (home) => ({
  ...Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('CANON_'))),
  HOME: home,
  PATH: `${home}/.local/bin:${PATH}`,
  TERM: 'dumb',
  CANON_ACTOR: 'human:Sam',
});

const run = (cwd, home, args) => execFileSync(BIN, args, { cwd, env: seedEnv(home), encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });

function seed(kind, home, house) {
  if (kind === 'share') {
    const src = join(home, 'share-src');
    mkdirSync(src, { recursive: true });
    run(src, home, ['init', '--profile', 'house']);
    run(src, home, ['add', 'Guests may stay up to three nights.']);
    writeFileSync(join(home, 'share.txt'), run(src, home, ['share']));
    return;
  }
  if (kind === 'notes') {
    mkdirSync(join(home, 'house-notes'), { recursive: true });
    writeFileSync(
      join(home, 'house-notes', 'chat.txt'),
      'Sam: reminder the recycling goes out sunday night, not monday\nMira: ^ this has bitten us three times now, can we make it a rule\nSam: yes. recycling out sunday night.\n',
    );
    return;
  }
  if (kind === 'explore') {
    run(house, home, ['init', '--profile', 'house']);
    run(house, home, ['add', 'Quiet hours are 10pm to 7am.']);
    run(house, home, ['add', 'Whoever cooks does not wash up.']);
    const guest = run(house, home, ['add', 'Guests may stay up to three nights.']).split(/\s+/)[0];
    run(house, home, ['supersede', guest, 'Guests may stay up to three nights; longer needs a house chat.', '-m', 'Sam\'s cousin stayed two weeks and nobody knew how to raise it.']);
    run(house, home, ['question', 'Who looks after the plants when everyone travels at once?']);
    run(house, home, ['silence', 'a chore rotation', '-m', 'We decided in March not to have one; it would turn a kindness into a duty.']);
  }
}


export async function runScene(storyId, personId) {
  const [story, person] = [STORIES[storyId], PEOPLE[personId]];
  if (!story) throw new Error(`no story named ${JSON.stringify(storyId)}: one of ${Object.keys(STORIES).join(', ')}`);
  if (!person) throw new Error(`no person named ${JSON.stringify(personId)}: one of ${Object.keys(PEOPLE).join(', ')}`);
  if (!existsSync(BIN)) throw new Error(`${BIN} is not built; cargo build -p canon-cli, or set CANON_BIN`);

  const root = await mkdtemp(join(tmpdir(), 'canon-xenia-'));
  const home = join(root, 'home');
  const house = join(home, 'house');
  await mkdir(house, { recursive: true });
  await mkdir(join(home, '.local', 'bin'), { recursive: true });
  await writeFile(join(home, '.gitconfig'), `[user]\n\tname = ${person.name}\n\temail = ${person.email}\n`);
  if (!story.install) await symlink(BIN, join(home, '.local', 'bin', 'canon'));
  seed(story.seed, home, house);

  const t = await terminal([{ as: person.name, home, cwd: house }]);
  const line = {
    begins: story.begins,
    cast: [
      {
        as: person.name,
        person: `../people/${personId}.md`,
        device: 'computer',
        url: t.url(person.name),
        ...(story.goal ? { goal: story.goal } : {}),
        milestones: story.milestones,
      },
    ],
    screens: story.screens,
    displays: [],
    channels: [{ name: 'the house chat', members: [person.name], opening: [{ from: person.from, to: [person.name], text: story.opening }] }],
    faults: [{ expr: "document.body.innerText.toLowerCase().includes('panicked at')" }],
    bounds: story.bounds,
    reach: [t.origin],
    ...(story.explore ? { explore: story.explore } : {}),
  };
  console.log(JSON.stringify(line));
  await new Promise((done) => {
    process.stdin.on('end', done);
    process.stdin.on('close', done);
    process.stdin.resume();
  });
  await t.close();
}
