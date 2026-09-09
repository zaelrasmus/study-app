<script lang="ts">
	/**
	 * A topic's canvas.
	 *
	 * Must live inside a `<SvelteFlowProvider>`: `useSvelteFlow()` reads the flow
	 * store out of Svelte context and throws if no provider is an ancestor, which
	 * is why the flow lives in this child component rather than in the route.
	 *
	 * The arrangement is the structure. Frames carry document order explicitly
	 * and show it; membership in a frame is what commits a node to the document;
	 * anything outside a frame stays scratch.
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		Background,
		BackgroundVariant,
		Controls,
		SvelteFlow,
		useSvelteFlow,
		type Edge,
		type Node
	} from '@xyflow/svelte';

	import { toast } from 'svelte-sonner';
	import { convertFileSrc } from '@tauri-apps/api/core';
	import { getCurrentWebview } from '@tauri-apps/api/webview';
	import { open as pickFile } from '@tauri-apps/plugin-dialog';
	import { openPath, openUrl } from '@tauri-apps/plugin-opener';
	import * as ipc from '$lib/ipc';
	import { session } from '$lib/state/session.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import { drag, type DragPayload, type DropContext } from '$lib/state/drag.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { CARD_COLOURS } from '$lib/cardColour';
	import { DEFAULT_INK, normalise } from '$lib/ink';
	import type { Asset, CanvasEdge, CanvasNode, Frame, Note, Question, Topic } from '$lib/types';

	import NoteNode from './NoteNode.svelte';
	import FrameNode from './FrameNode.svelte';
	import TopicNode from './TopicNode.svelte';
	import InkNode from './InkNode.svelte';
	import InkOverlay from './InkOverlay.svelte';
	import RectOverlay from './RectOverlay.svelte';
	import CanvasEmptyState from './CanvasEmptyState.svelte';
	import CanvasToolbar from './CanvasToolbar.svelte';
	import type { Tool } from './tools';

	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import DocumentPanel from '$lib/components/DocumentPanel.svelte';
	import TextNode from './TextNode.svelte';
	import AssetNode from './AssetNode.svelte';
	import LinkNode from './LinkNode.svelte';
	import LinkPrompt from './LinkPrompt.svelte';
	import QuestionNode from './QuestionNode.svelte';
	import FrameOrderStrip from './FrameOrderStrip.svelte';

	let { topicId, ontopic }: { topicId: string; ontopic: (t: Topic) => void } = $props();

	/** Presentation only. Never gates review -- a card exists because you
	 *  reproduced something, not because of which board it sits on. */
	let study = $state(false);

	const nodeTypes = {
		note: NoteNode as never,
		frame: FrameNode as never,
		topic: TopicNode as never,
		ink: InkNode as never,
		text: TextNode as never,
		image: AssetNode as never,
		pdf: AssetNode as never,
		link: LinkNode as never,
		question: QuestionNode as never
	};

	let frames = $state<Frame[]>([]);
	let canvasNodes = $state<CanvasNode[]>([]);
	/** Plain record rather than a Map: `$state` deep-proxies objects, not Maps. */
	let notes = $state<Record<string, Note>>({});
	let topicTitles = $state<Record<string, string>>({});

	let nodes = $state.raw<Node[]>([]);
	let edges = $state.raw<Edge[]>([]);
	/** The stored pointers. `edges` is the flow's projection of these. */
	let canvasEdges = $state<CanvasEdge[]>([]);
	/** The files placed on this board, keyed by id. */
	let assets = $state<Record<string, Asset>>({});
	/** The questions placed on this board, keyed by id. */
	let questions = $state<Record<string, Question>>({});
	/** Where the copies live. Read once; it never changes while running. */
	let assetRoot = $state('');

	let tool = $state<Tool>('select');
	let editingNoteId = $state<string | null>(null);
	/** The note a distil-drag came out of, so the extract knows its source. */
	let distilSource = $state<string | null>(null);
	/** True while a file is being dragged over the board from outside. */
	let dropping = $state(false);
	/**
	 * Where the text tool was clicked, before anything has been written.
	 *
	 * Held here rather than written to the database, so abandoning the thought
	 * leaves nothing behind. An empty text node is not a thought you had, it is
	 * a row you did not fill in — and it would sit on the board invisibly.
	 */
	let draftText = $state<{ x: number; y: number } | null>(null);
	let documentOpen = $state(false);
	let loaded = $state(false);

	const flow = useSvelteFlow();

	/** How many notes each frame has committed to the document. */
	const frameCounts = $derived(
		Object.fromEntries(
			frames.map((f) => [f.id, canvasNodes.filter((n) => n.frame_id === f.id).length])
		)
	);

	/** The strip is also a way back to a frame you have lost on a big canvas. */
	/** Non-destructive in both directions: no note fact is touched. */
	async function setMode(next: boolean) {
		const topic = await ipc.setTopicMode(topicId, next);
		study = topic.study;
		ontopic(topic);
		slot.enterBoard(topic, frames, focusFrame);
		rebuild();
	}

	function focusFrame(frameId: string) {
		const frame = frames.find((f) => f.id === frameId);
		if (!frame) return;
		flow.fitBounds(
			{ x: frame.x, y: frame.y, width: frame.width, height: frame.height },
			{ duration: 320, padding: 0.25 }
		);
	}

	const isEmpty = $derived(loaded && frames.length === 0 && canvasNodes.length === 0);

	// -- loading ----------------------------------------------------------

	async function load(id: string) {
		loaded = false;

		const [t, snapshot, noteList, all] = await Promise.all([
			ipc.openTopic(id),
			ipc.canvasSnapshot(id),
			ipc.topicNotes(id),
			ipc.allTopics()
		]);

		frames = snapshot.frames;
		canvasNodes = snapshot.nodes;
		canvasEdges = snapshot.edges;
		assets = Object.fromEntries(snapshot.assets.map((a) => [a.id, a]));
		questions = Object.fromEntries(snapshot.questions.map((q) => [q.id, q]));
		notes = Object.fromEntries(noteList.map((n) => [n.id, n]));
		topicTitles = Object.fromEntries(all.map((x) => [x.id, x.title]));

		study = t.study;
		ontopic(t);
		slot.enterBoard(t, snapshot.frames, focusFrame);
		session.noteVisit(t);
		loaded = true;
		rebuild();
	}

	// Keyed on topicId alone, so nothing the load itself writes can retrigger it.
	$effect(() => {
		load(topicId);
	});

	// Read once. It cannot change while the app is running.
	$effect(() => {
		ipc.assetDir().then((dir) => {
			assetRoot = dir;
			if (loaded) rebuild();
		});
	});

	/**
	 * Dropping a file from the desktop onto the board.
	 *
	 * This is Tauri's *native* drop, not the HTML one: the webview hands file
	 * drops to the host, which is the same reason note dragging inside the app
	 * had to move to pointer events. It gives us a real path, which is what the
	 * import needs — the browser's File object would only give us bytes and a
	 * name.
	 */
	$effect(() => {
		const stop = getCurrentWebview().onDragDropEvent(async (event) => {
			if (event.payload.type === 'over') {
				dropping = true;
				return;
			}
			if (event.payload.type !== 'drop') {
				dropping = false;
				return;
			}

			dropping = false;
			const { x, y } = flow.screenToFlowPosition({
				x: event.payload.position.x,
				y: event.payload.position.y
			});

			// Laid out in a row from the drop point, so dropping five images does
			// not stack five cards on one spot.
			let offset = 0;
			for (const path of event.payload.paths) {
				await bringIn(path, x + offset, y);
				offset += 40;
			}
		});

		return () => {
			stop.then((fn) => fn()).catch(() => {});
		};
	});

	/** One file in. Anything the webview cannot render is refused by Rust. */
	async function bringIn(path: string, x: number, y: number) {
		try {
			const [asset, node] = await ipc.importAsset(topicId, path, x, y);
			assets[asset.id] = asset;
			canvasNodes = [...canvasNodes, node];
			rebuild();
		} catch (err) {
			// A refusal here is a real answer -- "images and PDFs only" -- so it
			// is worth saying rather than swallowing.
			toast.error(String(err));
		}
	}

	/** The discoverable route in, for when dragging is not to hand. */
	async function chooseFile() {
		const picked = await pickFile({
			multiple: false,
			filters: [
				{ name: 'Images and PDFs', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'avif', 'pdf'] }
			]
		});
		if (typeof picked !== 'string') return;

		const at = flow.screenToFlowPosition({
			x: window.innerWidth / 2,
			y: window.innerHeight / 2
		});
		await bringIn(picked, at.x - 130, at.y - 95);
	}

	/**
	 * A page on the web. Typed, not fetched: nothing is downloaded.
	 *
	 * The click position is held until the address is typed, so the card lands
	 * where you asked for it rather than in the middle of the view.
	 */
	let linkAt = $state<{ x: number; y: number } | null>(null);

	function addLinkAt(clientX: number, clientY: number) {
		const { x, y } = flow.screenToFlowPosition({ x: clientX, y: clientY });
		linkAt = { x: x - 115, y: y - 29 };
	}

	async function commitLink(url: string) {
		const at = linkAt;
		linkAt = null;
		if (!at) return;

		try {
			const node = await ipc.addLinkNode(topicId, url, at.x, at.y);
			canvasNodes = [...canvasNodes, node];
			rebuild();
		} catch (err) {
			toast.error(String(err));
		}
	}

	// Cards carry `connecting` in their data, so switching tools has to reproject
	// them -- otherwise the ports never appear.
	$effect(() => {
		tool;
		if (loaded) rebuild();
	});

	// -- projection -------------------------------------------------------

	function rebuild() {
		const framedIds = new Set(canvasNodes.filter((n) => n.frame_id).map((n) => n.id));

		const frameNodes: Node[] = frames.map((frame) => ({
			id: `f:${frame.id}`,
			type: 'frame',
			position: { x: frame.x, y: frame.y },
			width: frame.width,
			height: frame.height,
			zIndex: 0,
			// Only the visible handle drags the frame. Without this, clicking
			// anywhere inside a frame would drag the frame instead of the note.
			dragHandle: '.frame-handle',
			data: {
				frame,
				count: canvasNodes.filter((n) => n.frame_id === frame.id).length,
				study,
				onlabel: relabelFrame,
				ondelete: removeFrame
			}
		}));

		const contentNodes: Node[] = canvasNodes.flatMap((node) => {
			const base = {
				id: `n:${node.id}`,
				position: { x: node.x, y: node.y },
				zIndex: 1
			};

			if (node.kind === 'note') {
				const note = notes[node.note_id!];
				if (!note) return [];

				return [
					{
						...base,
						type: 'note',
						width: node.width ?? 236,
						height: node.height ?? 118,
						data: {
							note,
							framed: framedIds.has(node.id),
							study,
							colour: node.color,
							connecting: tool === 'connect',
							onopen: (id: string) => (editingNoteId = id),
							oncontext: (event: MouseEvent) => cardMenu(event, node.id, note.id)
						}
					} as Node
				];
			}

			if (node.kind === 'topic') {
				return [
					{
						...base,
						type: 'topic',
						width: node.width ?? 216,
						height: node.height ?? 46,
						data: {
							title: topicTitles[node.target_topic_id!] ?? 'Topic',
							onopen: () => goto(`/t/${node.target_topic_id}`),
							oncontext: (event: MouseEvent) => nestedMenu(event, node.id)
						}
					} as Node
				];
			}

			if (node.kind === 'ink') {
				return [
					{
						...base,
						type: 'ink',
						data: {
							ink: JSON.parse(node.ink!),
							oncontext: (event: MouseEvent) => strokeMenu(event, node.id)
						}
					} as Node
				];
			}

			if (node.kind === 'image' || node.kind === 'pdf') {
				const asset = assets[node.asset_id!];
				if (!asset || !assetRoot) return [];

				return [
					{
						...base,
						type: node.kind,
						width: node.width ?? (node.kind === 'pdf' ? 380 : 260),
						height: node.height ?? (node.kind === 'pdf' ? 480 : 190),
						data: {
							asset,
							src: convertFileSrc(`${assetRoot}/${asset.rel_path}`),
							connecting: tool === 'connect',
							oncontext: (event: MouseEvent) => fileMenu(event, node.id, asset)
						}
					} as Node
				];
			}

			if (node.kind === 'question') {
				const question = questions[node.question_id!];
				if (!question) return [];

				return [
					{
						...base,
						type: 'question',
						width: node.width ?? 230,
						height: node.height ?? 96,
						data: {
							question,
							connecting: tool === 'connect',
							onanswer: answerOnBoard,
							oncontext: (event: MouseEvent) => questionMenu(event, node.id, question)
						}
					} as Node
				];
			}

			if (node.kind === 'link') {
				return [
					{
						...base,
						type: 'link',
						width: node.width ?? 230,
						height: node.height ?? 58,
						data: {
							url: node.url ?? '',
							connecting: tool === 'connect',
							oncontext: (event: MouseEvent) => linkMenu(event, node.id, node.url ?? '')
						}
					} as Node
				];
			}

			if (node.kind === 'text') {
				return [
					{
						...base,
						type: 'text',
						width: node.width ?? 220,
						height: node.height ?? 72,
						data: {
							text: node.text ?? '',
							connecting: tool === 'connect',
							autoedit: false,
							onchange: (text: string) => editText(node.id, text),
							onabandon: () => removeFromBoard(node.id),
							oncontext: (event: MouseEvent) => textMenu(event, node.id)
						}
					} as Node
				];
			}

			return [];
		});

		// The unwritten one, if the text tool is waiting for words. It is a flow
		// node like any other so it sits in the right place at the right zoom,
		// but nothing about it exists in the database yet.
		const draftNode: Node[] = draftText
			? [
					{
						id: 'draft',
						type: 'text',
						position: { x: draftText.x, y: draftText.y },
						width: 220,
						height: 72,
						zIndex: 2,
						data: {
							text: '',
							connecting: false,
							autoedit: true,
							onchange: commitDraft,
							onabandon: () => {
								draftText = null;
								rebuild();
							},
							oncontext: () => {}
						}
					} as Node
				]
			: [];

		nodes = [...frameNodes, ...contentNodes, ...draftNode];

		// Pointers are drawn between nodes, so they are projected here too.
		edges = canvasEdges.map((e) => ({
			id: `e:${e.id}`,
			source: `n:${e.source_id}`,
			target: `n:${e.target_id}`,
			label: e.label || undefined,
			animated: false
		}));
	}

	// -- loose text -------------------------------------------------------

	/**
	 * Text written straight onto the board.
	 *
	 * Deliberately not a note: no title, no state, no card. Thinking beside your
	 * cards should not first cost you a decision about whether the thought is a
	 * note — that decision is [`promoteText`], and it comes later or never.
	 */
	function createTextAt(clientX: number, clientY: number) {
		const { x, y } = flow.screenToFlowPosition({ x: clientX, y: clientY });
		draftText = { x: x - 110, y: y - 20 };
		tool = 'select';
		rebuild();
	}

	/** The first words are what bring the node into existence. */
	async function commitDraft(text: string) {
		const at = draftText;
		draftText = null;
		if (!at) return;

		const node = await ipc.addTextNode(topicId, text, at.x, at.y);
		canvasNodes = [...canvasNodes, node];
		rebuild();
	}

	async function editText(nodeId: string, text: string) {
		const node = canvasNodes.find((n) => n.id === nodeId);
		if (!node) return;

		node.text = text;
		canvasNodes = [...canvasNodes];
		rebuild();
		await ipc.setNodeText(nodeId, text);
	}

	/**
	 * Loose text becomes a note, in place.
	 *
	 * Same node, same position, same board: the arrangement you already built
	 * around the thought survives the decision that it was a note all along.
	 * The text becomes the body, not the title — titling it is a separate act,
	 * and it is the act that makes a note atomic.
	 */
	async function promoteText(nodeId: string) {
		const [note, node] = await ipc.promoteTextNode(nodeId);

		notes[note.id] = note;
		canvasNodes = canvasNodes.map((n) => (n.id === node.id ? node : n));
		rebuild();
		editingNoteId = note.id;
	}

	// -- pointers ---------------------------------------------------------

	/** Drawn between two cards on this board. Idempotent on the ordered pair. */
	async function onconnect({ source, target }: { source: string; target: string }) {
		if (!source.startsWith('n:') || !target.startsWith('n:')) return;

		const edge = await ipc.connectNodes(source.slice(2), target.slice(2));
		if (!canvasEdges.some((e) => e.id === edge.id)) canvasEdges = [...canvasEdges, edge];
		rebuild();
	}

	/** One click, no confirmation: an arrow is an arrangement, not a document. */
	async function dropPointer(edgeId: string) {
		canvasEdges = canvasEdges.filter((e) => e.id !== edgeId);
		rebuild();
		await ipc.disconnectNodes(edgeId);
	}

	function edgeMenu(event: MouseEvent, edgeId: string) {
		menu.show(event, [{ label: 'Remove pointer', onpick: () => dropPointer(edgeId) }]);
	}

	/** A stroke is only ever an arrangement, so it goes without asking. */
	function strokeMenu(event: MouseEvent, nodeId: string) {
		menu.show(event, [
			{ label: 'Delete stroke', destructive: true, onpick: () => removeFromBoard(nodeId) }
		]);
	}

	/** Takes the nested board off this canvas. The board itself is untouched. */
	function nestedMenu(event: MouseEvent, nodeId: string) {
		menu.show(event, [
			{ label: 'Remove from this board', onpick: () => removeFromBoard(nodeId) }
		]);
	}

	/**
	 * Right-clicking a file.
	 *
	 * Removing it takes the copy with it — an asset belongs to the board it was
	 * brought onto, unlike a note, which the library keeps. So there is no
	 * "remove from board" and "delete" pair to choose between: there is one act,
	 * and it says what it does.
	 */
	function fileMenu(event: MouseEvent, nodeId: string, asset: Asset) {
		menu.show(event, [
			{ label: 'Open in the system viewer', onpick: () => revealAsset(asset) },
			{
				label: 'Remove from this board',
				destructive: true,
				onpick: () => removeFromBoard(nodeId)
			}
		]);
	}

	async function revealAsset(asset: Asset) {
		try {
			await openPath(`${assetRoot}/${asset.rel_path}`);
		} catch {
			/* Nothing registered to open it is not worth an alert. */
		}
	}

	function linkMenu(event: MouseEvent, nodeId: string, url: string) {
		menu.show(event, [
			{ label: 'Open in browser', onpick: () => openUrl(url).catch(() => {}) },
			{ label: 'Remove from this board', destructive: true, onpick: () => removeFromBoard(nodeId) }
		]);
	}

	/**
	 * Answering a question from the board.
	 *
	 * Closing one requires the note that answers it — the one rule the app
	 * genuinely enforces — so this makes that note here, on this board, where
	 * the cards that prompted the answer already are.
	 */
	async function answerOnBoard(question: Question) {
		const [note, node] = await ipc.createNoteOnCanvas(
			topicId,
			question.text.replace(/\?+$/, '').trim(),
			0,
			0
		);
		await ipc.resolveQuestion(question.id, note.id);
		session.openQuestions = Math.max(0, session.openQuestions - 1);

		// The card was the open question; answered, it has nothing left to show.
		const card = canvasNodes.find((n) => n.question_id === question.id);
		if (card) {
			await ipc.deleteNode(card.id);
			canvasNodes = canvasNodes.filter((n) => n.id !== card.id);
			// Put the answer where the question was standing.
			node.x = card.x;
			node.y = card.y;
			await ipc.moveNode(node.id, card.x, card.y, null, null);
		}

		notes[note.id] = note;
		canvasNodes = [...canvasNodes, node];
		delete questions[question.id];
		rebuild();
		editingNoteId = note.id;
	}

	function questionMenu(event: MouseEvent, nodeId: string, question: Question) {
		menu.show(event, [
			{ label: 'Answer it with a note', onpick: () => answerOnBoard(question) },
			{ label: 'Take it off this board', onpick: () => removeFromBoard(nodeId) },
			{
				label: 'Drop the question',
				destructive: true,
				onpick: async () => {
					await ipc.setQuestionStatus(question.id, 'abandoned');
					session.openQuestions = Math.max(0, session.openQuestions - 1);
					await removeFromBoard(nodeId);
				}
			}
		]);
	}

	function textMenu(event: MouseEvent, nodeId: string) {
		menu.show(event, [
			{ label: 'Make it a note', onpick: () => promoteText(nodeId) },
			{ label: 'Delete', destructive: true, onpick: () => removeFromBoard(nodeId) }
		]);
	}

	// -- frames -----------------------------------------------------------

	function relabelFrame(id: string, label: string) {
		frames = frames.map((f) => (f.id === id ? { ...f, label } : f));
		ipc.labelFrame(id, label);
		rebuild();
	}

	async function removeFrame(id: string) {
		await ipc.deleteFrame(id);
		frames = frames.filter((f) => f.id !== id);
		// Nodes are released, not deleted: losing a narrative must not cost you
		// the thinking.
		canvasNodes = canvasNodes.map((n) => (n.frame_id === id ? { ...n, frame_id: null } : n));
		rebuild();
	}

	async function drawFrame(rect: { x: number; y: number; width: number; height: number }) {
		const frame = await ipc.createFrame(
			topicId,
			'',
			rect.x,
			rect.y,
			Math.max(rect.width, 280),
			Math.max(rect.height, 200)
		);

		frames = [...frames, frame];
		tool = 'select';
		rebuild();

		// Anything already sitting inside the new frame joins it immediately.
		await Promise.all(
			canvasNodes
				.filter((n) => !n.frame_id && inside(n, frame))
				.map(async (n) => {
					n.frame_id = frame.id;
					await ipc.setNodeFrame(n.id, frame.id, null);
				})
		);

		await resequence(frame.id);
		canvasNodes = [...canvasNodes];
		rebuild();
	}

	function inside(node: CanvasNode, frame: Frame): boolean {
		const cx = node.x + (node.width ?? 236) / 2;
		const cy = node.y + (node.height ?? 118) / 2;
		return cx >= frame.x && cx <= frame.x + frame.width && cy >= frame.y && cy <= frame.y + frame.height;
	}

	function frameAt(x: number, y: number, w: number, h: number): Frame | null {
		const cx = x + w / 2;
		const cy = y + h / 2;
		// Later frames win where they overlap, matching what is drawn on top.
		return (
			[...frames]
				.reverse()
				.find((f) => cx >= f.x && cx <= f.x + f.width && cy >= f.y && cy <= f.y + f.height) ?? null
		);
	}

	/**
	 * Within a frame, reading order is row-major. The result is written to
	 * `order_in_frame` explicitly, so the document always reads from stored
	 * order and never from live coordinates.
	 */
	async function resequence(frameId: string) {
		const members = canvasNodes
			.filter((n) => n.frame_id === frameId)
			.sort((a, b) => (Math.abs(a.y - b.y) > 40 ? a.y - b.y : a.x - b.x));

		await Promise.all(
			members.map((n, i) => {
				n.order_in_frame = i;
				return ipc.setNodeFrame(n.id, frameId, i);
			})
		);
	}

	// -- dragging ---------------------------------------------------------

	async function onnodedragstop({ targetNode }: { targetNode: Node | null }) {
		if (!targetNode) return;
		const { x, y } = targetNode.position;

		if (targetNode.id.startsWith('f:')) {
			const id = targetNode.id.slice(2);
			const frame = frames.find((f) => f.id === id);
			if (!frame) return;

			// Geometry only. Moving a frame never renumbers the document.
			frame.x = x;
			frame.y = y;
			await ipc.moveFrame(id, x, y, frame.width, frame.height);
			frames = [...frames];
			rebuild();
			return;
		}

		const id = targetNode.id.slice(2);
		const node = canvasNodes.find((n) => n.id === id);
		if (!node) return;

		const w = targetNode.measured?.width ?? 236;
		const h = targetNode.measured?.height ?? 118;

		node.x = x;
		node.y = y;
		await ipc.moveNode(id, x, y, w, h);

		const before = node.frame_id;
		const after = frameAt(x, y, w, h)?.id ?? null;

		if (before !== after) {
			node.frame_id = after;
			await ipc.setNodeFrame(id, after, null);
			if (before) await resequence(before);
		}

		if (after) await resequence(after);

		canvasNodes = [...canvasNodes];
		rebuild();
	}



	// The inspector borrows this screen's opener, so a backlink is followable
	// from the panel without the panel needing to know how notes get opened.
	$effect(() => {
		slot.openNote = (id: string) => (editingNoteId = id);
		return () => (slot.openNote = undefined);
	});

	// -- context menus ----------------------------------------------------

	/**
	 * Right-clicking a card.
	 *
	 * The two removals are deliberately different weights. Taking a note off
	 * this board is reversible and costs nothing — the note goes back to the
	 * library. Deleting it is irreversible and destroys work, so it is separated
	 * by a rule, coloured, and asks first.
	 */
	function cardMenu(event: MouseEvent, nodeId: string, noteId: string) {
		menu.show(event, [
			{ label: 'Open', onpick: () => (editingNoteId = noteId) },
			{
				swatches: [{ id: null, label: 'No colour' }, ...CARD_COLOURS.map((c) => ({ id: c.id, label: c.label }))],
				label: 'Colour',
				onswatch: (colour: string | null) => setColour(nodeId, colour)
			},
			{ label: 'Remove from this board', onpick: () => removeFromBoard(nodeId) },
			{
				label: 'Delete note everywhere',
				destructive: true,
				onpick: () => deleteNote(nodeId, noteId)
			}
		]);
	}

	function paneMenu(event: MouseEvent) {
		if ((event.target as HTMLElement).closest('.svelte-flow__node')) return;

		menu.show(event, [
			{ label: 'New note here', onpick: () => createNoteAt(event.clientX, event.clientY) },
			{ label: 'New text here', onpick: () => createTextAt(event.clientX, event.clientY) },
			{ label: 'New frame here', onpick: () => newFrameHere(event) },
			{ label: 'Add a link…', onpick: () => addLinkAt(event.clientX, event.clientY) },
			{ label: 'Add an image or PDF…', onpick: chooseFile }
		]);
	}

	async function newFrameHere(event: MouseEvent) {
		const { x, y } = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
		await drawFrame({ x, y, width: 480, height: 320 });
	}

	async function setColour(nodeId: string, colour: string | null) {
		const node = canvasNodes.find((n) => n.id === nodeId);
		if (!node) return;

		node.color = colour;
		canvasNodes = [...canvasNodes];
		rebuild();
		await ipc.setNodeColor(nodeId, colour);
	}

	/** Reversible: the note returns to the library untouched. */
	async function removeFromBoard(nodeId: string) {
		await ipc.deleteNode(nodeId);
		canvasNodes = canvasNodes.filter((n) => n.id !== nodeId);
		rebuild();
	}

	/** Irreversible, so it asks — the only confirmation in the app. */
	async function deleteNote(nodeId: string, noteId: string) {
		const note = notes[noteId];
		const name = note?.title?.trim() || 'this note';
		if (!confirm(`Delete ${name} everywhere? This cannot be undone.`)) return;

		await ipc.deleteNote(noteId);
		canvasNodes = canvasNodes.filter((n) => n.id !== nodeId);
		delete notes[noteId];
		rebuild();
	}

	// -- creating ---------------------------------------------------------

	/** Double-click empty canvas: make a note there and open it straight away. */
	async function createNoteAt(clientX: number, clientY: number) {
		const { x, y } = flow.screenToFlowPosition({ x: clientX, y: clientY });
		const [note, node] = await ipc.createNoteOnCanvas(topicId, '', x - 118, y - 59);

		notes[note.id] = note;
		canvasNodes = [...canvasNodes, node];
		rebuild();
		editingNoteId = note.id;
	}

	/**
	 * A note dropped in from the library. Placement is membership, so this is
	 * what makes an existing note part of this board -- and the only route from
	 * a capture onto a canvas.
	 */
	async function dropOnCanvas(payload: DragPayload, ctx: DropContext) {
		const { x, y } = flow.screenToFlowPosition({ x: ctx.clientX, y: ctx.clientY });

		if (payload.kind === 'note') {
			// Already here: do not create a second node for the same note.
			if (canvasNodes.some((n) => n.note_id === payload.noteId)) return;

			const node = await ipc.addNoteNode(topicId, payload.noteId, x - 118, y - 59);
			const placed = await ipc.getNote(payload.noteId);
			if (placed) notes[placed.id] = placed;
			canvasNodes = [...canvasNodes, node];
			rebuild();
			return;
		}

		if (payload.kind === 'question') {
			// Already here: one card per question, not one per drop.
			if (canvasNodes.some((n) => n.question_id === payload.questionId)) return;

			// Placement is membership: this is also what attaches a stray doubt
			// to the board, so the board's panel can find it afterwards.
			const node = await ipc.addQuestionNode(topicId, payload.questionId, x - 115, y - 48);
			const placed = await ipc.questionQueue(topicId);
			questions = Object.fromEntries(placed.map((q) => [q.id, q]));

			canvasNodes = [...canvasNodes, node];
			rebuild();
			return;
		}

		if (payload.kind === 'blocks' && distilSource) {
			// The fast lane. Written with the source visible, so it is a draft --
			// memory mode stays the only route to recalled.
			const [note, node] = await ipc.distilToCanvas(
				topicId,
				distilSource,
				payload.json,
				payload.html,
				payload.text,
				x - 118,
				y - 59
			);

			notes[note.id] = note;
			canvasNodes = [...canvasNodes, node];
			rebuild();
			// Untitled on purpose: naming it is what makes it atomic.
			editingNoteId = note.id;
		}
	}

	$effect(() => drag.register('canvas', dropOnCanvas));

	function onpanedblclick(event: MouseEvent) {
		if (tool !== 'select') return;

		const target = event.target as HTMLElement;
		if (!target.closest('.svelte-flow__pane')) return;
		if (target.closest('.svelte-flow__node')) return;

		createNoteAt(event.clientX, event.clientY);
	}

	/**
	 * The text tool writes on a single click: loose text should cost one gesture.
	 *
	 * Tested against the flow *pane* rather than against "not a node", because
	 * the toolbar and the overlays are children of this same element — so a
	 * click on the Text button itself bubbled here and dropped a node behind the
	 * button that had just been pressed.
	 */
	function onpaneclick(event: MouseEvent) {
		if (tool !== 'text') return;

		const target = event.target as HTMLElement;
		if (!target.closest('.svelte-flow__pane')) return;
		if (target.closest('.svelte-flow__node')) return;

		createTextAt(event.clientX, event.clientY);
	}

	async function commitInk(points: [number, number, number][]) {
		const local = normalise(points);
		const node = await ipc.addInkNode(
			topicId,
			JSON.stringify({ ...DEFAULT_INK, points: local.points }),
			local.x,
			local.y
		);

		canvasNodes = [...canvasNodes, node];
		rebuild();
	}

	function onSaved(note: Note) {
		notes[note.id] = note;
		rebuild();
	}

	async function onFramesReordered(ids: string[]) {
		await ipc.reorderFrames(topicId, ids);
		frames = ids
			.map((id, i) => {
				const f = frames.find((x) => x.id === id)!;
				return { ...f, order_index: i };
			})
			.filter(Boolean);
		rebuild();
	}

	// -- keyboard ---------------------------------------------------------

	function onkeydown(event: KeyboardEvent) {
		const typing = (event.target as HTMLElement)?.closest(
			'input, textarea, [contenteditable="true"]'
		);
		if (typing || event.ctrlKey || event.metaKey || event.altKey) return;

		if (event.key === 'v') tool = 'select';
		if (event.key === 'p') tool = 'draw';
		if (event.key === 'f') tool = 'frame';
		if (event.key === 't') tool = 'text';
		if (event.key === 'c') tool = 'connect';
		if (event.key === 'Escape') tool = 'select';
	}

	onMount(() => {
		window.addEventListener('keydown', onkeydown);
		return () => window.removeEventListener('keydown', onkeydown);
	});
</script>

<div
	class="relative h-full w-full transition-shadow"
	role="presentation"
	data-drop-zone="canvas"
	onclick={onpaneclick}
	ondblclick={onpanedblclick}
	oncontextmenu={paneMenu}
	style={drag.over === 'canvas' || dropping
		? 'box-shadow: inset 0 0 0 2px var(--state-recalled)'
		: undefined}
>
	<SvelteFlow
		bind:nodes
		bind:edges
		{nodeTypes}
		{onnodedragstop}
		minZoom={0.15}
		maxZoom={2.5}
		nodesDraggable={tool === 'select'}
		nodesConnectable={tool === 'connect'}
		{onconnect}
		onedgecontextmenu={({ event, edge }) => edgeMenu(event as MouseEvent, edge.id.slice(2))}
		panOnDrag={tool === 'select' || tool === 'connect'}
		selectionOnDrag={false}
		zoomOnDoubleClick={false}
		proOptions={{ hideAttribution: true }}
	>
		<Background
			variant={BackgroundVariant.Dots}
			gap={26}
			size={1}
			bgColor="var(--canvas)"
			patternColor="var(--canvas-dot)"
		/>
		<Controls position="bottom-right" showLock={false} />
	</SvelteFlow>

	{#if linkAt}
		<LinkPrompt onsubmit={commitLink} oncancel={() => (linkAt = null)} />
	{/if}

	{#if dropping}
		<div
			class="pointer-events-none absolute inset-0 z-30 flex items-center justify-center"
			style="background: color-mix(in oklch, var(--background) 55%, transparent)"
		>
			<p
				class="bg-card rounded-md border px-4 py-2.5 text-[12.5px] shadow-sm"
				style="border-color: var(--hairline-strong)"
			>
				Drop to bring it onto this board
			</p>
		</div>
	{/if}

	{#if tool === 'draw'}
		<InkOverlay oncommit={commitInk} onescape={() => (tool = 'select')} />
	{/if}

	{#if tool === 'frame'}
		<RectOverlay oncommit={drawFrame} onescape={() => (tool = 'select')} />
	{/if}

	{#if isEmpty}
		<CanvasEmptyState />
	{/if}

	{#if study}
		<FrameOrderStrip
			{frames}
			counts={frameCounts}
			onreorder={onFramesReordered}
			onfocus={focusFrame}
		/>
	{/if}

	<CanvasToolbar
		bind:tool
		{study}
		onmode={setMode}
		ondocument={() => (documentOpen = true)}
		onrecall={() => goto(`/t/${topicId}/recall`)}
	/>
</div>

{#if editingNoteId}
	{#key editingNoteId}
		<NoteEditor
		noteId={editingNoteId}
		{topicId}
		ondistil={(id) => (distilSource = id)}
		onclose={() => (editingNoteId = null)}
		onsaved={onSaved}
	/>
	{/key}
{/if}

<DocumentPanel bind:open={documentOpen} {topicId} />

