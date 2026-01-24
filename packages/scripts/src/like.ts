interface LikeButtonData {
    postSlug: string;
    apiUrl: string;
    loading: boolean;
    init(): Promise<void>;
    loadButton(): Promise<void>;
    toggle(): Promise<void>;
}

export function likeButton(postSlug: string, apiUrl: string): LikeButtonData {
    return {
        postSlug,
        apiUrl,
        loading: true,

        async init() {
            await this.loadButton();
        },

        async loadButton() {
            const container = document.getElementById("like-button");
            if (!container) return;

            try {
                const res = await fetch(
                    `${this.apiUrl}/api/v2/like/status?postSlug=${encodeURIComponent(this.postSlug)}`,
                    { credentials: "include" }
                );
                if (!res.ok) throw new Error();
                container.innerHTML = await res.text();
            } catch {
                container.innerHTML =
                    '<button class="post-like__button" disabled>오류</button>';
            } finally {
                this.loading = false;
            }
        },

        async toggle() {
            if (this.loading) return;

            const container = document.getElementById("like-button");
            if (!container) return;

            this.loading = true;

            try {
                const res = await fetch(`${this.apiUrl}/api/v2/like/toggle`, {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    credentials: "include",
                    body: JSON.stringify({ postSlug: this.postSlug }),
                });

                if (!res.ok) throw new Error();
                container.innerHTML = await res.text();
            } catch {
                // Keep current state on error
            } finally {
                this.loading = false;
            }
        },
    };
}

declare global {
    interface Window {
        likeButton: typeof likeButton;
    }
}

window.likeButton = likeButton;
