const PARTIAL_DIR = "/html";
const MAIN_SELECTOR = "#main";
const PARTIAL_CONTENT_SELECTOR = ".partial-content";
const SPA_STYLE_ID = "spa-page-styles";

declare global {
    interface Window {
        Alpine: typeof Alpine;
    }
    const Alpine: {
        initTree: (el: HTMLElement) => void;
    };
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
    if (
        ext &&
        ["pdf", "zip", "png", "jpg", "jpeg", "gif", "webp"].includes(ext)
    ) {
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

function updatePageMeta(container: Element): void {
    const partialContent = container.querySelector(PARTIAL_CONTENT_SELECTOR);

    if (!partialContent) {
        return;
    }

    const title = partialContent.getAttribute("data-page-title");
    if (title) {
        document.title = title;
    }

    const styles = partialContent.getAttribute("data-page-styles");
    updatePageStyles(styles);
}

function updatePageStyles(newStylesUrl: string | null): void {
    const existingLink = document.getElementById(
        SPA_STYLE_ID
    ) as HTMLLinkElement | null;
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

        const container = main.querySelector(".container");
        if (container) {
            container.innerHTML = html;

            updatePageMeta(container);
            reInitAlpine(container);
            reInitCodeCopy(container);
        }

        if (pushState) {
            history.pushState({ url }, "", url);
        }

        window.scrollTo({ top: 0, behavior: "instant" });
    } catch (error) {
        console.error("SPA navigation failed:", error);
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

    history.replaceState(
        { url: window.location.href },
        "",
        window.location.href
    );
}
