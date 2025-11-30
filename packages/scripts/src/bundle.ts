import { fit, throttle } from "./utils";

document.addEventListener("DOMContentLoaded", () => {
    initGlobalNavigation();
    initScrollToTop();
    initCodeCopy();
});

function initGlobalNavigation(): void {
    const navigation =
        document.querySelector<HTMLDivElement>(".global-navigation");
    if (!navigation) {
        return;
    }

    const handleScroll = fit(() => {
        if (window.scrollY >= 10) {
            navigation.classList.add("global-navigation--scrolled");
        } else {
            navigation.classList.remove("global-navigation--scrolled");
        }
    });

    window.addEventListener("scroll", handleScroll, { passive: true });
}

function initScrollToTop(): void {
    const button = document.querySelector<HTMLButtonElement>(".scroll-to-top");
    if (!button) {
        return;
    }

    const toggleVisibility = throttle(() => {
        button.classList.toggle("visible", window.scrollY > 300);
    }, 100);

    window.addEventListener("scroll", toggleVisibility, { passive: true });
    button.addEventListener("click", () => {
        window.scrollTo({ top: 0, behavior: "smooth" });
    });
}

function initCodeCopy(): void {
    const codeBlocks = document.querySelectorAll("pre > code");
    codeBlocks.forEach((block) => {
        const pre = block.parentElement;
        if (!pre) {
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
