# study-app

A desktop app for thinking in writing: a spatial canvas crossed with a
spaced-repetition system, built around one specific failure mode.

Tauri v2 · SvelteKit (Svelte 5 runes) · Rust · SQLite. Local-first, single user,
no server, no sync.

---

## Why it is shaped like this

The app exists to fix a three-stage problem:

1. **Reading generates far more questions than answers.** The unanswered ones
   pile up and make a kind of mental noise that will not clear until they are
   written down somewhere.
2. **Sometimes** that gets organised and some of it gets answered.
3. **A clean, formal document of the whole topic** is where the knowledge
   actually consolidates — and it is also where the work almost always gets
   abandoned. It is expensive, it has no defined edges, and the effort goes into
   making it look publishable rather than into understanding it.

So the material dies at stage one.

Every design decision here follows from that. If a change makes stage 1 cheaper
or makes stage 3 finishable, it belongs. If it adds a place to tidy, a setting
to fiddle with, or a number that grows while you sleep, it does not.

Three rules fall out of it, and they show up everywhere in the code:

- **Nothing accumulates in your eyeline.** The review queue is capped at a day's
  worth (`DAILY_CAP`, `src-tauri/src/review.rs`) and the true backlog is never
  returned by anything. The Tasks *panel* is one day wide. Coming back to "412
  due" is the most reliable way to quit.
- **Knowledge state is derived, never stored.** There are no illegal transitions
  to guard and no way for a note to get stuck — see `note_state.rs`.
- **Structure is a by-product of working, not a thing you maintain first.**
  Boards are ordered by when you last opened them; there are no folders, no
  paths, no filing step.

---

## The shape of the app

```
┌──────────┬───────────────────────────────┬────────────────┐
│ sidebar  │  the thing you are working on │  inspector     │
│          │  (the centre)                 │  (right slot)  │
└──────────┴───────────────────────────────┴────────────────┘
```

Both side panels run the full height of the window; the top bar spans only the
centre. That is deliberate — the panels are the frame, and the work sits inside
it rather than under a full-width chrome bar.

**Screens** (the centre; things you navigate to):

| Route             | What it is                                                  |
| ----------------- | ----------------------------------------------------------- |
| `/`               | Not a screen — redirects to whatever `prefs.startPage` says |
| `/journal`        | The day's page. Where capture happens. The default landing  |
| `/questions`      | The question queue, led by one question rather than a list  |
| `/review`         | The day's cards                                             |
| `/library`        | Every note                                                  |
| `/tags`           | Tags                                                        |
| `/tasks`          | Every task, gathered out of note bodies                     |
| `/t/[id]`         | A board: the spatial canvas                                 |
| `/t/[id]/recall`  | Reproducing a board's document from memory                  |

**The inspector** (`src/lib/components/slot/`) is never a destination. It only
ever describes what the centre is already showing: this note's info, this
board's questions, this day's tasks, this document's headings. Six tools, always
the same six, always in the same order, all always pressable — a panel that
rearranges itself as you navigate is one you can never learn. Pressing a tool
that has nothing to say prints a sentence explaining what it *would* show, which
is how you find out what it is for.

Closed means **gone**, not a rail of icons down the edge. A rail costs width and
gives nothing back.

---

## Architecture

### Where truth lives

**The note body is the single source of truth.** It is a TipTap/ProseMirror
JSON document stored in `notes.body_json`, and several things are *derived* from
it rather than stored alongside it:

- `note_tasks` — every `taskItem` in the body. Rebuilt from scratch on every
  save.
- `note_links` — every mention of another note. Same.
- Knowledge state — computed in `note_state.rs` on every read.

Delete `note_tasks` or `note_links` and a re-save reconstructs them. They exist
only because a JSON blob is opaque to SQL: you cannot ask "what is due today" or
"what links here" of a text column.

### The layers

```
src/                       SvelteKit frontend
  routes/                  screens
  lib/components/          UI, including canvas/ and slot/ (the inspector)
  lib/state/*.svelte.ts    runes-based stores (slot, prefs, data, drag, menu…)
  lib/ipc.ts               EVERY call into Rust. Nothing else uses `invoke`
  lib/types.ts             mirrors of the Rust domain types

src-tauri/src/
  lib.rs                   wiring: pool, data dir, the command registry
  commands.rs              the IPC surface. Thin: translate, call one repo fn
  domain.rs                the types that cross the wire
  note_state.rs            the two axes of knowledge state (derived)
  review.rs                scheduling maths. No card content, only schedules
  repo/                    ALL SQL lives here and nowhere else
  migrations/              embedded, run on startup
```

### Two things worth knowing before you change anything

**1. `ipc.ts` is the only door into Rust, and mutations announce themselves.**

Every writing command goes through `mutate()`, which marks what kind of data it
touched. Screens declare what they depend on:

```ts
$effect(() => {
  data.questions;                 // re-run when any question changes
  ipc.questionQueue(id).then(…);
});
```

Doing this in `ipc.ts` rather than at each call site is the point: a signal you
have to remember to send is one that will be forgotten. It is deliberately
coarse — "some question changed" is enough to re-read a list that is one cheap
query, and anything finer would be a cache, which is a second copy of the truth.
See `src/lib/state/data.svelte.ts`.

**2. While an editor holds a document, that editor owns it.**

An open editor has its own in-memory copy of the note body. Anything that writes
that body to the database behind it will be silently overwritten the next time
it saves. `set_task_done` does exactly that, which is why ticking a task in the
inspector routes through the open editor instead — see `src/lib/tasks.ts` and
`slot.holdDocument`.

If you add another way to edit a note body from outside the editor, it has to
respect this or it will produce the same ghost.

---

## Traps that have already bitten

Each of these cost real debugging. They are all documented at the site too; this
is the index.

- **Never subscribe the journal editor to `data.notes` for its body.** Saving
  the page marks notes changed, so the editor would reload the day it had just
  written and pour it back mid-sentence. `setContent` replaces the whole
  document, and ProseMirror maps the caret to the end — so your next word lands
  on the last line. (`src/routes/journal/+page.svelte`)
- **Never save a document that has not changed.** The editor's copy goes stale
  the moment anything else edits the note, so writing it out "just in case" on
  teardown can undo someone else's edit. Both editors track `dirty`.
- **Task ids are `<note id>:<position>`, and position skips empty task items.**
  The backend and the frontend both have to count the same way or a tick lands
  on the wrong line. (`repo/derived.rs`, `src/lib/tasks.ts`)
- **`serde(flatten)` and `sqlx(flatten)` are different attributes.** A struct
  with only the second reads fine from SQLite and then serialises nested, so
  every field arrives `undefined` in the frontend with no error anywhere.
- **A `.svelte.ts` module-level singleton is shared by every component that
  imports it.** The vendored editor's table-of-contents does this, which is why
  the Outline panel walks the document itself instead. (`src/lib/outline.ts`)
- **`{@const}` is only legal as an immediate child of a block**, so a dynamic
  component has to come from a capitalised `$derived`.

---

## Running it

```bash
bun install
bun run tauri dev        # the app (starts vite on :1420 itself)
bun run check            # svelte-check + tsc
cd src-tauri && cargo test
```

`bun run dev` starts only the web half, which is rarely what you want — most of
the app needs the Rust side.

**Port 1420 in use** means a previous dev session did not shut down. Stop the
stray `study-app.exe` (and its vite child) and try again.

The database lives beside the app's data directory:

```
%APPDATA%/com.kiza2.study-app/study-app.db
```

Migrations are embedded and run on startup, so a stale database upgrades itself
on first launch. WAL journal mode, with a busy timeout — a locked database
surfaces as an error rather than a hang, because a blocking `setup` hook means
no window ever appears.

---

## Where to start reading

In this order, they explain the app to you:

1. `src-tauri/src/note_state.rs` — the two axes, and why state is derived.
2. `src-tauri/src/review.rs` — why the queue is capped and nothing decays.
3. `src/lib/state/slot.svelte.ts` — the inspector, and the memory-mode lock.
4. `src/routes/journal/+page.svelte` — capture, and the autosave rules.
5. `src/lib/components/NoteEditor.svelte` — the four steps of the memory flow.
6. `src/lib/components/canvas/TopicCanvas.svelte` — the board.

Most files open with a comment explaining what they are for and why they are
that way. Those headers are the real documentation; this file is only the map.
