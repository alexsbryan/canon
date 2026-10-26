# Start a house: one steward now, the whole house later

One person writes the house's first rules alone. The house joins through Slack
later, and every rule carries over. This page is the whole path. [Run it through
Slack](./SELF_HOST.md) covers the bot itself.

## Seed it

**1. Install canon 0.5.0.** Older canon can't read `share`.

```sh
curl -fsSL https://raw.githubusercontent.com/alexsbryan/canon/main/install.sh | sh
canon --version          # 0.5.0
```

**2. Act as the name the house will know you by**: your exact Slack full name,
with no dots in it. The bot names everyone `human:<Slack full name>`, so any
other spelling makes you two people. Put it in your shell profile.

```sh
export CANON_ACTOR='human:Alex Bryan'
```

**3. Start it in a private git repository**, the one the bot will sync later.

```sh
mkdir house && cd house && git init
canon init --profile house
```

**4. Grant yourself the house before anything else.** The first grant closes an
open canon. If your first grant goes to someone else, your later grants don't
take.

```sh
canon grant "$CANON_ACTOR" house
```

**5. Write the rules, each in `house`**, the scope the bot proposes in. Say in
the first one what this is.

```sh
canon add "These rules were written by Alex alone to start; when the house joins, any of them is open to change." --scope house
canon add "Quiet hours are 10pm to 7am." --scope house
canon list               # what's live
canon who house          # you, deciding alone under standing
```

To read agreements you already keep, run a local OpenAI-compatible model
server (llama-server on `:8080`, for example) and point canon at it. Check
each proposal against the passage it quotes.

```sh
canon config set endpoint http://localhost:8080/v1
canon config set model YOUR_MODEL_NAME
canon draft --from ~/house-notes
```

**6. Commit after each sitting.** `.canon/.gitignore` keeps the model settings,
which belong to one machine, out of the repository.

```sh
git add .canon && git commit -m "canon: what you added" && git push
```

## Don't

- **Run `canon identify`.** The bot writes for everyone unsigned. Once a name
  binds a key, the bot's writes for that person are refused.
- **Edit `.canon/acts.jsonl` by hand.** canon only appends to it.
- **Choose `threshold` or `joint`.** A fixed count, or a list of names, locks a
  house that someone leaves: nothing passes again, including a change of rule.

## Open it to the house

**1. Choose how the house decides while you still hold it alone.** Once others
hold it, changing the rule is itself a proposal. Set it for the whole canon,
with no `--scope`.

```sh
canon ratification set share:60%     # or majority, or consent:3d
```

`share` and `majority` count whoever holds the house when each proposal is
written. A house that shrinks can still decide.

**2. Grant each member** by their exact Slack full name.

```sh
canon grant 'human:Sam Lee' house
```

When someone moves out, withdraw them that day:
`canon withdraw 'human:Sam Lee' house`. Until you do, they still count.

**3. Start the bot** ([Run it through Slack](./SELF_HOST.md)) on a machine that
stays on, with `YOUR_NAME` set to the name from step 2 of seeding. Build its
image at 0.5.0, and put 0.5.0 on every machine that reads the canon.

**4. Tell the house:**
- **Proposing:** use `/canon propose`.
- **Approving and objecting:** use the buttons. An objection needs a reason.
- **Under `share` or `majority`:** a rule takes enough Approves.
- **Under consent:** only an objection counts.
- **When nothing seems to happen:** Slack doesn't say when a proposal settles
  without a click, so check `/canon rules`.
