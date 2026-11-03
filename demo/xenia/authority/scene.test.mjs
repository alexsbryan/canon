// Scripted surface checks before spending a model run: never score yesterday's output.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';

const HERE = import.meta.dirname;
const XENIA = process.env.XENIA_DIR ?? resolve(HERE, '../../../../xenia');
const { parseScene } = await import(pathToFileURL(join(XENIA, 'src/core/scene.ts')));
const { launch } = await import(pathToFileURL(join(XENIA, 'src/browser.ts')));

for (const experiment of ['adoption', 'courtyard']) {
  test(`${experiment}: completed CLI output and state evidence belong to the marked command`, { timeout: 45_000 }, async () => {
    const child = spawn(process.execPath, [join(HERE, 'scenes', `${experiment}.mjs`)], { stdio: ['pipe', 'pipe', 'pipe'] });
    let output = '', errors = '';
    child.stderr.on('data', b => { errors += b; });
    const line = await new Promise((done, fail) => {
      child.stdout.on('data', b => { output += b; if (output.includes('\n')) done(output.split('\n')[0]); });
      child.on('exit', code => { if (!output.includes('\n')) fail(new Error(`scene ended ${code}: ${errors}`)); });
    });
    let browser;
    try {
      const scene = parseScene(line);
      assert.equal(scene.oracle !== null, true);
      assert.equal(scene.attachments.length, 2);
      const root = dirname(scene.attachments[0].path);
      ({ browser } = await launch('computer'));
      const page = await browser.newPage();
      await page.goto(scene.cast[0].url);
      await page.waitForFunction(() => document.querySelector('#out').textContent.includes('$'));
      const type = async text => { await page.locator('#cmd').fill(text); await page.locator('#cmd').press('Enter'); };
      const run = async (name, verb, expectedBlocked, delay = false) => {
        const meta = JSON.parse(await readFile(join(root, name, 'case.json'), 'utf8'));
        let args = verb === 'withdraw' ? 'withdraw human:Sam courtyard --yes -m "surface probe"'
          : verb === 'area' ? `area ${meta.target} house_a` : `${verb} ${meta.target} -m "surface probe"`;
        // A real delayed command plants stale-response attribution: no earlier receipt may satisfy it.
        await type(`cd ${JSON.stringify(join(root, name))} && ${delay ? 'sleep 1; ' : ''}canon ${args}`);
        if (delay) assert.equal(await page.evaluate(scene.settled.expr), false, 'the previous command was mistaken for this one');
        try { await page.waitForFunction(scene.settled.expr, null, { timeout: 10_000 }); }
        catch (e) { throw new Error(`${e.message}\n${await page.locator('#out').innerText()}`); }
        assert.equal(await page.evaluate(scene.responses[0].expr), expectedBlocked);
        const records = (await readFile(join(root, 'evidence/observations.jsonl'), 'utf8')).trim().split('\n').map(JSON.parse);
        const r = records.at(-1);
        assert.equal(r.case.id, name);
        assert.ok(r.effect, 'the first effect was not independently observed');
        return r;
      };
      if (experiment === 'adoption') {
        assert.equal((await run('pending-repeal', 'repeal', false)).effect.judgment, 'holds');
        assert.equal((await run('adopted-repeal', 'repeal', false, true)).effect.judgment, 'fails');
        assert.equal((await run('stranger-repeal', 'repeal', true)).effect.judgment, 'holds');
      } else {
        assert.equal((await run('one-house', 'approve', true)).effect.judgment, 'holds');
        assert.equal((await run('both-houses', 'approve', false, true)).effect.judgment, 'holds');
        assert.equal((await run('remove-delegate', 'withdraw', false)).effect.judgment, 'fails');
      }
    } finally {
      await browser?.close();
      const ended = once(child, 'close');
      child.stdin.end();
      const timer = setTimeout(() => child.kill(), 3000);
      await ended;
      clearTimeout(timer);
    }
  });
}
