// Canon's two optional-charter rehearsals; xenia supplies the stage and the people.
import { execFileSync } from 'node:child_process';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { terminal } from '../world/terminal.mjs';

const HERE = import.meta.dirname;
const BIN = process.env.CANON_BIN ?? resolve(HERE, '../../../target/debug/canon');
const WORDS = {
  adoption: {
    as: 'Helper', file: '../people/helper.md', from: 'Mira',
    text: `June and I agreed that you can cancel your own pending proposals. Once we both adopt a rule, changing or removing it belongs to the house, not its original drafter. I've left separate copies in pending-repeal, adopted-repeal, pending-undo, adopted-undo, and stranger-repeal. Each folder starts afresh and sets your identity for that copy; stranger-repeal is a different, unrelated agent. START.md lists them and ABOUT.md and RULE_ID in each identify the rule. Please try both repeal and undo on your pending and adopted proposals, then the stranger's repeal, and tell us whether the software follows that agreement. You can use canon's help. Mark each command that attempts a change with kind authority. Type the complete canon command with a newline so it runs as you mark it; read RULE_ID first rather than using a shell substitution in the marked command. The Command started/complete lines are only the recorder, not canon's judgment.`,
  },
  courtyard: {
    as: 'Mira', file: '../people/mira.md', from: 'Sam',
    text: `You represent House A and I represent House B. We each approve local changes; changes to the shared courtyard need both of us. Neither of us may unilaterally remove the other's courtyard seat or move courtyard rules into our own house. There are five fresh copies here: local, one-house, both-houses, remove-delegate, and move-rule. In local, approve the local proposal. In one-house, try approving a courtyard change by yourself. In both-houses, I've already approved it; add your approval. In remove-delegate, try withdrawing my courtyard seat without my assent. In move-rule, try moving the adopted courtyard rule to house_a. START.md lists them, and ABOUT.md and RULE_ID in each identify the rule. Each copy starts afresh and your commands act as human:Mira. Use canon's own help and tell me where our agreement holds or breaks. Mark each command that attempts a change with kind authority. Type the complete canon command with a newline so it runs as you mark it; read RULE_ID first rather than using a shell substitution in the marked command. The Command started/complete lines are only the recorder, not canon's judgment.`,
  },
};

// Only output between the matching start/complete lines belongs to this invocation.
const lines = `[...document.querySelectorAll('#out .line')].map(x => x.textContent.trim().replace(/^\\$ (?=Command (?:started|complete): )/, ''))`;
const settled = `(() => { const l = ${lines}; const c = l.findLastIndex(x => x.startsWith('$ ') && x !== '$'); const s = l.findLastIndex(x => x.startsWith('Command started: ')); const e = l.findLastIndex(x => x.startsWith('Command complete: ')); return s > c && e > s && l[s].slice(17) === l[e].slice(18); })()`;
const blocked = `(() => { const l = ${lines}; const s = l.findLastIndex(x => x.startsWith('Command started: ')); const e = l.findLastIndex(x => x.startsWith('Command complete: ')); return s >= 0 && e > s && /(^|\\n)\\s*(NOT APPLIED|PROPOSED|REFUSED|error:)/i.test(l.slice(s + 1, e).join('\\n')); })()`;

export async function runAuthority(experiment) {
  const person = WORDS[experiment];
  const root = await mkdtemp(join(tmpdir(), `canon-authority-${experiment}-`));
  execFileSync('python3', [join(HERE, 'authority.py'), 'prepare', experiment, root], { env: { ...process.env, CANON_BIN: BIN } });
  const t = await terminal([{ as: person.as, home: join(root, 'home'), cwd: root }], {
    CANON_BIN: BIN, CANON_EXPERIMENT_ROOT: root,
  });
  console.log(JSON.stringify({
    begins: `at a real canon CLI through a recording terminal bridge; five prepared copies, identity fixed per copy, setup and live negotiations not seen`,
    cast: [{ as: person.as, person: person.file, device: 'computer', url: t.url(person.as) }],
    screens: [{ name: 'the terminal', text: 'Command line' }],
    channels: [{ name: 'the house chat', members: [person.as], opening: [] }],
    scenes: [{ id: experiment, brief: [{ from: person.from, to: [person.as], text: person.text }],
      feedback: ['What happened when you tried the agreement?',
        { ask: 'What was unclear about who could change what?', bad: 'authority unclear' }],
      minutes: 6, talk: 180 }],
    reach: [t.origin], faults: [{ expr: "document.body.innerText.includes('panicked at')" }],
    kinds: { authority: { objects: ['blocked'] } }, responses: [{ name: 'blocked', expr: blocked }],
    settled: { expr: settled, seconds: 8 },
    oracle: `python3 ${JSON.stringify(join(HERE, 'authority.py'))} oracle ${JSON.stringify(join(root, 'evidence'))}`,
    attachments: [
      { as: person.as, path: join(root, 'manifest.json') },
      { as: person.as, path: join(root, 'evidence/observations.jsonl') },
    ],
  }));
  await new Promise(done => { process.stdin.on('end', done); process.stdin.on('close', done); process.stdin.resume(); });
  await t.close();
}
