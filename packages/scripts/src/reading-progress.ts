import { fit } from "./utils";

const ARTICLE_SELECTOR = "article.post";

export function initReadingProgress(): void {
    const container = document.querySelector<HTMLElement>(
        "[data-reading-progress]"
    );
    const bar = document.querySelector<HTMLElement>(
        "[data-reading-progress-bar]"
    );

    if (!container || !bar) {
        return;
    }

    let article: HTMLElement | null = null;

    const update = (): void => {
        if (!article) {
            bar.style.transform = "scaleX(0)";
            return;
        }

        const scrolled = window.scrollY - article.offsetTop;
        const total = article.offsetHeight - window.innerHeight;
        const ratio = total > 0 ? Math.min(Math.max(scrolled / total, 0), 1) : 0;

        bar.style.transform = `scaleX(${ratio})`;
    };

    const refresh = (): void => {
        article = document.querySelector<HTMLElement>(ARTICLE_SELECTOR);
        container.classList.toggle("reading-progress--active", Boolean(article));
        update();
    };

    const onScroll = fit(update);

    refresh();
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll, { passive: true });
    document.addEventListener("spa:navigate", refresh);
}
