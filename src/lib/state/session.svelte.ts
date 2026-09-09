/**
 * Shared session state.
 *
 * Deliberately small: the sidebar list and the capture surface. Everything else
 * is loaded by the route that needs it, so no screen waits on another screen.
 */

import * as ipc from '$lib/ipc';
import type { Note, Topic } from '$lib/types';

class Session {
	/** Recently visited topics. The rest fall asleep; sleeping is not failing. */
	topics = $state<Topic[]>([]);
	/** Open questions across everything. This is work, not debt, so it can show. */
	openQuestions = $state(0);
	/** Today's queue size, never the backlog behind it. */
	reviewDue = $state(0);
	inbox = $state<Note[]>([]);
	captureOpen = $state(false);
	/** Shown in the custom titlebar. This is what replaces a folder tree. */
	crumbs = $state<{ label: string; href?: string }[]>([]);

	async refresh() {
		const [topics, openQuestions, inbox, reviewDue] = await Promise.all([
			ipc.awakeTopics(),
			ipc.openQuestionCount(),
			ipc.inboxNotes(),
			ipc.reviewDueToday()
		]);

		this.topics = topics;
		this.openQuestions = openQuestions;
		this.inbox = inbox;
		this.reviewDue = reviewDue;
	}

	/**
	 * Capture. The only path in the app that must stay instant: it writes a bare
	 * title and returns, with no topic, no placement and no decisions.
	 */
	async capture(title: string): Promise<Note> {
		const note = await ipc.createNote(title.trim());
		this.inbox = [note, ...this.inbox];
		return note;
	}

	async createTopic(title: string): Promise<Topic> {
		const topic = await ipc.createTopic(title.trim());
		this.topics = [topic, ...this.topics];
		return topic;
	}

	/** Cheap refresh after grading a card, without reloading the whole session. */
	async refreshReviewCount() {
		this.reviewDue = await ipc.reviewDueToday();
	}

	/** Keeps the sidebar honest after a visit without a full reload. */
	noteVisit(topic: Topic) {
		this.topics = [topic, ...this.topics.filter((t) => t.id !== topic.id)];
	}
}

export const session = new Session();
