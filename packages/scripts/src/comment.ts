import { t } from "./i18n";

/** The API rejects a token younger than this (`MIN_FORM_AGE_SECONDS` plus clock slack). */
const MIN_TOKEN_AGE_MS = 3500;

interface CommentFormData {
    postSlug: string;
    apiUrl: string;
    name: string;
    url: string;
    homepage: string;
    formToken: string;
    body: string;
    parentId: string;
    replyingTo: string;
    loading: boolean;
    init(): Promise<void>;
    loadComments(): Promise<void>;
    loadFormToken(): Promise<void>;
    refreshFormToken(): Promise<void>;
    postComment(): Promise<Response>;
    setReply(id: string, name: string): void;
    cancelReply(): void;
    submit(): Promise<void>;
}

export function commentForm(postSlug: string, apiUrl: string): CommentFormData {
    return {
        postSlug,
        apiUrl,
        name: localStorage.getItem("comment-name") || "",
        url: localStorage.getItem("comment-url") || "",
        homepage: "",
        formToken: "",
        body: "",
        parentId: "",
        replyingTo: "",
        loading: false,

        async init() {
            await Promise.all([this.loadComments(), this.loadFormToken()]);
            document.getElementById("comment-list")?.addEventListener("click", (e) => {
                const target = e.target as HTMLElement;
                if (target.matches(".comment-bubble__reply-btn")) {
                    this.setReply(target.dataset.id || "", target.dataset.name || "");
                }
            });
        },

        async loadComments() {
            const list = document.getElementById("comment-list");
            if (!list) return;

            try {
                const res = await fetch(
                    `${this.apiUrl}/api/v2/comment/list?postSlug=${encodeURIComponent(this.postSlug)}&lang=${encodeURIComponent(document.documentElement.lang)}`,
                );
                if (!res.ok) throw new Error();
                list.innerHTML = await res.text();
            } catch {
                list.innerHTML = '<p class="comment-list__error">' + t("commentsLoadError") + "</p>";
            }
        },

        async loadFormToken() {
            // Proves the comment came from a loaded form; without it the API rejects the post.
            try {
                const res = await fetch(`${this.apiUrl}/api/v2/comment/token`);
                if (!res.ok) return;
                const { token } = (await res.json()) as { token: string };
                this.formToken = token;
            } catch {
                // Leave it empty; submit surfaces the failure.
            }
        },

        async refreshFormToken() {
            // Recovers a form whose token never loaded or went stale, without losing the draft.
            await this.loadFormToken();
            await new Promise((resolve) => setTimeout(resolve, MIN_TOKEN_AGE_MS));
        },

        postComment() {
            return fetch(`${this.apiUrl}/api/v2/comment/create`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    postSlug: this.postSlug,
                    name: this.name || t("anonymous"),
                    url: this.url || undefined,
                    body: this.body,
                    parentCommentId: this.parentId || undefined,
                    lang: document.documentElement.lang,
                    homepage: this.homepage || undefined,
                    formToken: this.formToken || undefined,
                }),
            });
        },

        setReply(id: string, name: string) {
            this.parentId = id;
            this.replyingTo = name;
            document.getElementById("comment-body")?.focus();
        },

        cancelReply() {
            this.parentId = "";
            this.replyingTo = "";
        },

        async submit() {
            this.loading = true;
            localStorage.setItem("comment-name", this.name);
            if (this.url) localStorage.setItem("comment-url", this.url);

            try {
                if (!this.formToken) await this.refreshFormToken();

                let res = await this.postComment();
                if (res.status === 400) {
                    await this.refreshFormToken();
                    res = await this.postComment();
                }

                if (!res.ok) throw new Error();

                const html = await res.text();

                if (this.parentId) {
                    const parent = document.getElementById("comment-" + this.parentId);
                    if (!parent) return;

                    let replies = parent.nextElementSibling;
                    if (!replies || replies.tagName !== "UL") {
                        replies = document.createElement("ul");
                        parent.after(replies);
                    }
                    replies.insertAdjacentHTML("beforeend", html);
                } else {
                    const list = document.getElementById("comment-list");
                    if (!list) return;

                    const ul = list.querySelector("ul.comment-list");
                    if (ul) {
                        const empty = ul.querySelector(".comment-list__empty");
                        if (empty) empty.remove();
                        ul.insertAdjacentHTML("afterbegin", html);
                    } else {
                        list.innerHTML = '<ul class="comment-list">' + html + "</ul>";
                    }
                }

                this.body = "";
                this.cancelReply();
            } catch {
                alert(t("commentPostError"));
            } finally {
                this.loading = false;
            }
        },
    };
}

declare global {
    interface Window {
        commentForm: typeof commentForm;
    }
}

window.commentForm = commentForm;
