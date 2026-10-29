// A real shell behind a page xenia's browser stage can read. The CLI runs in a
// PTY; the page shows the output as text and puts a command line, Enter and the
// two interrupts where a keyboard would be. xenia's senses need no change: `look`
// reads the page, `type` fills the command line, `tap` presses Enter.
//
// The PTY comes from Python's own pty module: macOS's `script` refuses a piped
// stdin (`tcgetattr/ioctl: Operation not supported on socket`), and a native
// pty dependency would be a new one for xenia's trees.
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// The one thing macOS bash prints on start that is not the shell's or the
// product's. It is filtered so the person's first screen is the prompt.
const NOTICE = /^(The default interactive shell is now zsh\.|To update your account to use zsh, please run `chsh -s \/bin\/zsh`\.|For more details, please visit https:\/\/support\.apple\.com\/kb\/HT208050\.)$/;

// CSI sequences, OSC titles, and the two-character charset selects. Enough for
// a CLI's colour and cursor codes; the transcript is text, not a screen buffer.
export const strip = (s) => s
  .replace(/\x1b\][^\x07]*\x07/g, '')
  .replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '')
  .replace(/\x1b[()][A-Z0-9]/g, '');

const BASHRC = `PS1='$ '
stty -echo
PROMPT_COMMAND=''
HISTFILE=/dev/null
HISTSIZE=0
export PATH="$HOME/.local/bin:$PATH"
export TERM=dumb
`;

// One shell per person. Its home is the person's whole world; its cwd is the
// folder the story happens in. The page's `data-last` is the last completed
// line, which is what a screen check reads: a terminal's current state is its
// latest output, not the whole scrollback.
function shellFor(as, home, cwd) {
  mkdirSync(home, { recursive: true });
  mkdirSync(cwd, { recursive: true });
  writeFileSync(join(home, '.bashrc'), BASHRC);
  const proc = spawn('python3', ['-u', '-c', 'import pty,sys; pty.spawn(["/bin/bash","--noprofile"])'], {
    env: {
      HOME: home,
      PATH: `${home}/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin`,
      TERM: 'dumb',
      PS1: '$ ',
    },
    cwd,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  const s = { proc, buf: '', partial: '', lines: [], listeners: new Set(), timer: null };
  // A line editor redraws with a carriage return and erases with a backspace.
  // A real terminal applies both; the page is a transcript, so the server does.
  const apply = (text) => {
    let out = '';
    for (const ch of text) {
      if (ch === '\b') out = out.slice(0, -1);
      else if (ch === '\r') out = '';
      else out += ch;
    }
    return out;
  };
  // A terminal shows its errors on the terminal: stderr goes through the
  // same lines as stdout, so a CLI's refusal is read rather than vanishing.
  const onData = (chunk) => {
    s.buf += strip(chunk.toString()).replace(/\r\n/g, '\n');
    for (;;) {
      const nl = s.buf.indexOf('\n');
      if (nl < 0) break;
      const line = apply(s.buf.slice(0, nl));
      s.buf = s.buf.slice(nl + 1);
      if (NOTICE.test(line)) continue;
      s.lines.push(line);
      for (const send of s.listeners) send({ line });
    }
    const partial = apply(s.buf);
    if (partial === s.partial) return;
    s.partial = partial;
    clearTimeout(s.timer);
    // A prompt has no newline. It is sent once the shell has been quiet for a
    // moment, so a half-written line does not flicker the page.
    s.timer = setTimeout(() => { for (const send of s.listeners) send({ partial: s.partial }); }, 80);
  };
  proc.stdout.on('data', onData);
  proc.stderr.on('data', onData);
  proc.on('exit', () => { for (const send of s.listeners) send({ line: '(the terminal closed)' }); });
  return s;
}

const PAGE = (as) => `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Terminal</title>
<style>
html,body{margin:0;background:#111;color:#e8e8e8;font:14px/1.45 ui-monospace,SFMono-Regular,Menlo,monospace}
#out{padding:14px 14px 140px;white-space:pre-wrap}
.line{min-height:1.45em}
#bar{position:fixed;left:0;right:0;bottom:0;background:#111;border-top:1px solid #333;padding:8px 10px;display:flex;gap:8px;align-items:flex-start}
label{color:#9a9a9a}
textarea{flex:1;height:3.2em;background:#000;color:#fff;border:1px solid #444;font:inherit;padding:4px}
button{background:#2a2a2a;color:#fff;border:1px solid #555;font:inherit;padding:4px 10px;cursor:pointer}
</style></head><body><div id="out"><div class="line" id="seed">$ </div></div>
<div id="bar"><label>Command line <textarea id="cmd" autofocus></textarea></label><button id="enter" type="button">Enter</button><button id="ctrl-c" type="button">Ctrl-C</button><button id="ctrl-d" type="button">Ctrl-D</button></div>
<script>
const out=document.getElementById('out'), cmd=document.getElementById('cmd');
let last=document.getElementById('seed');
const bottom=()=>window.scrollTo(0, document.body.scrollHeight);
function append(text){last=null;const d=document.createElement('div');d.className='line';d.textContent=text;out.appendChild(d);const t=text.trim();if(t)document.body.dataset.last=t;bottom();return d;}
function partial(text){if(!last){last=document.createElement('div');last.className='line';out.appendChild(last);}last.textContent=text;bottom();}
new EventSource('/events?as=${encodeURIComponent(as)}').onmessage=(e)=>{const m=JSON.parse(e.data);if('line' in m)append(m.line);else if('partial' in m)partial(m.partial);};
async function post(path, body){await fetch(path,{method:'POST',body:body ?? ''});}
async function submit(){const text=cmd.value;if(!text.trim())return;cmd.value='';const line='$ '+text;if(last){last.textContent=line;last=null;}else{const d=document.createElement('div');d.className='line';d.textContent=line;out.appendChild(d);}bottom();await post('/cmd?as=${encodeURIComponent(as)}',text);cmd.focus();}
document.getElementById('enter').onclick=submit;
document.getElementById('ctrl-c').onclick=()=>post('/key?as=${encodeURIComponent(as)}&key=c');
document.getElementById('ctrl-d').onclick=()=>post('/key?as=${encodeURIComponent(as)}&key=d');
cmd.addEventListener('keydown',(e)=>{if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();submit();}});
</script></body></html>`;

// `people` is `[{ as, home, cwd }]`; each gets its own shell and page. The
// server is bound to this computer only, as the worlds are.
export async function terminal(people) {
  const shells = new Map();
  const shell = (as) => {
    if (shells.has(as)) return shells.get(as);
    const p = people.find((x) => x.as === as);
    if (!p) throw new Error(`no person named ${JSON.stringify(as)} here`);
    const s = shellFor(as, p.home, p.cwd);
    shells.set(as, s);
    return s;
  };
  const server = createServer((req, res) => {
    const url = new URL(req.url ?? '/', 'http://127.0.0.1');
    const as = url.searchParams.get('as') ?? people[0]?.as ?? '';
    if (url.pathname === '/') {
      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
      return res.end(PAGE(as));
    }
    if (url.pathname === '/events') {
      res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-cache', connection: 'keep-alive' });
      const s = shell(as);
      const send = (m) => res.write(`data: ${JSON.stringify(m)}\n\n`);
      for (const m of s.lines) send({ line: m });
      if (s.partial) send({ partial: s.partial });
      s.listeners.add(send);
      req.on('close', () => s.listeners.delete(send));
      return;
    }
    if (url.pathname === '/cmd' && req.method === 'POST') {
      let body = '';
      req.on('data', (c) => (body += c));
      req.on('end', () => {
        shell(as).proc.stdin.write(`${body}\n`);
        res.writeHead(200);
        res.end('ok');
      });
      return;
    }
    if (url.pathname === '/key' && req.method === 'POST') {
      const key = url.searchParams.get('key');
      shell(as).proc.stdin.write(key === 'd' ? '\x04' : '\x03');
      res.writeHead(200);
      res.end('ok');
      return;
    }
    res.writeHead(404);
    res.end('not here');
  });
  await new Promise((done) => server.listen(0, '127.0.0.1', done));
  const { port } = server.address();
  const origin = `http://127.0.0.1:${port}`;
  return {
    origin,
    url: (as) => `${origin}/?as=${encodeURIComponent(as)}`,
    close: () => {
      for (const s of shells.values()) s.proc.kill();
      server.close();
    },
  };
}
