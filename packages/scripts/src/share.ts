const COPY_RESET_MS = 2000;

function bindContainer(container: HTMLElement): void {
    const copyButton = container.querySelector<HTMLButtonElement>(
        "[data-share-copy]"
    );

    if (!copyButton || copyButton.dataset.bound === "true") {
        return;
    }

    copyButton.dataset.bound = "true";

    const url =
        container.getAttribute("data-share-url") || window.location.href;
    const label = copyButton.querySelector<HTMLElement>(
        ".post-share__copy-label"
    );

    copyButton.addEventListener("click", async () => {
        try {
            await navigator.clipboard.writeText(url);

            if (label) {
                const original = label.textContent;
                label.textContent = "복사됨!";
                setTimeout(() => {
                    label.textContent = original;
                }, COPY_RESET_MS);
            }
        } catch {
            /* clipboard unavailable — leave the link visible to copy manually */
        }
    });
}

function setup(root: ParentNode = document): void {
    root
        .querySelectorAll<HTMLElement>("[data-share]")
        .forEach((container) => bindContainer(container));
}

export function initShare(): void {
    setup();
    document.addEventListener("spa:navigate", (event) => {
        const detail = (event as CustomEvent<{ container?: ParentNode }>).detail;
        setup(detail?.container ?? document);
    });
}
