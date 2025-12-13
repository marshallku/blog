interface CommentFormData {
    postSlug: string;
    apiUrl: string;
    name: string;
    url: string;
    body: string;
    parentId: string;
    replyingTo: string;
    loading: boolean;
    init(): Promise<void>;
    loadComments(): Promise<void>;
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
        body: "",
        parentId: "",
        replyingTo: "",
        loading: false,

        async init() {
            await this.loadComments();
            document
                .getElementById("comment-list")
                ?.addEventListener("click", (e) => {
                    const target = e.target as HTMLElement;
                    if (target.matches(".comment-bubble__reply-btn")) {
                        this.setReply(
                            target.dataset.id || "",
                            target.dataset.name || ""
                        );
                    }
                });
        },

        async loadComments() {
            const list = document.getElementById("comment-list");
            if (!list) return;

            try {
                const res = await fetch(
                    `${this.apiUrl}/api/v2/comment/list?postSlug=${encodeURIComponent(this.postSlug)}`
                );
                if (!res.ok) throw new Error();
                list.innerHTML = await res.text();
            } catch {
                list.innerHTML =
                    '<p class="comment-list__error">댓글을 불러오지 못했습니다.</p>';
            }
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
                const res = await fetch(`${this.apiUrl}/api/v2/comment/create`, {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                        postSlug: this.postSlug,
                        name: this.name || "익명",
                        url: this.url || undefined,
                        body: this.body,
                        parentCommentId: this.parentId || undefined,
                    }),
                });

                if (!res.ok) throw new Error();

                const html = await res.text();

                if (this.parentId) {
                    const parent = document.getElementById(
                        "comment-" + this.parentId
                    );
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
                        list.innerHTML =
                            '<ul class="comment-list">' + html + "</ul>";
                    }
                }

                this.body = "";
                this.cancelReply();
            } catch {
                alert("댓글 등록에 실패했습니다.");
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
