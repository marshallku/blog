const PARTIAL_DIR = "/html";
const MAIN_SELECTOR = "#main";
const PARTIAL_CONTENT_ID = "partial-content";
const SPA_STYLE_ID = "spa-page-styles";

declare global {
    interface Window {
        Alpine: typeof Alpine;
    }
    const Alpine: {
        initTree: (el: HTMLElement) => void;
    };
}

const SHARED_COVER_NAME = "post-cover";
const COVER_IMG_SELECTOR = ".post__cover img";

let sharedEls: HTMLElement[] = [];

function prefersReducedMotion(): boolean {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function markShared(el: HTMLElement | null): void {
    if (!el) {
        return;
    }
    el.style.setProperty("view-transition-name", SHARED_COVER_NAME);
    sharedEls.push(el);
}

function clearShared(): void {
    sharedEls.forEach((el) => el.style.removeProperty("view-transition-name"));
    sharedEls = [];
}

interface NavigationState {
    isLoading: boolean;
}

const state: NavigationState = {
    isLoading: false,
};

function isInternalLink(link: HTMLAnchorElement): boolean {
    if (!link.href) {
        return false;
    }

    const url = new URL(link.href, window.location.origin);

    if (url.origin !== window.location.origin) {
        return false;
    }

    if (link.target === "_blank") {
        return false;
    }

    if (link.hasAttribute("download")) {
        return false;
    }

    const ext = url.pathname.split(".").pop()?.toLowerCase();
    if (ext && ["pdf", "zip", "png", "jpg", "jpeg", "gif", "webp"].includes(ext)) {
        return false;
    }

    return true;
}

function normalizePath(path: string): string {
    if (path.endsWith("/")) {
        return `${path}index.html`;
    }

    if (path.endsWith(".html")) {
        return path;
    }

    return `${path}/index.html`;
}

function toPartialUrl(url: string): string {
    const urlObj = new URL(url, window.location.origin);
    const path = urlObj.pathname;

    if (path.startsWith(PARTIAL_DIR)) {
        return url;
    }

    const partialPath = `${PARTIAL_DIR}${path}`;
    const normalizedPath = normalizePath(partialPath);

    return normalizedPath;
}

async function fetchPartial(url: string): Promise<string> {
    const partialUrl = toPartialUrl(url);

    const response = await fetch(partialUrl);

    if (!response.ok) {
        throw new Error(`Failed to fetch partial: ${response.status}`);
    }

    const html = await response.text();

    return html;
}

function updatePageMeta(): void {
    const partialContent = document.getElementById(PARTIAL_CONTENT_ID);
    if (!partialContent) {
        return;
    }

    const title = partialContent.dataset.pageTitle;
    if (title) {
        document.title = title;
    }

    const styles = partialContent.dataset.pageStyles;
    if (styles) {
        updatePageStyles(styles);
    }
}

function updatePageStyles(newStylesUrl: string | null): void {
    const existingLink = document.getElementById(SPA_STYLE_ID) as HTMLLinkElement | null;
    const currentHref = existingLink?.href ?? "";

    if (!newStylesUrl) {
        existingLink?.remove();
        return;
    }

    const absoluteNewUrl = new URL(newStylesUrl, window.location.origin).href;

    if (currentHref === absoluteNewUrl) {
        return;
    }

    if (existingLink) {
        existingLink.href = newStylesUrl;
        return;
    }

    const link = document.createElement("link");

    link.id = SPA_STYLE_ID;
    link.rel = "stylesheet";
    link.href = newStylesUrl;

    document.head.appendChild(link);
}

function reInitAlpine(container: Element): void {
    if (typeof Alpine === "undefined") {
        return;
    }

    const alpineElements = container.querySelectorAll("[x-data], [x-init]");
    alpineElements.forEach((el) => {
        if (el instanceof HTMLElement) {
            Alpine.initTree(el);
        }
    });
}

function reInitCodeCopy(container: Element): void {
    const codeBlocks = container.querySelectorAll("pre > code");
    codeBlocks.forEach((block) => {
        const pre = block.parentElement;
        if (!pre || pre.querySelector(".copy-code")) {
            return;
        }

        const button = document.createElement("button");
        button.className = "copy-code";
        button.textContent = "Copy";
        button.type = "button";

        button.addEventListener("click", async () => {
            const code = block.textContent || "";
            try {
                await navigator.clipboard.writeText(code);
                button.textContent = "Copied!";
                setTimeout(() => {
                    button.textContent = "Copy";
                }, 2000);
            } catch {
                button.textContent = "Failed";
            }
        });

        pre.style.position = "relative";
        pre.appendChild(button);
    });
}

async function navigateTo(url: string, pushState = true): Promise<void> {
    if (state.isLoading) {
        clearShared();
        return;
    }

    state.isLoading = true;

    const main = document.querySelector(MAIN_SELECTOR);
    if (!main) {
        state.isLoading = false;
        window.location.href = url;
        return;
    }

    main.classList.add("spa-loading");

    try {
        const html = await fetchPartial(url);

        // Destination has no cover → drop the card's shared name so it's a plain
        // root cross-fade instead of an orphaned morph with no matching target.
        // Parse with the exact selector (not a substring) so unrelated markup like
        // `prev-next-post-post__cover` doesn't count as a cover.
        if (sharedEls.length > 0) {
            const hasCover = new DOMParser().parseFromString(html, "text/html").querySelector(COVER_IMG_SELECTOR);
            if (!hasCover) {
                clearShared();
            }
        }

        const container = main.querySelector<HTMLElement>(".container");

        const applyUpdate = (): void => {
            if (container) {
                container.innerHTML = html;

                // Pair the incoming cover with the clicked card thumbnail so the
                // View Transition morphs one into the other. Only when a card was
                // marked on click (forward into a post) — otherwise plain cross-fade.
                if (sharedEls.length > 0) {
                    markShared(container.querySelector<HTMLElement>(COVER_IMG_SELECTOR));
                }

                updatePageMeta();
                reInitAlpine(container);
                reInitCodeCopy(container);
            }

            if (pushState) {
                history.pushState({ url }, "", url);
            }

            window.scrollTo({ top: 0, behavior: "instant" });

            if (container) {
                document.dispatchEvent(new CustomEvent("spa:navigate", { detail: { container } }));
            }
        };

        if (document.startViewTransition && !prefersReducedMotion()) {
            const transition = document.startViewTransition(applyUpdate);
            // Hold isLoading (reset in `finally`) until the animation settles, so a
            // rapid second click can't start a new navigation while shared names are
            // still live. `updateCallbackDone` rejects only if the DOM swap threw
            // (real error → outer catch); a skipped visual transition is not an error.
            await transition.updateCallbackDone;
            await transition.finished.catch(() => {});
            clearShared();
        } else {
            applyUpdate();
            clearShared();
        }
    } catch (error) {
        console.error("SPA navigation failed:", error);
        clearShared();
        window.location.href = url;
    } finally {
        main.classList.remove("spa-loading");
        state.isLoading = false;
    }
}

function handleLinkClick(event: MouseEvent): void {
    if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) {
        return;
    }

    if (event.button !== 0) {
        return;
    }

    const link = (event.target as Element).closest("a");
    if (!link || !(link instanceof HTMLAnchorElement)) {
        return;
    }

    if (!isInternalLink(link)) {
        return;
    }

    // Only morph the cover for the card's post-detail links (image/title), not the
    // tag links inside the card, which navigate to tag pages that have no cover.
    const card = link.closest<HTMLElement>(".post-card");
    if (card && !link.closest(".post-card__tags")) {
        markShared(card.querySelector<HTMLElement>("img"));
    }

    event.preventDefault();
    navigateTo(link.href);
}

function handlePopState(event: PopStateEvent): void {
    const url = event.state?.url || window.location.href;
    navigateTo(url, false);
}

export function initSpa(): void {
    document.addEventListener("click", handleLinkClick);
    window.addEventListener("popstate", handlePopState);

    history.replaceState({ url: window.location.href }, "", window.location.href);
}
