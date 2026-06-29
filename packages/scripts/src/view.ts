const counted = new Set<string>();

function ping(): void {
    const article = document.querySelector<HTMLElement>("article.post[data-view-slug]");

    if (!article) {
        return;
    }

    const slug = article.dataset.viewSlug;
    const api = article.dataset.viewApi;

    if (!slug || !api || counted.has(slug)) {
        return;
    }

    counted.add(slug);

    void fetch(`${api}/api/v2/view/hit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ postSlug: slug }),
    }).catch(() => {
        /* view counting is best-effort — never surface to the reader */
    });
}

export function initViewCounter(): void {
    ping();
    document.addEventListener("spa:navigate", ping);
}
